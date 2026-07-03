# impulse-client-ring

A [`reqwest`](https://docs.rs/reqwest)-style HTTP client that talks to a server
over the **Ring** shared-memory IPC bus instead of TCP/Unix sockets. Besides
unary request/response it also speaks **SSE, WebSocket and WebTransport** over
Ring channels (see [Streaming & upgrades](#streaming--upgrades)).

It is the client half of the Ring HTTP transport. The server half is
`ImpulseRingListener` in
[`impulse-server-kit`](../impulse-server-kit) (feature `impulse-ring`, on by
default). A server registers an application on the bus and serves HTTP over
shared memory; this client looks the application up **by name** and issues
ordinary HTTP requests against it — no ports, no kernel round-trips on the data
path.

```
┌────────────────────────┐        shared memory         ┌─────────────────────────┐
│  ImpulseRingClient      │  ── RingHttpRequest  ──▶     │  ImpulseRingListener     │
│  (impulse-client-ring)  │                              │  (impulse-server-kit)    │
│  get / post / …         │     ◀── RingHttpResponse ──  │  full salvo pipeline     │
└────────────────────────┘                              └─────────────────────────┘
                 ▲                                              ▲
                 └──────────────  impulsed broker  ─────────────┘
```

Every request is a single Avro-framed RPC carrying a `RingHttpRequest`; the
response is a `RingHttpResponse`. The wire schemas live in the shared
[`impulse-ring-http`](https://docs.rs/impulse-ring-http) crate, so the broker's
fingerprint check guarantees both ends agree on the shape.

Bodies of any size just work: a request body too large to fit the function's
request ring is transparently streamed over a dedicated Ring channel (the
listener reassembles it before salvo sees the request), and an oversized
response body is chunked back over a channel the same way — `send()` /
`send_blocking()` always return a complete inline body.

## Prerequisites

1. The **Ring broker** `impulsed` must be running (it owns the shared-memory
   control segment). From the `impulse-ring` repository:

   ```sh
   cargo run -p impulsed
   ```

2. A **server** must have registered the application name you want to call. See
   the `ring-server` example or `impulse-server-kit`'s `impulse-ring` feature.

## Quick start

```rust
use impulse_client_ring::ImpulseRingClient;

fn main() -> std::io::Result<()> {
  // `app_name` is the name the server registered on the bus.
  let client = ImpulseRingClient::connect("hello-ring")?;

  // Blocking call.
  let resp = client.get("/hello").send_blocking()?;
  println!("{} {}", resp.status(), resp.text()?);
  Ok(())
}
```

Async (with the default `async` feature; the bus call runs on a Tokio blocking
thread):

```rust,no_run
# use impulse_client_ring::ImpulseRingClient;
# #[derive(serde::Serialize)] struct NewItem { name: String }
# async fn run() -> std::io::Result<()> {
let client = ImpulseRingClient::connect("my-service")?.with_key("s3cret");

let resp = client
  .post("/items")
  .header("x-request-id", "abc123")
  .json(&NewItem { name: "widget".into() })?
  .send()
  .await?;

if resp.is_success() {
  let body: serde_json::Value = resp.json()?;
  println!("created: {body}");
}
# Ok(())
# }
```

## API at a glance

| Method | Purpose |
| --- | --- |
| `ImpulseRingClient::connect(app)` | Connect and target application `app`. |
| `ImpulseRingClient::connect_as(client, app)` | Connect under an explicit local name. |
| `ImpulseRingClient::with_connection(conn, app)` | Reuse one bus connection for many apps. |
| `.with_key(key)` / `.with_timeout(dur)` | Per-client access key / default timeout. |
| `.with_auto_reconnect(bool)` | Toggle transparent recovery after a broker restart (default: on). |
| `.get / .post / .put / .patch / .delete / .head / .request(m, uri)` | Start a request. |
| `RequestBuilder::header / .headers / .body / .json / .msgpack / .timeout` | Build the request. |
| `RequestBuilder::send().await` / `.send_blocking()` | Send it. |
| `RingResponse::status / .headers / .bytes / .text / .json / .msgpack / .error_for_status` | Read the response. |
| `.sse(uri)` / `.websocket(uri)` / `.webtransport(uri)` | Open a streaming session (see below). |
| `.connection()` | The shared bus `Connection`, for advanced channel work. |

## Streaming & upgrades

The base bus call is a single-shot RPC, so continuous flows are layered on top
of Ring **channels**: the initial RPC only negotiates the session and names the
channels, and the bytes travel as `RingStreamFrame`s. All of this lives in the
`streaming` module and requires the `async` feature (on by default).

- **SSE** — `client.sse("/events").await?` sends an ordinary handshake with
  `Accept: text/event-stream`; the server streams the response body onto a
  down-channel. Returns a `RingEventStream` (a `Stream` of raw event byte
  chunks, also usable via `.recv().await`).
- **WebSocket** — `client.websocket("/ws").await?` performs an
  `Upgrade: websocket` handshake and returns a `RingDuplex`: an
  `AsyncRead + AsyncWrite` "virtual socket". Drive any standard WebSocket
  client codec over it — salvo terminates the upgrade on the server side and
  Ring only relays the bytes.
- **WebTransport** — `client.webtransport("/wt").await?` returns a
  `RingWebTransport` session with datagrams (`send_datagram`/`recv_datagram`)
  and bidirectional streams (`open_bi`/`accept_bi`), mirroring the QUIC session
  API. The server handles these sessions via `ImpulseRingListener::on_webtransport`.

Each method also has a blocking variant (`open_sse_blocking`,
`open_websocket_blocking`, `open_webtransport_blocking`).

## Surviving an `impulsed` restart

If the broker restarts, the underlying bus connection detects it and
transparently reconnects — a client does **not** need to be rebuilt. This is on
by default; toggle with `.with_auto_reconnect(false)` (the setting is shared by
all clones of the client and by clients built from the same `Connection`).

What recovers, exactly:

- **Unary requests** — a request issued after the restart re-registers on the
  fresh broker and is retried once.
- **SSE** — an open stream is transparently re-handshaked against the fresh
  broker; events keep flowing (the server re-streams from the start).
- **WebSocket / WebTransport** — a live session cannot be resumed mid-stream:
  it reports a disconnect (broken pipe / EOF) after the restart and must be
  re-opened by the caller.

## Examples

A self-contained server and a `curl`-like CLI live in [`examples/`](examples):

```sh
# Terminal 1 — broker (impulse-ring repo)
cargo run -p impulsed

# Terminal 2 — a server that listens only over shared memory as `hello-ring`
cargo run -p impulse-client-ring --example ring-server

# Terminal 3 — call any method on it
cargo run -p impulse-client-ring --example ring-cli -- get  /hello
cargo run -p impulse-client-ring --example ring-cli -- post /echo --body 'hi there'
cargo run -p impulse-client-ring --example ring-cli -- \
  --app hello-ring put /thing -H 'content-type: application/json' --body '{"a":1}'
```

## Features

- `async` *(default)* — adds `RequestBuilder::send` (async) and the `streaming`
  module (SSE, WebSocket and WebTransport over Ring channels; needs Tokio's IO
  and sync primitives to bridge the blocking bus to async byte streams). The
  blocking unary API (`send_blocking`) is always available.

## Notes & limitations

- A plain request maps to one RPC (large bodies are streamed over side
  channels transparently). Continuous flows — SSE, WebSocket, WebTransport —
  are modelled over Ring channels via the `streaming` module; other HTTP
  upgrades are not supported over Ring.
- The client and the target server must share the same `impulse-ring-http`
  protocol version (the broker rejects a fingerprint mismatch).
- Linux only (Ring is POSIX-shared-memory based).
