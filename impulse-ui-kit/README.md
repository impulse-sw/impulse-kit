# Impulse UI Kit

Frontend framework with [`shadcn`-styled](https://ui.shadcn.com) [Thaw](https://thawui.vercel.app) components, based on [Leptos](https://leptos.dev/) v0.7.

## Usage

Just include it into your `Cargo.toml`:

```toml
[dependencies]
impulse-ui-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "0.11" }
```

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

## Some other utils

See [`utils.rs` file](./src/utils.rs).
