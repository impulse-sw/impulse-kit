# Impulse Server Kit

State-of-art simple and powerful web server based on [Salvo](https://github.com/salvo-rs/salvo). Provides extended tracing, configuration-over-YAML, a multi-protocol listener (HTTP/1.1, HTTP/2, HTTP/3 and the **Ring shared-memory bus**), TLS v1.3, MessagePack + SIMD JSON ser/de support, OpenAPI and OpenTelemetry features *by default*.

Table of contents:

- [How's it work](#1)
- [Using Server Kit](#2)
- [Extended utilities](#3)
- [4 Quick start steps](#4)
- [Common Salvo documentation](#5)
- [Code API Overview](#6)
- [Configuration Overview](#7)
- [Crawlers: robots.txt & sitemap.xml](#8)
- [Leptos SSR & SEO](#9)

<a name="1"></a>
## How's it work

1. You load configuration from the file on the startup via `load_generic_config` function.
2. You start logging, check config for misconfigurations and load the state - all just via `load_generic_state` function.
3. You create your own `salvo::Router` and then generate server's `Future` and handle by `start` function.
4. You manually start awaiting `server`.

<a name="2"></a>
## Using Server Kit

To use Server Kit, include this line into your `Cargo.toml`:

```toml
[dependencies]
impulse-server-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", branch = "release/1.8.x" }
```

And create empty `{app-name}.yaml` to fill later (see [Configuration Overview](#7) below).
The config is looked up in the current working directory first, then in
`/etc/{app-name}.yaml` — handy for system-wide deployments.

### Feature flags

Default features: `http3`, `cors`, `acme`, `oapi`, `otel`, `telemetry`,
`force-https`, `impulse-ring`.

Notable optional features:

| Feature | Effect |
| --- | --- |
| `http3` | HTTP/3 (QUIC) protocol support. |
| `impulse-ring` | Listen for HTTP over the Ring shared-memory bus. |
| `oapi` | OpenAPI spec generation and frontends (Scalar / SwaggerUI). |
| `otel` | OpenTelemetry span & metric exporters. |
| `telemetry` | Client-side telemetry collection endpoint (see below). |
| `websocket`, `sse` | Salvo's WebSocket / SSE support (both also work over Ring). |
| `static-server` | Static file routers with in-memory caching (powers `impulse-static-server`). |
| `leptos-ssr` | Leptos server-side-rendering adapter (see [Leptos SSR & SEO](#8)). |
| `leptos-server-fn` | `#[server]` functions dispatch bridged to Salvo. |
| `proxy`, `cache`, `csrf`, `session`, `jwt-auth`, `basic-auth`, … | Thin re-exports of the matching salvo features. |

There is also a `test_exts` module (`ResponseExt`) with helpers for asserting
on responses in integration tests — taking the body as `String`/bytes/JSON with
`Content-Encoding` (gzip/zlib/zstd/brotli) transparently decoded.

<a name="3"></a>
## Extended utilities

Server Kit uses `impulse-utils` to improve functionality by:

- providing describeful `ServerError` and associated `MResult`
- providing SIMD JSON and MsgPack support
- easy response macros

Read more: [`impulse-utils`](./../impulse-utils/README.md).

<a name="4"></a>
## 4 Quick start steps

1. Create `Setup` struct with your setup data fields and `GenericValues` inside.
2. Create simple endpoints - your HTTP requests' handlers.
3. Create `server-example.yaml` file in crate root.
4. Just setup your application in 5 lines in `main`.

YAML configuration example:

```yaml
protocols:
  - type: http1
    host: 127.0.0.1
    port: 8801
allow_oapi_access: true
oapi_frontend_type: Scalar
oapi_name: Server Test OAPI
oapi_ver: 0.0.1
oapi_api_addr: /api
enable_io_logs: true
io_log_level: debug
```

`Cargo.toml`:

```toml
[package]
name = "impulse-server-example"
version = "0.1.0"
edition = "2024"

[dependencies]
impulse-server-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", branch = "release/1.8.x" }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["macros"] }
tracing = "0.1"
```

The code itself:

```rust
use impulse_server_kit::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Default, Clone)]
struct Setup {
  #[serde(flatten)]
  generic_values: GenericValues,
  // this could be your global variables, such as the database URLs
}

impl GenericSetup for Setup {
  fn generic_values(&self) -> &GenericValues { &self.generic_values }
  fn generic_values_mut(&mut self) -> &mut GenericValues { &mut self.generic_values }
}

#[derive(Deserialize, Serialize, Debug, salvo::oapi::ToSchema)]
/// Some hello
struct HelloData {
  /// Hello's text
  text: String,
}

#[endpoint(
  tags("test"),
  request_body(content = HelloData, content_type = "application/json", description = "Some JSON hello to MsgPack"),
  responses((status_code = 200, description = "Some MsgPack hello", body = HelloData, content_type = ["application/msgpack"]))
)]
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
/// Convert hello from JSON to MsgPack
///
/// Simple endpoint which translates any given `HelloData` from JSON into MsgPack format.
async fn json_to_msgpack(req: &mut Request, depot: &mut Depot) -> MResult<MsgPack<HelloData>> {
  let hello = req.parse_json::<HelloData>().await?;
  let app_name = depot.obtain::<Setup>()?.generic_values().app_name.as_str();
  msgpack!(HelloData { text: format!("From `{}` application: {}", app_name, hello.text) })
}

#[endpoint(
  tags("test"),
  request_body(content = HelloData, content_type = "application/msgpack", description = "Some MsgPack hello to JSON"),
  responses((status_code = 200, description = "Some JSON hello", body = HelloData, content_type = ["application/json"]))
)]
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
/// Convert hello from MsgPack to JSON
///
/// Simple endpoint which translates any given `HelloData` from MsgPack into Json format.
async fn msgpack_to_json(req: &mut Request, depot: &mut Depot) -> MResult<Json<HelloData>> {
  let hello = req.parse_msgpack::<HelloData>().await?;
  let app_name = depot.obtain::<Setup>()?.generic_values().app_name.as_str();
  json!(HelloData { text: format!("From `{}` application: {}", app_name, hello.text) })
}

fn tests_router() -> Router {
  Router::new()
    .push(Router::with_path("msgpack-to-json").post(msgpack_to_json))
    .push(Router::with_path("json-to-msgpack").post(json_to_msgpack))
}

#[tokio::main]
async fn main() {
  let setup = load_generic_config::<Setup>("server-example").await.unwrap();
  let state = load_generic_state(&setup, true).await.unwrap();
  
  // any setup, like DB or auth client
  
  let router = get_root_router_autoinject(&state, setup.clone())
    // .hoop(salvo::affix_state::inject(my_db_client).inject(my_auth_client))
    .push(tests_router());
  let (server, _handler) = start(state, &setup, router).await.unwrap();
  server.await
}
```

Here we go! You can now start the server with `cargo run`!

<a name="5"></a>
## Common Salvo documentation

Server Kit is just a layer on top of Salvo framework. Use its [documentation and examples](https://salvo.rs/guide/quick-start.html) to know how to develop web servers in Server Kit.

<a name="6"></a>
## Code API Overview

> [!NOTE]
> To setup these features, you have to write them in your code.

<a id="logging-inside-code"></a>
### Logging

To install log collector application-wide, make sure that you've loaded generic state with `true` second option:

```rust
let state = load_generic_state(&setup, true).await.unwrap();
```

And, for logs, use either provided or included by yours `tracing` crate:

```rust
use tracing;  // or `use impulse_server_kit::prelude::*;

// inside any function
tracing::info!("There are {} available chickens!", chickens.len());
```

<a id="otel-inside-code"></a>
### OpenTelemetry

Spans example:

```rust
// Import `tracing` module
use impulse_server_kit::prelude::*;

// Use `tracing::instrument` attribute macro to instrument your handler and automatically create `my_handler` span
#[handler]
#[tracing::instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
async fn my_handler() -> MResult<OK> {
  // Use `tracing` instead of `log`
  tracing::debug!("This is the DEBUG level log!");
  
  // Use `.instrument(...)` method over async functions to define spans
  any_async_func
    .instrument(tracing::info_span!("Executed any async function"))
    .await;
  
  ok!()
}
```

Metrics example:

```rust
// Import `otel` module
use impulse_server_kit::prelude::*;

// Get a meter
let meter = otel::api::global::meter("my_meter");

// Create a metric
let counter = meter.u64_counter("my_counter").build();
counter.add(1, &[KeyValue::new("key", "value")]);
```

### Telemetry collection

The `telemetry` feature (enabled by default) adds an endpoint that ingests batches of
client-side telemetry events produced by `impulse-client-kit`'s telemetry monitors. Each
event is handed to a `TelemetrySink`; the default `TracingTelemetrySink` forwards events
into the existing tracing/OpenTelemetry stack (emitting the `client_telemetry_events`
counter and `client_telemetry_values` histogram when `otel` is on).

Mount the default router (events go to tracing/OpenTelemetry):

```rust
use impulse_server_kit::prelude::*;
use impulse_server_kit::telemetry::default_telemetry_router;

let router = get_root_router_autoinject(&state, setup.clone())
  .push(default_telemetry_router("api/telemetry"));
```

To persist events yourself, implement `TelemetrySink` and pass it to `telemetry_router`:

```rust
use std::sync::Arc;
use impulse_server_kit::prelude::*;

struct DbSink { /* pool, ... */ }

#[salvo::async_trait]
impl TelemetrySink for DbSink {
  async fn record(&self, event: &TelemetryEvent, ctx: &TelemetryRequestCtx) {
    // store `event` (anonymous or identified) however you like
  }
}

let router = get_root_router_autoinject(&state, setup.clone())
  .push(telemetry_router("api/telemetry", Arc::new(DbSink { /* ... */ })));
```

The endpoint accepts MessagePack (the client's canonical transport) and JSON, selected by
`Content-Type`.

### Force HTTPS

To enforce HTTPS, you should start another server via `start_force_https_redirect` function:

```rust
let (server, handler) = start_force_https_redirect(80, 443).await.unwrap();
```

<a name="7"></a>
## Configuration Overview

> [!NOTE]
> To setup these features, you have no need to edit code, just `{your-app}.yaml`.

### Protocols

The server listens on a **set** of protocols declared under the `protocols:`
key — any mix of the four below, all at once. The list must be non-empty.

| `type` | Transport | Required fields | Feature |
| --- | --- | --- | --- |
| `http1` | HTTP/1.1 over TCP. Cleartext, or HTTPS with TLS. Required for WebSockets. | `host`, `port` (+ optional `ssl_key_path`, `ssl_crt_path`) | — |
| `http2` | HTTP/2 over TCP. Cleartext h2c, or HTTPS (h2 over TLS). | `host`, `port` (+ optional `ssl_key_path`, `ssl_crt_path`) | — |
| `http3` | HTTP/3 over QUIC (TLS v1.3). | `host`, `port`, `ssl_key_path`, `ssl_crt_path` | `http3` |
| `impulse-ring` | HTTP over the Ring shared-memory bus — no socket. | `app_name` (+ optional `access_key`, `arena_size_kib`) | `impulse-ring` |

For `http1`/`http2`, TLS is **all-or-nothing**: provide both `ssl_key_path` and
`ssl_crt_path` to terminate HTTPS over TCP (TLS 1.2 + 1.3, for broad client
compatibility), or omit both for cleartext. `http3` always requires them (QUIC
mandates TLS 1.3).

The `type` values accept aliases: `http1` = `http/1.1` = `http1.1` =
`http_localhost`, `http2` = `http/2`, `http3` = `http/3` = `quic`,
`impulse-ring` = `impulse_ring` = `ring`.

```yaml
protocols:
  - type: http1            # needed for WebSockets
    host: 0.0.0.0
    port: 8080
  - type: http2
    host: 0.0.0.0
    port: 8081
  - type: http2            # HTTPS over TCP (h2 + http/1.1 via ALPN)
    host: 0.0.0.0
    port: 8443
    ssl_key_path: certs/privkey.pem
    ssl_crt_path: certs/fullchain.pem
  - type: http3            # QUIC, TLS v1.3 only
    host: 0.0.0.0
    port: 8082
    ssl_key_path: certs/privkey.pem
    ssl_crt_path: certs/fullchain.pem
  - type: impulse-ring     # shared-memory IPC, addressed by name
    app_name: my-service
```

When any `http3` protocol is present, the cleartext listeners automatically
advertise the QUIC upgrade via an `alt-svc` header.

#### Listening over the Ring shared-memory bus

The `impulse-ring` protocol (feature `impulse-ring`, **on by default**) serves
HTTP over the [Ring](https://github.com/impulse-sw/impulse-ring) shared-memory
IPC bus instead of a socket. It is the server-side counterpart of the
[`impulse-client-ring`](../impulse-client-ring) client (a `reqwest`-style API).

- The `impulsed` broker must be running; it owns the shared-memory control
  segment.
- Clients address the server by `app_name` — there is no host/port.
- Each request runs the **full salvo pipeline** (routing, middleware, OpenAPI,
  catcher), so handlers behave exactly as over TCP.
- Large bodies are handled transparently: an oversized request body arrives
  streamed over a side channel and is reassembled before salvo sees it, and an
  oversized response body is chunked back the same way.
- **SSE and WebSocket work over Ring too** (features `sse` / `websocket`): the
  listener detects the upgrade handshake and relays the byte stream over Ring
  channels, so salvo's ordinary SSE/`WebSocketUpgrade` handlers run unchanged.
  WebTransport sessions are Ring-native — register a handler via
  `ImpulseRingListener::on_webtransport` (salvo's own WebTransport needs QUIC,
  which Ring does not provide).
- If `impulsed` restarts, the listener transparently re-registers on the fresh
  broker (and logs the re-registration), so the service keeps serving without a
  restart of its own.

```yaml
protocols:
  - type: impulse-ring
    app_name: my-service
    # access_key: optional-shared-secret
    # arena_size_kib: 4096   # per-service request-arena size, see below
```

`arena_size_kib` sets the request-arena capacity of this application's bus
function (default 512 KiB; the broker clamps it to [256 KiB, 128 MiB] and
rounds up to a power of two). A larger arena lets a high-throughput service
buffer more in-flight requests before callers hit backpressure; it does *not*
raise the max inline body — large bodies stream over a channel regardless.

You can also drive the listener by hand without YAML:

```rust
use impulse_server_kit::prelude::*;

let listener = ImpulseRingListener::new("my-service")
  // .with_key("optional-shared-secret")
  // .with_arena_cap(4 * 1024 * 1024)
  // .on_webtransport(handler)
  ;
let service = salvo::Service::new(router);
// Serves until `shutdown` resolves; unregisters from the bus on completion.
serve_impulse_ring(listener, service, shutdown_future).await?;
// or: listener.serve(service, shutdown_future).await?;
```

### Auto-migrate binary

Specify `auto_migrate_bin` field to automatically execute any binary (for example, DB migrations) before actual server start.

### Allow CORS

Specify `allow_cors_domain` field to automatically manage CORS policy to given domain or domains.

Example:

```yaml
# ...
allow_cors_domain: "https://my-domain.com"
```

### Allow OAPI

> [!NOTE]
> Any OAPI config option requires the `oapi` feature (**enabled by default**;
> re-enable it if you build with `default-features = false`):
> 
> ```toml
> [dependencies]
> impulse-server-kit = { .., features = ["oapi"] }
> ```

Specify `allow_oapi_access` field to automatically generate OpenAPI specifications and provide to users.

Example:

```yaml
# ...
allow_oapi_access: true
oapi_frontend_type: Scalar # or `SwaggerUI`
oapi_name: My API
oapi_ver: 0.1.0
oapi_api_addr: /api        # path where the OAPI frontend is served
```

When `allow_oapi_access: true`, the `oapi_name`, `oapi_ver` and
`oapi_api_addr` fields become required — startup fails with a config error if
any of them is missing.

### Security headers

When the router is built via `get_root_router_autoinject`, every response gets
a conservative set of security headers. Defaults:

| Header | Value |
| --- | --- |
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains` |
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `SAMEORIGIN` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |

`Content-Security-Policy`, `Permissions-Policy` and
`Cross-Origin-Opener-Policy` are off by default (they are highly
application-specific) — set them explicitly to opt in. Tune everything under
the `security_headers:` key; set a field to `null` to disable one header, or
`enabled: false` to skip the hoop entirely:

```yaml
security_headers:
  hsts: null                                  # e.g. for local HTTP-only development
  content_security_policy: "default-src 'self'"
```

HSTS is only sent when a TLS-bearing protocol (HTTP/3 over QUIC) is being
served — advertising `Strict-Transport-Security` from a cleartext-only setup
would pin localhost in the developer's browser for a year. Headers already
present on a response are left untouched, so per-route overrides win.

### Logging

Server Kit uses `tracing` for logging inside routes' logic. You can choose any of these log types:

- I/O logs (terminal)
- file logs
- RFC 5424 (syslog) logs
- ECS (Elastic Common Schema with disabled normalization) structured JSON logs

See [how to use logging inside your code](#logging-inside-code)

##### Log levels

There are 5 log level types available:

- `trace`
- `debug`
- `info`
- `warn`
- `error`

##### File rotation types

There are 4 log file rotation types available:

- `never`
- `daily`
- `hourly`
- `minutely`

#### I/O logs

Configuration example:

```yaml
enable_io_logs: true
io_log_level: info    # error | warn | info | debug | trace
```

#### File logs

Logs will be written in file(-s) inside `logs` folder.

Configuration example:

```yaml
enable_file_logs: true
file_log_level: info           # error | warn  | info   | debug    | trace
file_log_rotation: daily       # never | daily | hourly | minutely
file_log_max_rolling_files: 5  # by default
```

#### Syslog

Logs produced by this connector will send by one of 4 transports:

- TCP
- UDP
- Unix Socket (Datagram)
- Unix Socket Stream

You should configure `syslog_addr`. Configuration example:

```yaml
enable_syslog_logs: true
syslog_addr: "udp://127.0.0.1:514"  # schemas: `tcp://` | `udp://` | `unix://` | `ustream://`
syslog_log_level: info
```

#### ECS

ECS logs will be also written in file(-s) (folder `ecs-logs`). Configuration example:

```yaml
enable_ecs_logs: true
ecs_log_level: info       # error | warn  | info   | debug    | trace
ecs_rotation: daily       # never | daily | hourly | minutely
ecs_max_rolling_files: 5  # by default
```

### OpenTelemetry

> [!NOTE]
> Any OpenTelemetry config option requires `otel` feature to be enabled:
> 
> ```rust
> [dependencies]
> impulse-server-kit = { .., features = ["otel"] }
> ```

Server Kit supports gRPC span exporter and HTTP binary metrics exporter.

#### Span tracing

To activate span tracing, enable `otel` feature (enabled by default) and specify `otel_grpc_endpoint` field:

```yaml
otel_grpc_endpoint: http://localhost:4317  # Jaeger default gRPC write API endpoint
```

Also, you can specify log level (if none specified, goes back to `log_level` field):

```yaml
otel_log_level: info  # error | warn | info | debug | trace
```

See [how can you use spans](#otel-inside-code).

Read more about `tracing`: [`tracing` docs](https://docs.rs/tracing/latest/tracing/).

#### Metrics

To activate metrics collector, enable `otel` feature (enabled by default) and specify `otel_http_endpoint` field:

```yaml
otel_http_endpoint: http://localhost:9090/api/v1/otlp/v1/metrics  # Prometheus default write API endpoint
```

See [how can you use metrics](#otel-inside-code).

Read more about `Meter`: [`opentelemetry` docs](https://docs.rs/opentelemetry/latest/opentelemetry/metrics/struct.Meter.html).

Server Kit also provides these default metrics:

- `sk_requests` - total number of requests
- `sk_request_duration` - HTTP request duration in seconds
- `sk_active_connections` - number of active HTTP connections

These metrics are implied automatically by using `get_root_router_autoinject` function. You also can use it by hands:

```rust
Router::new()
  .hoop(impulse_server_kit::startup::sk_default_metrics)
```

<a name="8"></a>
## Crawlers: robots.txt & sitemap.xml

Both files are *built*, not kept on disk, because both have to know things a
static file does not: which routes are meant to be public, what the app has
published since it started, and — the one that trips up every hand-written
`robots.txt` — the origin it is being served on. No feature flag; the whole
module is in the prelude.

```rust
let router = get_root_router(&state)
  .push(
    RobotsTxt::new()
      .comment("Only /p/ — published documents — is meant to be crawled.")
      .disallow("/s/")
      .disallow("/api/")
      .group(RobotsGroup::for_agents(["GPTBot", "CCBot"]).disallow("/"))
      .sitemap("/sitemap.xml")   // resolved against each request's origin
      .into_router(),
  )
  .push(app_router());
```

Mount it **ahead of any catch-all**: a fallback route that answers every
unmatched path with an app shell will answer this one too, and an HTML body
where a crawler expects rules reads as no rules at all.

`RobotsTxt` and `RobotsGroup` are `Deserialize`, so the same document can come
out of the app's own YAML:

```yaml
robots:
  comment: Staging. Nothing here is meant to be found.
  groups:
    - agents: ["*"]
      disallow: ["/"]
      crawl_delay: 10
```

Rules that cannot appear in a valid file — a path starting with neither `/` nor
`*`, anything carrying a `#` (which would comment out the rest of its own line,
turning `Disallow: /private#draft` into a ban on `/private` and a licence for
everything the author thought they had closed off) — are dropped with a warning
when the handler is built.

### Sitemaps

A fixed list mounts directly; a list that changes is written per request, since
`Sitemap` is a salvo `Writer`:

```rust
#[handler]
async fn sitemap(depot: &mut Depot, req: &mut Request) -> MResult<Sitemap> {
  let origin = request_origin(req);      // scheme://host, honouring X-Forwarded-*
  let mut map = Sitemap::new();
  for doc in published(depot).await? {
    map.push(SitemapUrl::new(format!("{origin}/p/{}", doc.slug)).lastmod(doc.updated_at.to_rfc3339()));
  }
  Ok(map)
}

let router = router.push(Router::with_path(SITEMAP_XML_PATH).get(sitemap));
```

Every `<loc>` must be absolute — hence `request_origin`, which reads
`X-Forwarded-Proto`/`X-Forwarded-Host` before falling back to `Host`, because
behind a TLS-terminating proxy the connection this server accepted is plain
HTTP and believing it publishes `http://` URLs for an `https://` site. One file
carries at most `MAX_SITEMAP_URLS` (50 000) entries; past that crawlers reject
it whole, so split across several `Sitemap:` lines. `lastmod` is the one field
they act on — a file where everything changed today teaches them to ignore it —
and `changefreq`/`priority` are advisory (Google ignores both).

### Several hostnames, one site

A proxy that points a product domain and a vanity domain at the same socket
gives you an application serving every page at two addresses — which a crawler
reads as two copies of one article, picking a winner itself and splitting the
signals. Nothing in the request tells the app which host it should be *found*
under, so that part is configuration:

```rust
let canonical = CanonicalOrigin::from_env("MY_APP_CANONICAL_ORIGIN");   // unset => follow the request

let robots = RobotsTxt::new()
  .disallow("/private/")
  .sitemap("/sitemap.xml")
  .canonical_origin(canonical.clone());
```

`canonical.resolve(req)` then returns the same origin whatever host asked, and
every published URL should be built from it: the page's
`<link rel="canonical">`, the sitemap's `<loc>`s, and the `Sitemap:` line —
which `RobotsTxt` emits **only** on the canonical host, because a sitemap
listing another host's URLs is cross-submission and is ignored unless the owner
has verified both.

The other hosts keep the same crawl rules on purpose. They serve the same pages,
and the way a crawler learns that two addresses are one page is by fetching both
and finding the same canonical; `Disallow: /` would leave it unable to see that
and free to list the alias as a bare URL anyway. Redirecting the alias at the
proxy is the other valid answer, and the better one when nobody is meant to read
the site under that name.

Worth setting even on one domain: it pins the *scheme*. A proxy that terminates
TLS without adding `X-Forwarded-Proto` leaves the server no way to know it is an
HTTPS site, and `request_origin` then honestly reports `http://`.

### Keeping something *out* of an index

`robots.txt` and `noindex` are not interchangeable, and most sites want both:

| | stops the fetch | binds a crawler that fetched anyway |
| --- | --- | --- |
| `robots.txt` `Disallow` | yes | no — it never saw the page |
| `X-Robots-Tag` / `<meta name="robots">` | no | yes |

```rust
set_x_robots_tag(res, RobotsTag::NOINDEX_NOFOLLOW);
```

Only `Disallow` prevents the request from happening at all, which matters
whenever *being fetched* is itself the damage — a single-use link that a
crawler spends before its recipient arrives, an expensive export endpoint. Only
the header binds a crawler that ignored `robots.txt` or was handed the URL
directly. Pairing them has one documented cost: a disallowed URL that something
links to can still be listed bare, without a snippet, precisely because the
crawler never fetched it and so never saw the `noindex`.

`RobotsTag` exists so that a page's header and its `<meta name="robots">` can be
given the same constant — they must agree, and the way they stop agreeing is
somebody spelling one of them out again by hand.

<a name="9"></a>
## Leptos SSR & SEO

> [!NOTE]
> Any Leptos SSR config option requires the `leptos-ssr` feature to be enabled:
>
> ```toml
> [dependencies]
> impulse-server-kit = { .., features = ["leptos-ssr"] }
> ```

The Leptos SSR adapter renders a Leptos `App` to streaming HTML and serves the
front-end bundle alongside it. The HTML prefix is built before the application
view, so SEO/social tags are present in the very first bytes the crawler reads
— no JavaScript required. Tags that an app declares with `leptos_meta`
(`<Title>`, `<Meta>`, `<Link>`) are spliced into the same `<head>` after the
defaults, so per-page overrides still work.

### Bundle wiring

```yaml
frontend_dist_path: ./dist      # falls back to env IMPULSE_FRONTEND_DIST,
                                # then ./dist, then /usr/local/frontend-dist
leptos_output_name: my_app      # matches cargo-leptos `output-name`
leptos_server_fn_prefix: /api/leptos   # optional; defaults to /api/leptos
```

| Field | Effect |
| --- | --- |
| `frontend_dist_path` | Directory that holds the wasm/JS/CSS bundle and any static assets. Every file under it is served at the URL mirroring its on-disk path; unknown paths fall through to the SSR renderer. |
| `leptos_output_name` | Used to build URLs for `/pkg/<name>.js`, `/pkg/<name>.css`, `/pkg/<name>_bg.wasm`. Defaults to `app_name` when unset. |
| `leptos_server_fn_prefix` | Route prefix where `#[server]` functions are mounted (e.g. `/api/leptos`). Must match the value the client expects. |

### SEO defaults

All SEO fields live under the `leptos_seo` key. Each field is optional — the
prefix only emits a tag when the corresponding value is set, so an empty
`leptos_seo:` block is a no-op.

```yaml
leptos_seo:
  title_template: "{} · My App"
  default_title: "My App — short tagline"
  description: "One-paragraph elevator pitch for crawlers and social cards."
  og_image: "https://my-app.example.com/og-preview.png"
  og_logo: "https://my-app.example.com/logo-512.png"
  canonical_base: "https://my-app.example.com"
  twitter_handle: "@my_app"
  robots: "index,follow"
  locale: "en"
  site_name: "My App"
```

What ends up in the rendered `<head>` (in the order the prefix emits it):

| Field | Tag(s) emitted | Notes |
| --- | --- | --- |
| `description` | `<meta name="description">`, `<meta property="og:description">` | Same string is reused for the OG description so social previews match the page summary. |
| `robots` | `<meta name="robots">` | Set to `noindex,nofollow` on staging. |
| `canonical_base` | `<link rel="canonical">`, `<meta property="og:url">` | The current request path is appended to the base, so every URL gets its own canonical. Trailing slashes on the base are trimmed. |
| `twitter_handle` | `<meta name="twitter:site">` | The leading `@` is conventional but not required by Twitter. |
| `og_image` | `<meta property="og:image">`, `<meta name="twitter:image">` | Use an absolute URL — relative paths break for crawlers that don't resolve against the canonical base. |
| `og_logo` | `<meta property="og:logo">` | Not part of the original OG spec, but consumed by some SEO validators and rich-result scrapers. Typically the square brand mark, distinct from `og_image`. |
| `default_title` | `<meta property="og:title">` | Page-level `<Title>` from `leptos_meta` still overrides the actual `<title>` element. |
| `site_name` (falls back to `default_title`) | `<meta property="og:site_name">` | The brand name; usually shorter than the title (e.g. `"My App"` vs `"My App — short tagline"`). |
| `locale` | `<html lang="…">`, `<meta property="og:locale">` | OG locales are normalised to the `lang_REGION` form: `ru` → `ru_RU`, `en` → `en_US`, `pt-BR` → `pt_BR`. Unknown short codes pass through unchanged. |
| `title_template` | _Reserved for future_ | The struct already accepts it so YAML written today is forward-compatible. |

`<meta charset="UTF-8">`, `<meta name="viewport">`, `<meta property="og:type" content="website">` and `<meta name="twitter:card" content="summary_large_image">` are always emitted regardless of `leptos_seo` settings.

### Canonical URL & duplicate `<head>` tags

The HTML prefix already emits `<meta name="description">` and
`<link rel="canonical">` when `leptos_seo.description` /
`leptos_seo.canonical_base` are set. **Do not also declare them inside the
Leptos `App`** — that creates two tags and SEO scanners flag it. The pattern
is:

- Cross-cutting defaults (description, canonical, OG, Twitter, robots,
  locale) go in `server.yaml`.
- Per-page overrides go in the Leptos view as `<Title>`, `<Meta>` and
  `<Link>` from `leptos_meta`. These render *after* the defaults, so they
  appear later in the `<head>` and win the document `<title>`.

### Index-page redirect

The SSR handler returns `301 Moved Permanently` from `/index.html`,
`/index.htm` and `/index.php` to `/`. This avoids duplicate-content reports
from SEO crawlers when the landing page would otherwise be reachable under
two URLs. If a real `dist/index.html` is on disk (rare in SSR setups), the
static-asset router serves it before the redirect runs.

### Streaming mode

```rust
let mut opts = LeptosOptions::from_generic_values(setup.generic_values());
opts.stream_mode = SsrStreamMode::InOrder;   // or OutOfOrder
```

- `InOrder` (default): predictable, no client-side fragment swapping;
  `<Suspense>` boundaries block streaming until they resolve.
- `OutOfOrder`: faster first-meaningful-paint; suspense placeholders flush
  immediately and are replaced via injected `<template>`/`<script>` chunks
  once resources resolve.

### Theme cookie

The SSR handler reads a `theme` cookie and applies the value to
`<html class="…">` so server-rendered markup matches the user's last
preference (light/dark). The cookie is also injected as an
`InitialTheme(String)` Leptos context for app-level use.

[ACME]: https://en.wikipedia.org/wiki/Automatic_Certificate_Management_Environment
