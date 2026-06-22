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

#[cfg(feature = "async")]
pub mod streaming;

/// Re-export of the HTTP-over-Ring wire protocol, so consumers (e.g. the LBRP
/// `impring://` connector) can read the upgrade headers / kinds without taking a
/// direct dependency on `impulse-ring-http`.
pub use impulse_ring_http;

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use impulse_ring_connector::{Connection, Subscriber};
use impulse_ring_http::{
  HEADER_BODY_CHANNEL, REQUEST_SCHEMA, RESPONSE_SCHEMA, RingHeader, RingHttpRequest, RingHttpResponse, RingStreamFrame,
  STREAM_SCHEMA, http_fn_name, opcode,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// How long to wait for the next body chunk before declaring the stream stalled.
const BODY_CHUNK_TIMEOUT: Duration = Duration::from_secs(10);

/// How long to wait for a freshly published body channel to appear on the broker.
const BODY_CHANNEL_WAIT: Duration = Duration::from_secs(5);

/// Default per-request timeout if the caller does not set one.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

static CLIENT_SEQ: AtomicU64 = AtomicU64::new(0);
static SESSION_SEQ: AtomicU64 = AtomicU64::new(0);

/// Mint a process-unique session id for a streaming upgrade.
#[cfg(feature = "async")]
fn next_session_id() -> u64 {
  let seq = SESSION_SEQ.fetch_add(1, Ordering::Relaxed);
  let nanos = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_nanos() as u64)
    .unwrap_or(0);
  (nanos << 16) ^ ((std::process::id() as u64) << 8) ^ seq
}

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
  ///
  /// If the server returned the body out-of-band over a channel (because it was
  /// too large to fit the reply ring), the chunks are reassembled here, so the
  /// returned response always carries a complete inline body.
  pub fn call_blocking(&self, req: RingHttpRequest, timeout: Duration) -> io::Result<RingHttpResponse> {
    let mut resp = self.conn.call_blocking::<RingHttpRequest, RingHttpResponse>(
      &self.fn_name,
      self.key.as_deref(),
      &req,
      REQUEST_SCHEMA,
      RESPONSE_SCHEMA,
      timeout,
    )?;
    self.reassemble_chunked_body(&mut resp)?;
    Ok(resp)
  }

  /// If `resp` was delivered with a chunked body (see [`HEADER_BODY_CHANNEL`]),
  /// subscribe to the named channel, reassemble the body and strip the marker
  /// header — leaving an ordinary inline response. A no-op otherwise.
  fn reassemble_chunked_body(&self, resp: &mut RingHttpResponse) -> io::Result<()> {
    let Some(chan) = header_value(&resp.headers, HEADER_BODY_CHANNEL).map(str::to_owned) else {
      return Ok(());
    };
    let sub = subscribe_body_channel(&self.conn, &chan, self.key.as_deref())?;
    let mut body = Vec::new();
    loop {
      match sub.recv::<RingStreamFrame>(BODY_CHUNK_TIMEOUT)? {
        Some(frame) => match frame.opcode {
          opcode::DATA | opcode::STREAM_DATA => body.extend_from_slice(&frame.payload),
          opcode::CLOSE | opcode::STREAM_CLOSE => break,
          _ => {}
        },
        None => return Err(io::Error::new(io::ErrorKind::TimedOut, "ring body stream stalled")),
      }
    }
    resp.headers.retain(|h| !h.name.eq_ignore_ascii_case(HEADER_BODY_CHANNEL));
    resp.body = body;
    Ok(())
  }

  /// The underlying bus connection (shared; cloneable).
  ///
  /// Exposed so the streaming layer (and the LBRP connector) can publish and
  /// subscribe to the channels that carry SSE/WebSocket/WebTransport data.
  pub fn connection(&self) -> &Arc<Connection> {
    &self.conn
  }

  /// Open a Server-Sent Events stream against `uri`.
  ///
  /// Sends an ordinary RPC handshake carrying `Accept: text/event-stream`; the
  /// listener answers with the upgrade headers (naming a Ring channel) and
  /// streams the response body onto that channel. The returned
  /// [`streaming::RingEventStream`] yields the raw event bytes.
  #[cfg(feature = "async")]
  pub async fn sse(&self, uri: impl Into<String>) -> io::Result<streaming::RingEventStream> {
    let this = self.clone();
    let uri = uri.into();
    tokio::task::spawn_blocking(move || this.open_sse_blocking(&uri))
      .await
      .map_err(io::Error::other)?
  }

  /// Blocking variant of [`ImpulseRingClient::sse`].
  #[cfg(feature = "async")]
  pub fn open_sse_blocking(&self, uri: &str) -> io::Result<streaming::RingEventStream> {
    use impulse_ring_http::{HEADER_CHAN_DOWN, HEADER_UPGRADE, RingUpgradeKind};

    let req = RingHttpRequest {
      method: "GET".to_string(),
      uri: uri.to_string(),
      headers: vec![RingHeader::new("accept", "text/event-stream")],
      body: Vec::new(),
    };
    let resp = self.call_blocking(req, self.timeout)?;

    match header_value(&resp.headers, HEADER_UPGRADE).and_then(RingUpgradeKind::parse) {
      Some(RingUpgradeKind::Sse) => {}
      _ => {
        return Err(io::Error::other("server did not upgrade the request to an SSE stream"));
      }
    }
    let down = header_value(&resp.headers, HEADER_CHAN_DOWN)
      .ok_or_else(|| io::Error::other("SSE upgrade is missing the down-channel header"))?;
    let subscriber = streaming::subscribe_by_name(&self.conn, down, self.key.as_deref())?;
    Ok(streaming::RingEventStream::new(self.conn.clone(), subscriber))
  }

  /// Open a WebSocket virtual connection against `uri`.
  ///
  /// Publishes the client→server channel, hands the listener its name via an
  /// `Upgrade: websocket` handshake, then returns a [`streaming::RingDuplex`]
  /// virtual socket. The caller drives a normal WebSocket client codec over it
  /// (salvo terminates the upgrade on the server side — Ring only relays bytes).
  #[cfg(feature = "async")]
  pub async fn websocket(&self, uri: impl Into<String>) -> io::Result<streaming::RingDuplex> {
    let this = self.clone();
    let uri = uri.into();
    tokio::task::spawn_blocking(move || this.open_websocket_blocking(&uri))
      .await
      .map_err(io::Error::other)?
  }

  /// Blocking variant of [`ImpulseRingClient::websocket`].
  #[cfg(feature = "async")]
  pub fn open_websocket_blocking(&self, uri: &str) -> io::Result<streaming::RingDuplex> {
    use impulse_ring_http::{
      HEADER_CHAN_DOWN, HEADER_CHAN_UP, HEADER_SESSION, HEADER_UPGRADE, RingUpgradeKind, stream_channel_name,
    };

    let session = next_session_id();
    let up_name = stream_channel_name(&self.app_name, session, "up");
    // Publish our outbound channel before the listener tries to subscribe to it.
    let up_pub = streaming::publish_stream_channel(&self.conn, &up_name, self.key.as_deref())?;

    let req = RingHttpRequest {
      method: "GET".to_string(),
      uri: uri.to_string(),
      headers: vec![
        RingHeader::new("connection", "Upgrade"),
        RingHeader::new("upgrade", "websocket"),
        RingHeader::new(HEADER_UPGRADE, RingUpgradeKind::WebSocket.as_str()),
        RingHeader::new(HEADER_CHAN_UP, up_name.clone()),
        RingHeader::new(HEADER_SESSION, format!("{session:016x}")),
      ],
      body: Vec::new(),
    };
    let resp = self.call_blocking(req, self.timeout)?;

    match header_value(&resp.headers, HEADER_UPGRADE).and_then(RingUpgradeKind::parse) {
      Some(RingUpgradeKind::WebSocket) => {}
      _ => return Err(io::Error::other("server did not accept the WebSocket upgrade")),
    }
    let down = header_value(&resp.headers, HEADER_CHAN_DOWN)
      .ok_or_else(|| io::Error::other("WebSocket upgrade is missing the down-channel header"))?;
    let down_sub = streaming::subscribe_by_name(&self.conn, down, self.key.as_deref())?;
    Ok(streaming::RingDuplex::new(self.conn.clone(), up_pub, down_sub))
  }

  /// Open a WebTransport session against `uri`.
  ///
  /// Establishes the session channel pair via an `Upgrade: webtransport`
  /// handshake and returns a [`streaming::RingWebTransport`] supporting datagrams
  /// and bidirectional streams. (Real WebTransport-over-HTTP/3 is terminated by
  /// salvo at the edge, e.g. LBRP; this is the Ring-side session.)
  #[cfg(feature = "async")]
  pub async fn webtransport(&self, uri: impl Into<String>) -> io::Result<streaming::RingWebTransport> {
    let this = self.clone();
    let uri = uri.into();
    tokio::task::spawn_blocking(move || this.open_webtransport_blocking(&uri))
      .await
      .map_err(io::Error::other)?
  }

  /// Blocking variant of [`ImpulseRingClient::webtransport`].
  #[cfg(feature = "async")]
  pub fn open_webtransport_blocking(&self, uri: &str) -> io::Result<streaming::RingWebTransport> {
    use impulse_ring_http::{
      HEADER_CHAN_DOWN, HEADER_CHAN_UP, HEADER_SESSION, HEADER_UPGRADE, RingUpgradeKind, stream_channel_name,
    };

    let session = next_session_id();
    let up_name = stream_channel_name(&self.app_name, session, "up");
    let up_pub = streaming::publish_stream_channel(&self.conn, &up_name, self.key.as_deref())?;

    let req = RingHttpRequest {
      method: "CONNECT".to_string(),
      uri: uri.to_string(),
      headers: vec![
        RingHeader::new("connection", "Upgrade"),
        RingHeader::new("upgrade", "webtransport"),
        RingHeader::new(HEADER_UPGRADE, RingUpgradeKind::WebTransport.as_str()),
        RingHeader::new(HEADER_CHAN_UP, up_name.clone()),
        RingHeader::new(HEADER_SESSION, format!("{session:016x}")),
      ],
      body: Vec::new(),
    };
    let resp = self.call_blocking(req, self.timeout)?;

    match header_value(&resp.headers, HEADER_UPGRADE).and_then(RingUpgradeKind::parse) {
      Some(RingUpgradeKind::WebTransport) => {}
      _ => return Err(io::Error::other("server did not accept the WebTransport session")),
    }
    let down = header_value(&resp.headers, HEADER_CHAN_DOWN)
      .ok_or_else(|| io::Error::other("WebTransport upgrade is missing the down-channel header"))?;
    let down_sub = streaming::subscribe_by_name(&self.conn, down, self.key.as_deref())?;
    Ok(streaming::RingWebTransport::new(
      self.conn.clone(),
      up_pub,
      down_sub,
      true, // client is the initiator (even stream ids)
    ))
  }
}

/// Resolve a freshly published body channel by name and subscribe to it.
///
/// The publisher (server) registers the channel just before answering the RPC,
/// so this retries briefly while it propagates to the broker. Sync sibling of
/// `streaming::subscribe_by_name`, usable without the `async` feature.
fn subscribe_body_channel(conn: &Connection, name: &str, key: Option<&str>) -> io::Result<Subscriber> {
  let deadline = std::time::Instant::now() + BODY_CHANNEL_WAIT;
  loop {
    for ci in conn.list_channels()? {
      if ci.name == name {
        return conn.subscribe(ci.channel_id, key, STREAM_SCHEMA);
      }
    }
    if std::time::Instant::now() >= deadline {
      return Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("ring body channel '{name}' did not appear"),
      ));
    }
    std::thread::sleep(Duration::from_millis(25));
  }
}

/// Case-insensitive lookup of a header value in a wire header list.
fn header_value<'a>(headers: &'a [RingHeader], name: &str) -> Option<&'a str> {
  headers
    .iter()
    .find(|h| h.name.eq_ignore_ascii_case(name))
    .map(|h| h.value.as_str())
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

  /// Serialize `value` as MessagePack, set it as the body and add a
  /// `Content-Type: application/msgpack` header.
  ///
  /// MsgPack is impulse-kit's first-class wire format (see `impulse-utils`'
  /// `MsgPackRequest`/`MsgPackResponse`); this is the Ring-transport counterpart.
  pub fn msgpack<T: Serialize>(mut self, value: &T) -> io::Result<Self> {
    let body = rmp_serde::to_vec(value).map_err(io::Error::other)?;
    self.body = body;
    self
      .headers
      .push(RingHeader::new("content-type", "application/msgpack"));
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

  /// Deserialize the body as MessagePack.
  pub fn msgpack<T: DeserializeOwned>(&self) -> io::Result<T> {
    rmp_serde::from_slice(&self.body).map_err(io::Error::other)
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

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Serialize, serde::Deserialize, PartialEq, Debug)]
  struct Hello {
    text: String,
    n: u32,
  }

  #[test]
  fn msgpack_round_trips_through_the_wire_body() {
    // A request body encoded as msgpack must decode back from a response that
    // simply echoes the bytes — mirrors the JSON path but with `rmp_serde`.
    let value = Hello {
      text: "hi".into(),
      n: 7,
    };
    let body = rmp_serde::to_vec(&value).unwrap();

    let resp = RingResponse::from_wire(RingHttpResponse {
      status: 200,
      headers: vec![RingHeader::new("content-type", "application/msgpack")],
      body,
    })
    .unwrap();

    let decoded: Hello = resp.msgpack().unwrap();
    assert_eq!(decoded, value);
  }
}
