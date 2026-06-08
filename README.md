# impulse-kit

Collection of Rust libraries, frameworks and programs to build better Internet.

## Overview

### Impulse Server Kit

Server Kit is a simply configurable backend framework based on [Salvo](https://github.com/salvo-rs/salvo). It is simple enough and powerful.

[Server Kit Documentation](./impulse-server-kit/README.md)

### Impulse Server Kit DSL

Server Kit provides DSL-to-API prototype translator to simplify development:

- automated version bumping on breaking changes
- automated OpenAPI spec generation
- automated OpenTelemetry instrumenting.

SK DSL allows you to export server API prototypes (you just need to implement endpoints' logic) and Rust & JS clients for this API.

[SK DSL Documentation](./impulse-server-kit-dsl/README.md)

### Impulse Static Server

Static Server is simple frontend-to-client provider built with Server Kit. You can edit `static-server.yaml` to specify Server Kit parameters.

On its own, Static Server serves all files from one of distribution folders:

- `/usr/local/frontend-dist`
- `{CURRENT_EXE_PATH}/dist`

And more! It internally redirects all requests without file extension to `index.html`, and your SPA apps can run smoothly.

Also, you can use Static Server as a library to include frontend router to your backend application:

```rust
  let router = impulse_server_kit::get_root_router(&state)
    .hoop(
      affix_state::inject(state.clone())
        .inject(setup.clone())
        .inject(connect_sea_orm().await?)
        .inject(auth_cli),
    )
    .push(crate::api::auth_router())
    .push(crate::api::chat_router())
    .push(impulse_static_server::frontend_router()?); // include it in the end for correct redirects
```

Also, you can specify distribution path:

```rust
  ...
    .push(impulse_static_server::frontend_router_from_given_dist(&PathBuf::from("/any/other/folder"))?);
```

[Static Server Documentation](./impulse-static-server/README.md)

### `impulse-utils`

`impulse-utils` is a bunch of fullstack utils:

- common error types: `ServerError`, `ClientError` and `ErrorResponse`
- unified result types: `MResult<T> = Result<T, ServerError>` and `CResult<T> = Result<T, ClientError>`
- backend response types for Salvo and Server Kit: `ok!()`, `plain!(str)`, `html!(str)`, `file_upload!(pathbuf, filename)`, `json!(ser)` and `msgpack!(ser)`
- `ExplicitServerWrite` backend trait which uses only `&mut Response` to respond unlike `ServerResponseWriter::write(self, req, depot, res)`
- MsgPack extraction traits for `reqwest::Response` and `salvo::Request`
- MsgPack send trait for `reqwest::RequestBuilder`
- SIMD JSON support

In a way, `impulse-utils` is useful in many cases such as error handling and response writing.

[`impulse-utils` Documentation](./impulse-utils/README.md)

### Impulse UI Kit

UI Kit is just superstructure above Leptos framework. It provides:

- simple application entrypoint
- logging support with `log`
- automated light/dark themes (with Tailwind support)
- utils to perform request to the backend (`impulse_ui_kit::router::endpoint` and `impulse_ui_kit::router::redirect` functions)
- WebSocket & WebTransport bindings (with optional automatic reconnection)

Startup example:

```rust
fn main() {
  impulse_ui_kit::setup_app(log::Level::Info, Box::new(move || { view! { <App /> }.into_any() }))
}
```

[UI Kit Documentation](./impulse-ui-kit/README.md)

[UI Kit Showcase](./impulse-ui-kit/examples/showcase/README.md)

[Server-Side Rendered Showcase](./impulse-ui-kit/examples/ssr_showcase/README.md)

## Rust Toolchain

**This repository actively uses `nightly` toolchain.** While these frameworks and libraries are battle-tested anyway, consider not to choose `impulse-kit` to use if you are not aware of `nightly` toolchain.
