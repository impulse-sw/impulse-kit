# Impulse Server Kit

State-of-art simple and powerful web server based on [Salvo](https://github.com/salvo-rs/salvo). Provides extended tracing, configuration-over-YAML, HTTP3, TLS v1.3, MessagePack + SIMD JSON ser/de support, ACME, OpenAPI and OpenTelemetry features *by default*.

Table of contents:

- [How's it work](#1)
- [Using Server Kit](#2)
- [Extended utilities](#3)
- [4 Quick start steps](#4)
- [Common Salvo documentation](#5)
- [Code API Overview](#6)
- [Configuration Overview](#7)

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
impulse-server-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "0.11" }
```

And create empty `{app-name}.yaml` to fill later (see [Configuration Overview](#7) below).

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
startup_type: http_localhost
server_port: 8801
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
edition = "2021"

[dependencies]
impulse-server-kit = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "0.11" }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["macros"] }
tracing = "1"
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

### Force HTTPS

To enforce HTTPS, you should start another server via `start_force_https_redirect` function:

```rust
let (server, handler) = start_force_https_redirect(80, 443).await.unwrap();
```

<a name="7"></a>
## Configuration Overview

> [!NOTE]
> To setup these features, you have no need to edit code, just `{your-app}.yaml`.

### Startup types

There are several startup types:

1. `http_localhost` - will listen `http://127.0.0.1:{port}` only
2. `unsafe_http` - will listen `http://0.0.0.0:{port}`
3. `https_acme` (requires `acme` feature) - will listen `https://{host}:{port}` with [ACME] support
4. `quinn_acme` (requires both `acme` and `http3` features) - will listen `https://` and `quic://` with [ACME]
5. `https_only` - will listen `https://{host}:{port}`
6. `quinn` (requires `http3` feature) - will listen `https://` and `quic://`
7. `quinn_only` (requires `http3` feature) - will listen `quic://{host}:{port}`

Any HTTPS connection will use TLS v1.3 only.

Example:

```yaml
startup_type: quinn
```

#### ACME domain

Specify `acme_domain` to use [ACME] (TLS ALPN-01).

Example:

```yaml
startup_type: quinn_acme
server_host: 0.0.0.0
server_port: 443
acme_domain: tls-alpn-01.domain.com
```

#### Server host & server port

Specify `server_host` as IP address to listen with server (except `http_localhost` and `unsafe_http` startup types).

Specify `server_port` to listen with server. If you use your app with Server Kit as internal service, specify any port; if you want to expose your ports to the Internet, use `80` to HTTP and `443` for HTTPS or QUIC.

Also, if you want to specify your listening port after application start, you can use `server_port_achiever` field (see below).

Example:

```yaml
startup_type: quinn
server_host: 0.0.0.0
server_port: 443
```

#### SSL key & certs

Example:

```yaml
startup_type: quinn
server_host: 0.0.0.0
server_port: 443
ssl_crt_path: certs/fullchain.pem
ssl_key_path: certs/privkey.pem
```

#### Server port achieveing

You can specify `server_port_achiever` field to any filepath to make server wait for file creation and writing actual server port to listen to it.

Example:

```yaml
startup_type: quinn
server_host: 0.0.0.0
server_port_achiever: write/port/to/me.txt
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
> Any OAPI config option requires `oapi` feature to be enabled:
> 
> ```rust
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
```

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
  .hoop(cc_server_kit::startup::sk_default_metrics)
```

[ACME]: https://en.wikipedia.org/wiki/Automatic_Certificate_Management_Environment
