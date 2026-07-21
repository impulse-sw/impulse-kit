# impulse-kit

Collection of Rust libraries, frameworks and programs to build better Internet.

## Workspace map

| Crate | What it is |
| --- | --- |
| [`impulse-endpoint`](./impulse-endpoint) | Transport-agnostic HTTP wire types and a small endpoint/router abstraction. |
| [`impulse-server-kit`](./impulse-server-kit) | Backend framework on top of Salvo: multi-protocol listener (HTTP/1.1–3 + Ring shared memory), YAML config, tracing/OTel, OpenAPI, Leptos SSR. |
| [`impulse-server-kit-dsl`](./impulse-server-kit-dsl) | `skdsl` — DSL-to-API translator: endpoint prototypes, version bumping, Rust/JS clients. |
| [`impulse-static-server`](./impulse-static-server) | Static frontend server (`iks` binary) and SPA routers as a library. |
| [`impulse-utils`](./impulse-utils) | Fullstack utils: errors/results, response macros, MsgPack & SIMD JSON, telemetry wire types, page-lifecycle recovery. |
| [`impulse-client-kit`](./impulse-client-kit) | Frontend framework over Leptos: entrypoints, themes, WS/WT bindings, telemetry. |
| [`impulse-client-kit-components`](./impulse-client-kit/components) | 60+ shadcn-styled UI components. |
| [`impulse-client-kit-blocks`](./impulse-client-kit/blocks) | Higher-level blocks: Markdown, charts, node graph, landing-page sections. |
| [`impulse-tailwind-sources`](./impulse-client-kit/tailwind-sources) | Build-script glue that lets Tailwind scan component crates from the Cargo registry. |
| [`impulse-client-ring`](./impulse-client-ring) | `reqwest`-style HTTP client over the Ring shared-memory bus (incl. SSE/WS/WT). |
| [`impulse-error-pages`](./impulse-error-pages) | Ready-made error pages (400–500) as a static frontend bundle. |

## Overview

### Impulse Server Kit

Server Kit is a simply configurable backend framework based on [Salvo](https://github.com/salvo-rs/salvo). It is simple enough and powerful.

It also ships a telemetry collection endpoint (the `telemetry` feature, on by default): pair it with `impulse-client-kit`'s telemetry monitors to ingest client-side events through a pluggable `TelemetrySink`, defaulting to the existing tracing/OpenTelemetry stack.

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

### Impulse Client Kit

Client Kit is just superstructure above Leptos framework. It provides:

- simple application entrypoint (CSR, hydrate and SSR modes)
- logging support with `log`
- automated light/dark themes (with Tailwind support)
- utils to perform request to the backend (`impulse_client_kit::router::endpoint` and `impulse_client_kit::router::redirect` functions)
- WebSocket & WebTransport bindings (with optional automatic reconnection, including an async per-attempt URL provider for token-refreshing reconnects, frozen-page/bfcache recovery and a per-attempt connect watchdog)
- telemetry collection: monitor components (`<ClickMonitor>`, `<ViewMonitor>`, `<HoverMonitor>`, `<FocusMonitor>`, `<SubmitMonitor>`, `<EventMonitor>`) plus imperative `track_event`/`track_log`/`track_metric`/`track_span` helpers, with anonymous or identified collection (see the `telemetry` module)

UI lives in two companion crates: [`impulse-client-kit-components`](./impulse-client-kit/components/README.md) (60+ shadcn-styled components) and [`impulse-client-kit-blocks`](./impulse-client-kit/blocks/README.md) (Markdown, charts, an interactive node graph, landing-page sections). Their Tailwind classes are wired into the consuming app's build via [`impulse-tailwind-sources`](./impulse-client-kit/tailwind-sources/README.md).

Startup example:

```rust
fn main() {
  impulse_client_kit::setup_app(log::Level::Info, Box::new(move || { view! { <App /> }.into_any() }))
}
```

[Client Kit Documentation](./impulse-client-kit/README.md)

[Client Kit Showcase](./impulse-client-kit/examples/showcase/README.md)

[Server-Side Rendered Showcase](./impulse-client-kit/examples/ssr_showcase/README.md)

### Impulse Client Ring

Client Ring is a [`reqwest`](https://docs.rs/reqwest)-style HTTP client that
talks over the **Ring** shared-memory IPC bus instead of TCP/Unix sockets. It is
the client half of Server Kit's `impulse-ring` listener: a server registers an
application on the bus and serves HTTP over shared memory, and Client Ring looks
it up by name and issues ordinary requests — no ports, no kernel round-trips on
the data path. Beyond unary requests it speaks SSE, WebSocket and WebTransport
over Ring channels, streams large bodies transparently, and survives an
`impulsed` broker restart without being rebuilt. Ships with a server example
and a `curl`-like CLI example.

[Client Ring Documentation](./impulse-client-ring/README.md)

### Impulse Error Pages

Ready-made error pages for Impulse services (400, 401, 403, 404, 405 & 500),
built as a small Leptos frontend — serve the bundle with Static Server (or any
static server) and redirect to `/{status-code}`.

[Error Pages Documentation](./impulse-error-pages/README.md)

## Build profiles

`impulse-kit` ships two release profiles:

| Profile | Used for | `panic` |
| --- | --- | --- |
| `release` | Native binaries (`iks` Static Server, `ring-server`, any Server Kit backend) | `unwind` (default) |
| `wasm-release` | WASM frontends (`inherits = "release"`) | `immediate-abort` |

`immediate-abort` strips the panic/formatting machinery for a much smaller WASM
bundle — a good trade-off in the browser, where a crashed tab is recoverable.
It is deliberately **not** applied to native release builds: there, turning
every reachable panic into an `abort()` of the whole process is a remote-DoS
surface for a long-running server. Native binaries therefore unwind.

Build WASM with the `wasm-release` profile (the `cargo-wasm-rel` Deployer action
already does this):

```sh
cargo build --profile wasm-release --lib --target wasm32-unknown-unknown -p <crate>
```

Trunk-driven frontends additionally pass `-Cpanic=immediate-abort` via
`RUSTFLAGS` together with `build-std`, so they get the same size win regardless
of profile.

## Rust Toolchain

**This repository actively uses `nightly` toolchain.** While these frameworks and libraries are battle-tested anyway, consider not to choose `impulse-kit` to use if you are not aware of `nightly` toolchain.
