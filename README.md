# cc-services

Collection of Rust libraries, frameworks and programs to build better Internet.

## Overview

### CC Server Kit

Server Kit is a simply configurable backend framework based on [Salvo](https://github.com/salvo-rs/salvo). It is simple enough and powerful.

[Server Kit Documentation](./cc-server-kit/README.md)

### CC Static Server

Static Server is simple frontend-to-client provider built with Server Kit. You can edit `static-server.yaml` to specify Server Kit parameters.

On its own, Static Server serves all files from one of distribution folders:

- `/usr/local/frontend-dist`
- `{CURRENT_EXE_PATH}/dist`

And more! It internally redirects all requests without file extension to `index.html`, and your SPA apps can run smoothly.

Also, you can use Static Server as a library to include frontend router to your backend application:

```rust
  let router = cc_server_kit::get_root_router(&state)
    .hoop(
      affix_state::inject(state.clone())
        .inject(setup.clone())
        .inject(connect_sea_orm().await?)
        .inject(auth_cli),
    )
    .push(crate::api::auth_router())
    .push(crate::api::chat_router())
    .push(cc_static_server::frontend_router()); // include it in the end for correct redirects
```

Also, you can specify distribution path:

```rust
  ...
    .push(cc_static_server::frontend_router_from_given_dist(&PathBuf::from("/any/other/folder")));
```

[Static Server Documentation](./cc-static-server/README.md)

### `cc-utils`

`cc-utils` is a bunch of fullstack utils:

- common error types: `ServerError`, `CliError` and `ErrorResponse`
- unified result types: `MResult<T> = Result<T, ServerError>` and `CResult<T> = Result<T, CliError>`
- backend response types for Salvo and Server Kit: `ok!()`, `plain!(str)`, `html!(str)`, `file_upload!(pathbuf, filename)`, `json!(ser)` and `msgpack!(ser)`
- `ExplicitServerWrite` backend trait which uses only `&mut Response` to respond unlike `ServerResponseWriter::write(self, req, depot, res)`
- MsgPack extraction traits for `reqwest::Response` and `salvo::Request`
- MsgPack send trait for `reqwest::RequestBuilder`
- SIMD JSON support

In a way, `cc-utils` is useful in many cases such as error handling and response writing.

[`cc-utils` Documentation](./cc-utils/README.md)

### CC UI Kit

UI Kit is just superstructure above Leptos and ThawUI frameworks. It provides:

- simple application entrypoint
- logging support with `log`
- automated light/dark themes (with Tailwind support)
- utils to perform request to the backend (`cc_ui_kit::router::endpoint` and `cc_ui_kit::router::redirect` functions)

Startup example:

```rust
fn main() {
  cc_ui_kit::setup_app(log::Level::Info, Box::new(move || { view! { <App /> }.into_any() }))
}
```

[CC UI Kit Documentation](./cc-ui-kit/README.md)

[CC UI Kit Example](https://github.com/impulse-sw/cc-ui-kit-example)
