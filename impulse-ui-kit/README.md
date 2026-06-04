# Impulse UI Kit

Frontend framework with [`shadcn`-styled](https://ui.shadcn.com) components, based on [Leptos](https://leptos.dev/) v0.8.

## Usage

Just include it into your `Cargo.toml`:

```toml
[dependencies]
impulse-ui-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.2.0" }
```

## Components and its usage

See the [components README.md](./components/README.md).

## Simple application entrypoint

This is all you need to start Leptos application:

```rust
impulse_ui_kit::setup_app(log::Level::Info, Box::new(move || { view! { <YourMainComponent /> }.into_any() }))
```

`setup_app` will automatically install given log level, set the panic error hook and initialize logs at `console`.

> [!NOTE]
> If your project is built at debug mode, logs will be set to `DEBUG` level automatically.

## Automated light/dark theme switch

UI Kit supports automated `dark` Tailwind class switching and also automated Thaw components styling.

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
  .uikit-app-container {
    min-height: 100%;
    min-width: 100%;
    overflow-x: auto;
  }
  .uikit-app-content {
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

UI Kit exposes `impulse_ui_kit::router::endpoint` to construct full URL of the backend, if this backend provided your frontend also.

```rust
// Let assume that your backend is located at `127.0.0.1:8080` with HTTP schema
endpoint("/some/api/route") // equals to "http://127.0.0.1:8080/some/api/route"
```

If you need to go on any other page, use `impulse_ui_kit::router::redirect`:

```rust
redirect("https://github.com")
```

## WebSocket and WebTransport

UI Kit ships optional reactive wrappers around the browser `WebSocket` and `WebTransport` APIs. They are pulled in via Cargo features and are designed to mirror their server-side counterparts in `impulse-server-kit`.

### WebSocket

Enable the `websocket` feature:

```toml
[dependencies]
impulse-ui-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.2.0", features = ["websocket"] }
```

Open a connection and observe state/messages reactively:

```rust
use impulse_ui_kit::ws::{use_websocket, WebSocketMessage, WebSocketReadyState};
use impulse_ui_kit::prelude::*;

let ws = use_websocket(format!("wss://{}/socket", impulse_ui_kit::router::get_host()?))?;

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
impulse-ui-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.2.0", features = ["webtransport"] }
```

The browser WebTransport API is gated by `web-sys` behind `--cfg=web_sys_unstable_apis`. Add this to your downstream `.cargo/config.toml`:

```toml
[target.wasm32-unknown-unknown]
rustflags = ["--cfg=web_sys_unstable_apis"]
```

Then:

```rust
use impulse_ui_kit::wt::{use_webtransport, WebTransportState};
use impulse_ui_kit::prelude::*;

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

## Some other utils

See [`utils.rs` file](./src/utils.rs).
