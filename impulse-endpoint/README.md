# impulse-endpoint

Transport-agnostic HTTP wire types and a tiny endpoint/router abstraction — the
neutral foundation of the Impulse request stack. It depends on neither `reqwest`,
`salvo`, `tauri` nor `leptos`, so the same request-handling logic can be **written
once and mounted on either host**:

* on the **server**, via the `impulse-server-kit` salvo adapter
  (`endpoint_router`);
* in a **Tauri app**, via `impulse-tauri-engine`, which serves the identical
  routes from a local store while offline.

## What's in it

### Wire types

`Method`, `HttpRequest`, `HttpResponse` — small, serialisable types that describe
a request/response independently of how it's executed. They cross the Tauri IPC
boundary unchanged and are re-exported by `impulse_client_kit::client` (the
browser/Tauri transport) and used by `impulse-tauri-engine` (the native side).

```rust
use impulse_endpoint::{HttpRequest, HttpResponse, Method};
```

### Endpoint / Router

Write a handler once as an `Endpoint<S>` over an app state `S`, add it to a
`Router<S>`, and hand that router to whichever host runs it.

```rust
use impulse_endpoint::{Endpoint, EndpointCtx, EndpointFuture, EndpointResponse, Method, Router};

struct GetItem;
impl Endpoint<Db> for GetItem {
    fn call<'a>(&'a self, ctx: EndpointCtx<'a, Db>) -> EndpointFuture<'a> {
        Box::pin(async move {
            let id: i64 = ctx.params.parse("id")?;          // {id} from the path
            let me = ctx.require_identity()?;               // host-supplied identity
            let item = ctx.state.get(me, id).await?;        // your business logic
            EndpointResponse::json(&item)
        })
    }
}

let routes = Router::<Db>::new()
    .route(Method::Get,  "/api/v1/items/{id}", GetItem)
    .route(Method::Post, "/api/v1/items",      CreateItem);
```

`EndpointCtx` gives handlers `state`, an optional `identity` (resolved by the
host — auth middleware on the server, the signed-in user in the engine), the
matched path `params`, the `query`, `headers` and the raw `body`, plus helpers
(`require_identity`, `json_body`, `query_param`). `EndpointResponse::{json,
empty, from_error}` build the reply; the host adapter turns it into a real HTTP
response.

Handlers are object-safe (`Box<dyn Endpoint<S>>` via a boxed future), so one
`Router` holds heterogeneous handlers over a shared state.

## Features

* `reqwest` *(off by default)* — adds `impl From<Method> for reqwest::Method` for
  the client transport. The server and engine use the wire types and router
  without pulling `reqwest`.

## Where it fits

```
impulse-endpoint  ── wire types + Endpoint/Router (this crate)
      ├── impulse_client_kit::client   (browser fetch / Tauri IPC transport)
      ├── impulse-tauri-engine         (native offline engine; runs a Router locally)
      └── impulse-server-kit::endpoint (salvo adapter; mounts a Router on the server)
```
