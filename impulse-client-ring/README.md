# impulse-client-ring

A [`reqwest`](https://docs.rs/reqwest)-style HTTP client that talks to a server
over the **Ring** shared-memory IPC bus instead of TCP/Unix sockets.

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
| `.get / .post / .put / .patch / .delete / .head / .request(m, uri)` | Start a request. |
| `RequestBuilder::header / .headers / .body / .json / .timeout` | Build the request. |
| `RequestBuilder::send().await` / `.send_blocking()` | Send it. |
| `RingResponse::status / .headers / .bytes / .text / .json / .error_for_status` | Read the response. |

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

- `async` *(default)* — adds `RequestBuilder::send` (async). The blocking API
  (`send_blocking`) is always available.

## Notes & limitations

- One request maps to one RPC: request/response only. Streaming bodies,
  WebSockets and HTTP upgrades are not modelled over Ring — use a TCP `http1`
  listener for those.
- The client and the target server must share the same `impulse-ring-http`
  protocol version (the broker rejects a fingerprint mismatch).
- Linux only (Ring is POSIX-shared-memory based).
