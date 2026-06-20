//! `impulse-client-ring` — a [`reqwest`](https://docs.rs/reqwest)-style HTTP
//! client that speaks over the **Ring** shared-memory IPC bus instead of
//! TCP/Unix sockets.
//!
//! It is the client counterpart to `impulse-server-kit`'s `ImpulseRingListener`:
//! a server registers an application on the bus and serves HTTP over shared
//! memory; this client looks the application up *by name* and issues ordinary
//! HTTP requests against it — no ports, no kernel round-trips on the data path.
//!
//! ```no_run
//! use impulse_client_ring::ImpulseRingClient;
//!
//! # fn main() -> std::io::Result<()> {
//! // `impulsed` (the Ring broker) must be running, and a server must have
//! // registered the application name `"hello-service"`.
//! let client = ImpulseRingClient::connect("hello-service")?;
//!
//! let resp = client.get("/hello").send_blocking()?;
//! println!("{} {}", resp.status(), resp.text()?);
//! # Ok(())
//! # }
//! ```
//!
//! Every request becomes a single Avro-framed RPC carrying a
//! [`RingHttpRequest`](impulse_ring_http::RingHttpRequest); the response is a
//! [`RingHttpResponse`](impulse_ring_http::RingHttpResponse). The wire schemas
//! are shared with the server through the [`impulse_ring_http`] crate, so the
//! broker's fingerprint check guarantees both ends agree.

#![deny(warnings, clippy::todo, clippy::unimplemented)]

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use impulse_ring_connector::Connection;
use impulse_ring_http::{REQUEST_SCHEMA, RESPONSE_SCHEMA, RingHeader, RingHttpRequest, RingHttpResponse, http_fn_name};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Default per-request timeout if the caller does not set one.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

static CLIENT_SEQ: AtomicU64 = AtomicU64::new(0);

/// A connected client addressing a single Ring HTTP application by name.
///
/// Cloning is cheap: clones share the underlying bus connection.
#[derive(Clone)]
pub struct ImpulseRingClient {
  conn: Arc<Connection>,
  app_name: String,
  fn_name: String,
  key: Option<String>,
  timeout: Duration,
}

impl ImpulseRingClient {
  /// Connect to the broker and target the application registered as `target_app`.
  ///
  /// A unique local client name is generated automatically. Requires the
  /// `impulsed` broker to be running.
  pub fn connect(target_app: &str) -> io::Result<Self> {
    let seq = CLIENT_SEQ.fetch_add(1, Ordering::Relaxed);
    let client_name = format!("ring-http-client-{}-{seq}", std::process::id());
    Self::connect_as(&client_name, target_app)
  }

  /// Connect under an explicit local client name.
  pub fn connect_as(client_name: &str, target_app: &str) -> io::Result<Self> {
    let conn = Connection::connect(client_name)?;
    Ok(Self::with_connection(Arc::new(conn), target_app))
  }

  /// Build a client from an existing [`Connection`], targeting `target_app`.
  ///
  /// Useful when a single process talks to several Ring applications and wants
  /// to share one bus connection.
  pub fn with_connection(conn: Arc<Connection>, target_app: &str) -> Self {
    ImpulseRingClient {
      conn,
      app_name: target_app.to_string(),
      fn_name: http_fn_name(target_app),
      key: None,
      timeout: DEFAULT_TIMEOUT,
    }
  }

  /// Set the access key required by the server's exposed function.
  #[must_use]
  pub fn with_key(mut self, key: impl Into<String>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Set the default per-request timeout.
  #[must_use]
  pub fn with_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  /// The target application name.
  pub fn app_name(&self) -> &str {
    &self.app_name
  }

  /// Start building a request with an arbitrary method.
  pub fn request(&self, method: Method, uri: impl Into<String>) -> RequestBuilder<'_> {
    RequestBuilder {
      client: self,
      method: method.as_str().to_string(),
      uri: uri.into(),
      headers: Vec::new(),
      body: Vec::new(),
      timeout: self.timeout,
    }
  }

  /// Start a `GET` request.
  pub fn get(&self, uri: impl Into<String>) -> RequestBuilder<'_> {
    self.request(Method::GET, uri)
  }
  /// Start a `POST` request.
  pub fn post(&self, uri: impl Into<String>) -> RequestBuilder<'_> {
    self.request(Method::POST, uri)
  }
  /// Start a `PUT` request.
  pub fn put(&self, uri: impl Into<String>) -> RequestBuilder<'_> {
    self.request(Method::PUT, uri)
  }
  /// Start a `PATCH` request.
  pub fn patch(&self, uri: impl Into<String>) -> RequestBuilder<'_> {
    self.request(Method::PATCH, uri)
  }
  /// Start a `DELETE` request.
  pub fn delete(&self, uri: impl Into<String>) -> RequestBuilder<'_> {
    self.request(Method::DELETE, uri)
  }
  /// Start a `HEAD` request.
  pub fn head(&self, uri: impl Into<String>) -> RequestBuilder<'_> {
    self.request(Method::HEAD, uri)
  }

  /// Issue a raw [`RingHttpRequest`] and block for the [`RingHttpResponse`].
  pub fn call_blocking(&self, req: RingHttpRequest, timeout: Duration) -> io::Result<RingHttpResponse> {
    self.conn.call_blocking::<RingHttpRequest, RingHttpResponse>(
      &self.fn_name,
      self.key.as_deref(),
      &req,
      REQUEST_SCHEMA,
      RESPONSE_SCHEMA,
      timeout,
    )
  }
}

/// A request under construction. Finished with [`RequestBuilder::send_blocking`]
/// (or [`RequestBuilder::send`] with the `async` feature).
pub struct RequestBuilder<'a> {
  client: &'a ImpulseRingClient,
  method: String,
  uri: String,
  headers: Vec<RingHeader>,
  body: Vec<u8>,
  timeout: Duration,
}

impl RequestBuilder<'_> {
  /// Append a header. May be called multiple times for repeated headers.
  #[must_use]
  pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
    self.headers.push(RingHeader::new(name, value));
    self
  }

  /// Append every header from an iterator of name/value pairs.
  #[must_use]
  pub fn headers<I, N, V>(mut self, headers: I) -> Self
  where
    I: IntoIterator<Item = (N, V)>,
    N: Into<String>,
    V: Into<String>,
  {
    for (n, v) in headers {
      self.headers.push(RingHeader::new(n, v));
    }
    self
  }

  /// Set the raw request body.
  #[must_use]
  pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
    self.body = body.into();
    self
  }

  /// Serialize `value` as JSON, set it as the body and add a
  /// `Content-Type: application/json` header.
  pub fn json<T: Serialize>(mut self, value: &T) -> io::Result<Self> {
    let body = serde_json::to_vec(value).map_err(io::Error::other)?;
    self.body = body;
    self.headers.push(RingHeader::new("content-type", "application/json"));
    Ok(self)
  }

  /// Override the per-request timeout.
  #[must_use]
  pub fn timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  /// Assemble the [`RingHttpRequest`] this builder describes.
  pub fn build(&self) -> RingHttpRequest {
    RingHttpRequest {
      method: self.method.clone(),
      uri: self.uri.clone(),
      headers: self.headers.clone(),
      body: self.body.clone(),
    }
  }

  /// Send the request, blocking the current thread until the response arrives.
  pub fn send_blocking(self) -> io::Result<RingResponse> {
    let req = self.build();
    let raw = self.client.call_blocking(req, self.timeout)?;
    RingResponse::from_wire(raw)
  }

  /// Send the request without blocking the async executor.
  ///
  /// The blocking bus call runs on a dedicated Tokio blocking thread.
  #[cfg(feature = "async")]
  pub async fn send(self) -> io::Result<RingResponse> {
    let req = self.build();
    let timeout = self.timeout;
    let client = self.client.clone();
    let raw = tokio::task::spawn_blocking(move || client.call_blocking(req, timeout))
      .await
      .map_err(io::Error::other)??;
    RingResponse::from_wire(raw)
  }
}

/// A response returned by a Ring HTTP server.
pub struct RingResponse {
  status: StatusCode,
  headers: HeaderMap,
  body: Vec<u8>,
}

impl RingResponse {
  fn from_wire(raw: RingHttpResponse) -> io::Result<Self> {
    let status = u16::try_from(raw.status)
      .ok()
      .and_then(|s| StatusCode::from_u16(s).ok())
      .ok_or_else(|| io::Error::other(format!("server returned invalid status code {}", raw.status)))?;

    let mut headers = HeaderMap::with_capacity(raw.headers.len());
    for h in &raw.headers {
      let name = HeaderName::from_bytes(h.name.as_bytes()).map_err(io::Error::other)?;
      let value = HeaderValue::from_str(&h.value).map_err(io::Error::other)?;
      headers.append(name, value);
    }

    Ok(RingResponse {
      status,
      headers,
      body: raw.body,
    })
  }

  /// The HTTP status code.
  pub fn status(&self) -> StatusCode {
    self.status
  }

  /// `true` for 2xx status codes.
  pub fn is_success(&self) -> bool {
    self.status.is_success()
  }

  /// The response headers.
  pub fn headers(&self) -> &HeaderMap {
    &self.headers
  }

  /// Borrow the raw response body.
  pub fn body(&self) -> &[u8] {
    &self.body
  }

  /// Consume the response and return the raw body bytes.
  pub fn bytes(self) -> Vec<u8> {
    self.body
  }

  /// Decode the body as UTF-8 text.
  pub fn text(self) -> io::Result<String> {
    String::from_utf8(self.body).map_err(io::Error::other)
  }

  /// Deserialize the body as JSON.
  pub fn json<T: DeserializeOwned>(&self) -> io::Result<T> {
    serde_json::from_slice(&self.body).map_err(io::Error::other)
  }

  /// Return an error if the status code is not a success (2xx).
  pub fn error_for_status(self) -> io::Result<Self> {
    if self.status.is_success() {
      Ok(self)
    } else {
      Err(io::Error::other(format!("server returned status {}", self.status)))
    }
  }
}
