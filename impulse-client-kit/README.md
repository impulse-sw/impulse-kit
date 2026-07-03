# Impulse Client Kit

Frontend framework with [`shadcn`-styled](https://ui.shadcn.com) components, based on [Leptos](https://leptos.dev/) v0.8.

## Usage

Just include it into your `Cargo.toml`:

```toml
[dependencies]
impulse-client-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.5.0" }
```

## Feature flags

Exactly one rendering mode must be enabled — they are mutually exclusive:

| Feature | Effect |
| --- | --- |
| `csr` *(default)* | Client-side rendering; `setup_app` mounts the app to the body. |
| `hydrate` | Hydrates server-rendered HTML in place (pair with `impulse-server-kit`'s `leptos-ssr`; see the [SSR showcase](./examples/ssr_showcase/README.md)). |
| `ssr` | Server-side build of the app crate; browser-side helpers become no-ops. |

Additional features:

| Feature | Effect |
| --- | --- |
| `telemetry` *(default)* | Monitor components + imperative telemetry helpers (no-op under `ssr`). |
| `websocket` | Reactive WebSocket bindings (requires `csr` or `hydrate`). |
| `webtransport` | Reactive WebTransport bindings (requires `csr` or `hydrate`). |

## Components and blocks

Low-level UI components (buttons, inputs, dialogs, …) live in the separate
[`impulse-client-kit-components`](./components/README.md) crate; higher-level
ready-made widgets (markdown, charts, node graph, landing-page sections) — in
[`impulse-client-kit-blocks`](./blocks/README.md).

## Simple application entrypoint

This is all you need to start Leptos application:

```rust
impulse_client_kit::setup_app(log::Level::Info, Box::new(move || { view! { <YourMainComponent /> }.into_any() }))
```

`setup_app` will automatically install given log level, set the panic error hook and initialize logs at `console`.

> [!NOTE]
> If your project is built at debug mode, logs will be set to `DEBUG` level automatically.

## Automated light/dark theme switch

Client Kit supports automated `dark` Tailwind class switching and also automated Thaw components styling.

To use automated light/dark theme switch, ensure to build your app on top of this [`index.html`](./examples/index.html) example:

```html
<!DOCTYPE html>
<html style="height: 100%; width: 100%;">
<head>
  <title>Your title</title>
  <link rel="shortcut icon" type="image/x-icon" href="/favicon.ico">
  <meta content="text/html;charset=utf-8" http-equiv="Content-Type" />
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta charset="UTF-8" />
  <script>
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      document.documentElement.setAttribute('data-theme', 'dark');
    } else {
      document.documentElement.setAttribute('data-theme', 'light');
    }
  </script>
  <style>
  [data-theme="light"] {
    --background-color: #fafafa;
  }
  [data-theme="dark"] {
    --background-color: #08080a;
  }
  body {
    background-color: var(--background-color);
    height: 100%;
    width: 100%;
    margin-top: 0px;
    margin-left: 0px;
  }
  .client-kit-app-container {
    min-height: 100%;
    min-width: 100%;
    overflow-x: auto;
  }
  .client-kit-app-content {
    display: flex;
    flex-direction: column;
  }
  </style>
  <link rel="stylesheet" href="/tailwind.css">
</head>
<body>
  <div id="main"></div>
</body>
</html>
```

## Router utils

Client Kit exposes `impulse_client_kit::router::endpoint` to construct full URL of the backend, if this backend provided your frontend also.

```rust
// Let assume that your backend is located at `127.0.0.1:8080` with HTTP schema
endpoint("/some/api/route") // equals to "http://127.0.0.1:8080/some/api/route"
```

If you need to go on any other page, use `impulse_client_kit::router::redirect`:

```rust
redirect("https://github.com")
```

## Telemetry

The `telemetry` feature (enabled by default) collects usage data and ships it to a
collection endpoint served by `impulse-server-kit`. Provide a context once near the
app root, then wrap views in monitor components or call the imperative helpers.

```rust
use impulse_client_kit::prelude::*;

#[component]
fn App() -> impl IntoView {
  // Anonymous by default; events carry only a random session id.
  provide_telemetry(TelemetryConfig::new("/api/telemetry"));

  view! {
    <ClickMonitor message="cta:signup">
      <Button>"Sign up"</Button>
    </ClickMonitor>

    // Reports an impression the first time it scrolls into view.
    <ViewMonitor message="hero:seen">
      <Hero />
    </ViewMonitor>
  }
}
```

Available monitors: `ClickMonitor`, `ViewMonitor` (impressions via `IntersectionObserver`),
`HoverMonitor`, `FocusMonitor`, `SubmitMonitor` and the generic `EventMonitor` (any DOM
event). Each accepts a `message` and an optional `endpoint` override.

Imperative helpers mirror `tracing` for ad-hoc logs, metrics and spans:

```rust
track_event(TelemetryEventKind::Custom, "video:played");
track_log(TelemetryLevel::Warn, "retry:checkout");
track_metric("cart:value", 42.0);
let _span = track_span("checkout:flow"); // duration reported on drop
```

### Anonymous vs. identified collection

A `TelemetryContext` is `Anonymous` by default — only a session-scoped random id is
sent. Switch to identified collection once a user is known:

```rust
let tele = use_telemetry().unwrap();
tele.set_mode(TelemetryMode::Identified);
tele.set_user_id(Some(user_id));
```

In anonymous mode the `user_id` is never transmitted even if one is set. Events are
delivered as MessagePack via `navigator.sendBeacon` (fire-and-forget, survives page
unloads). Under SSR the helpers are no-ops and monitors just render their children.

## WebSocket and WebTransport

Client Kit ships optional reactive wrappers around the browser `WebSocket` and `WebTransport` APIs. They are pulled in via Cargo features and are designed to mirror their server-side counterparts in `impulse-server-kit`.

### WebSocket

Enable the `websocket` feature:

```toml
[dependencies]
impulse-client-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.5.0", features = ["websocket"] }
```

Open a connection and observe state/messages reactively:

```rust
use impulse_client_kit::ws::{use_websocket, WebSocketMessage, WebSocketReadyState};
use impulse_client_kit::prelude::*;

let ws = use_websocket(format!("wss://{}/socket", impulse_client_kit::router::get_host()?))?;

Effect::new(move |_| {
  if ws.state.get() == WebSocketReadyState::Open {
    let _ = ws.send_text("hello");
  }
});

Effect::new(move |_| {
  match ws.message.get() {
    Some(WebSocketMessage::Text(text)) => log::info!("text: {text}"),
    Some(WebSocketMessage::Binary(bytes)) => log::info!("bin: {} bytes", bytes.len()),
    None => {}
  }
});
```

The connection is closed and event listeners are detached when the last `WebSocketHandle` clone is dropped.

### WebTransport

Enable the `webtransport` feature:

```toml
[dependencies]
impulse-client-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.5.0", features = ["webtransport"] }
```

The browser WebTransport API is gated by `web-sys` behind `--cfg=web_sys_unstable_apis`. Add this to your downstream `.cargo/config.toml`:

```toml
[target.wasm32-unknown-unknown]
rustflags = ["--cfg=web_sys_unstable_apis"]
```

Then:

```rust
use impulse_client_kit::wt::{use_webtransport, WebTransportState};
use impulse_client_kit::prelude::*;

let wt = use_webtransport("https://example.com/wt")?;
let datagrams = wt.datagram_signal()?;

Effect::new(move |_| {
  if wt.state.get() == WebTransportState::Open {
    leptos::task::spawn_local({
      let wt = wt.clone();
      async move { let _ = wt.send_datagram(b"ping").await; }
    });
  }
});

Effect::new(move |_| {
  if let Some(bytes) = datagrams.get() {
    log::info!("received {} bytes", bytes.len());
  }
});
```

`WebTransportHandle` also exposes `open_bidirectional_stream()` and `open_unidirectional_stream()` for application-level framing on top of QUIC streams. See [`wt.rs`](./src/wt.rs) for the full API.

### Automatic reconnection

Both wrappers can transparently re-establish a dropped connection. Reconnection is **off by default**; opt in with a `ReconnectOptions` policy. The delay before the first retry, the backoff multiplier, the maximum delay, and an optional cap on the number of attempts are all configurable:

```rust
use std::time::Duration;
use impulse_client_kit::prelude::*;

// Retry forever, starting at 500ms and doubling up to 10s.
let policy = ReconnectOptions::enabled()
  .with_initial_delay(Duration::from_millis(500))
  .with_max_delay(Duration::from_secs(10))
  .with_backoff_factor(2.0);

// WebSocket: pass the policy via `WebSocketOptions`.
let ws = use_websocket_with_options(
  "wss://example.com/socket",
  WebSocketOptions::default().with_reconnect(policy),
)?;

// WebTransport: a constant 1s delay capped at five attempts.
let wt = use_webtransport_with_reconnect(
  "https://example.com/wt",
  ReconnectOptions::enabled().with_backoff_factor(1.0).with_max_attempts(Some(5)),
)?;
```

While waiting between attempts the handle reports `Connecting`. The reactive `state`, inbound `message`/`datagram_signal`, sends, and stream constructors all keep working across reconnects — there is no need to re-create the handle. A close requested through `close()` (and, for WebTransport, a graceful close by either peer) is treated as final and never reconnects.

### Frozen-page recovery & connect watchdog

Reconnection can only react to a `close` event — and the browser does not
always deliver one. When a page is frozen into the back/forward cache (bfcache)
or a background tab is discarded, the connection's transport is torn down but
the `close` event can be dropped; on restore the handle would otherwise sit on
a dead socket forever, still reporting `Open`.

Both wrappers therefore hook the page-lifecycle events (via
`impulse_utils::page_lifecycle`) and revalidate the connection on resume:

- a bfcache restore (`pageshow` with `persisted == true`) forces a fresh
  reconnect unconditionally — the restored socket is stale by definition;
- the network coming back (`online`) or the tab becoming visible again
  triggers a reconnect only if the connection actually looks dead.

Independently, every connect attempt is covered by a **watchdog**: if an
attempt (including an async URL provider that never resolves) wedges, it is
timed out and counted as a failed attempt, so backoff and the attempt cap keep
working instead of the handle hanging in `Connecting` forever.

Both behaviours are built in and require no configuration.

### Per-attempt URL (token-refreshing reconnect)

The static-URL constructors capture the URL once. When the connection needs a
**fresh value on every attempt** — most commonly a single-use auth ticket that
can't ride a socket handshake as a cookie — supply an **async URL provider**
instead. It is invoked once per (re)connect, so each attempt can mint a new
ticket and bake it into the URL; a provider error is treated like a failed open,
so backoff and the attempt cap still apply.

```rust
use impulse_client_kit::ws::{use_websocket_with_url_fn, WebSocketOptions};
use impulse_client_kit::prelude::*;

let ws = use_websocket_with_url_fn(
  // Called again on every reconnect — fetch a fresh ticket each time.
  || async move {
    let ticket = fetch_ws_ticket().await?;
    Ok(format!("wss://example.com/socket?ticket={ticket}"))
  },
  WebSocketOptions::default().with_reconnect(ReconnectOptions::enabled()),
)?;
```

WebTransport has the same shape via `use_webtransport_with_url_fn(provider, options, reconnect)`.
Both also expose the lower-level `use_*_with_provider` taking an
`Rc<dyn Fn() -> …>` if you already hold a boxed provider.

## Some other utils

See [`utils.rs` file](./src/utils.rs).
