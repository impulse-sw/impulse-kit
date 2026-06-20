//! Listening for HTTP over the **Ring** shared-memory IPC bus.
//!
//! [`ImpulseRingListener`] is the shared-memory analogue of salvo's
//! `TcpListener`: instead of binding a socket, it registers an application on
//! the Ring bus (served by the `impulsed` broker) and exposes a single function
//! that carries HTTP requests and responses. Clients — typically
//! [`impulse-client-ring`](https://docs.rs/impulse-client-ring) — address the
//! server purely by its application name.
//!
//! Unlike a TCP listener, Ring is not a byte stream, so there is no HTTP codec
//! on the wire: each request is one Avro-framed RPC. The full salvo routing,
//! middleware and catcher pipeline still runs for every request via
//! [`Service::hyper_handler`](salvo::Service::hyper_handler), so handlers behave
//! exactly as they do over TCP.
//!
//! You normally do not construct this type directly — list an `impulse-ring`
//! entry under `protocols:` in the YAML config and let [`crate::startup::start`]
//! wire it up. The lower-level [`serve_impulse_ring`] entry point is available
//! when you build the server by hand.
//!
//! ```yaml
//! protocols:
//!   - type: impulse-ring
//!     app_name: my-service
//! ```

use std::io;
use std::sync::{Arc, Weak};

use http_body_util::BodyExt;
use impulse_ring_connector::Connection;
use impulse_ring_http::{
  HEADER_CHAN_DOWN, HEADER_UPGRADE, REQUEST_SCHEMA, RESPONSE_SCHEMA, RingHeader, RingHttpRequest, RingHttpResponse,
  RingStreamFrame, RingUpgradeKind, STREAM_SCHEMA, http_fn_name, stream_channel_name,
};
use salvo::Service;
use salvo::conn::SocketAddr;
use salvo::http::uri::Scheme;
use salvo::http::{Request, Response, StatusCode};
use tokio::runtime::Handle;

use impulse_utils::errors::ServerError;
use impulse_utils::prelude::MResult;

/// A WebTransport-over-Ring handler: invoked with a freshly accepted session.
///
/// Because salvo's native WebTransport needs a QUIC connection (unavailable over
/// Ring), an impring service handles WebTransport through this Ring-native
/// session API instead. The real WebTransport-over-HTTP/3 edge (e.g. LBRP)
/// terminates the protocol and relays the session over Ring.
pub type RingWebTransportHandler = Arc<
  dyn Fn(impulse_client_ring::streaming::RingWebTransport) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
    + Send
    + Sync,
>;

/// A handle describing a Ring HTTP application to serve.
///
/// Construct it with the application name that clients will use to reach the
/// server, then hand it to [`serve_impulse_ring`] together with the built
/// [`Service`].
#[derive(Clone)]
pub struct ImpulseRingListener {
  app_name: String,
  access_key: Option<String>,
  wt_handler: Option<RingWebTransportHandler>,
}

impl std::fmt::Debug for ImpulseRingListener {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ImpulseRingListener")
      .field("app_name", &self.app_name)
      .field("access_key", &self.access_key.as_ref().map(|_| "<set>"))
      .field("wt_handler", &self.wt_handler.as_ref().map(|_| "<set>"))
      .finish()
  }
}

impl ImpulseRingListener {
  /// Create a listener for the application named `app_name`.
  ///
  /// `app_name` is what clients pass to `ImpulseRingClient::connect`.
  pub fn new(app_name: impl Into<String>) -> Self {
    ImpulseRingListener {
      app_name: app_name.into(),
      access_key: None,
      wt_handler: None,
    }
  }

  /// Require callers to present this access key (gated by the broker).
  #[must_use]
  pub fn with_key(mut self, key: impl Into<String>) -> Self {
    self.access_key = Some(key.into());
    self
  }

  /// Register a handler for WebTransport-over-Ring sessions.
  ///
  /// Each accepted session is passed to `handler`, which drives it (datagrams and
  /// bidirectional streams) to completion.
  #[must_use]
  pub fn on_webtransport(mut self, handler: RingWebTransportHandler) -> Self {
    self.wt_handler = Some(handler);
    self
  }

  /// The application name this listener serves under.
  pub fn app_name(&self) -> &str {
    &self.app_name
  }

  /// The bus function name derived from the application name.
  pub fn fn_name(&self) -> String {
    http_fn_name(&self.app_name)
  }

  /// Register on the bus and serve `service` until `shutdown` resolves.
  ///
  /// This is a convenience wrapper around [`serve_impulse_ring`].
  pub async fn serve<F>(self, service: Service, shutdown: F) -> MResult<()>
  where
    F: std::future::Future<Output = ()> + Send + 'static,
  {
    serve_impulse_ring(self, service, shutdown).await
  }
}

/// Register `listener` on the Ring bus and serve `service` until `shutdown`
/// resolves.
///
/// Each incoming request is converted into a salvo [`Request`], run through the
/// service's full pipeline, and the resulting [`Response`] is shipped back to
/// the caller. The bus connection is torn down (unregistering the application)
/// when this future completes.
pub async fn serve_impulse_ring<F>(listener: ImpulseRingListener, service: Service, shutdown: F) -> MResult<()>
where
  F: std::future::Future<Output = ()> + Send + 'static,
{
  let app_name = listener.app_name.clone();
  let fn_name = listener.fn_name();
  let key = listener.access_key.clone();
  let wt_handler = listener.wt_handler.clone();
  let rt = tokio::runtime::Handle::current();

  // The connector is blocking and thread-based; set it up off the async
  // runtime, then keep the connection alive for the lifetime of the server.
  let setup_app = app_name.clone();
  let setup_key = key.clone();
  let conn = tokio::task::spawn_blocking(move || -> io::Result<Arc<Connection>> {
    let conn = Arc::new(Connection::connect(&format!("{setup_app}-ring-server"))?);
    // The handler holds a *weak* reference back to the connection so the strong
    // count owned here (and dropped on shutdown) still reaches zero — otherwise
    // the service thread's closure would keep the bus alive forever.
    let handler = RingHttpHandler {
      service: Arc::new(service),
      conn: Arc::downgrade(&conn),
      app_name: Arc::new(setup_app),
      key: setup_key.map(Arc::new),
      rt: rt.clone(),
      wt_handler,
    };
    conn.expose_function::<RingHttpRequest, RingHttpResponse, _>(
      &fn_name,
      REQUEST_SCHEMA,
      RESPONSE_SCHEMA,
      key.as_deref(),
      move |req| handler.rt.clone().block_on(handler.handle(req)),
    )?;
    Ok(conn)
  })
  .await
  .map_err(|e| ServerError::from_private(e).with_500())?
  .map_err(|e| {
    ServerError::from_private(e)
      .with_public("Failed to register the application on the Ring bus (is `impulsed` running?).")
      .with_500()
  })?;

  tracing::info!("Listening for HTTP over Ring as application '{}'.", listener.app_name);

  // Block the connection from being dropped until shutdown is requested.
  shutdown.await;
  tracing::info!("Ring listener for '{}' shutting down.", listener.app_name);
  drop(conn);
  Ok(())
}

/// Drives the salvo pipeline for a single Ring request. Cheap to clone.
struct RingHttpHandler {
  service: Arc<Service>,
  /// Weak ref to the bus, used to publish/subscribe stream channels on demand.
  conn: Weak<Connection>,
  /// Application name, used to derive stream channel names.
  app_name: Arc<String>,
  /// Access key gating the stream channels we publish.
  key: Option<Arc<String>>,
  /// Runtime handle for spawning background stream pumps / virtual connections.
  rt: Handle,
  /// Optional Ring-native WebTransport session handler.
  wt_handler: Option<RingWebTransportHandler>,
}

impl RingHttpHandler {
  /// Convert a wire request into a salvo response (or a streaming handshake).
  async fn handle(&self, req: RingHttpRequest) -> RingHttpResponse {
    let result = match upgrade_intent(&req) {
      Some(RingUpgradeKind::Sse) => self.handle_sse(req).await,
      Some(RingUpgradeKind::WebSocket) => self.handle_websocket(req).await,
      Some(RingUpgradeKind::WebTransport) => self.handle_webtransport(req).await,
      None => self.handle_plain(req).await,
    };
    match result {
      Ok(resp) => resp,
      Err(e) => RingHttpResponse {
        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
        headers: vec![RingHeader::new("content-type", "text/plain; charset=utf-8")],
        body: format!("ring listener error: {e}").into_bytes(),
      },
    }
  }

  /// Run the salvo pipeline for `req`, returning a salvo [`Response`].
  async fn run_pipeline(&self, req: RingHttpRequest) -> io::Result<Response> {
    let salvo_req = build_salvo_request(req)?;
    // Build the hyper handler per request (cheap: it just clones `Arc`s) and run
    // the full routing / middleware / catcher pipeline.
    Ok(
      self
        .service
        .hyper_handler(SocketAddr::Unknown, SocketAddr::Unknown, Scheme::HTTP, None, None)
        .handle(salvo_req)
        .await,
    )
  }

  /// Plain one-shot HTTP: run the pipeline and collect the whole response body.
  async fn handle_plain(&self, req: RingHttpRequest) -> io::Result<RingHttpResponse> {
    let mut res = self.run_pipeline(req).await?;
    response_to_wire(&mut res).await
  }

  /// Server-Sent Events: run the pipeline, and if the handler produced an
  /// event-stream response, publish a channel and stream the body onto it,
  /// answering the RPC with the upgrade handshake.
  async fn handle_sse(&self, req: RingHttpRequest) -> io::Result<RingHttpResponse> {
    let mut res = self.run_pipeline(req).await?;

    // Only treat genuine event-stream responses as SSE; otherwise fall back to a
    // normal collected response (e.g. the route returned an error page).
    if !response_is_event_stream(&res) {
      return response_to_wire(&mut res).await;
    }

    let conn = self.conn()?;
    let session = next_session_id();
    let down_name = stream_channel_name(&self.app_name, session, "down");
    let publisher = conn.publish_channel(&down_name, STREAM_SCHEMA, self.key())?;

    // Handshake: hand back the response's status + headers plus the channel name.
    let status = res.status_code.unwrap_or(StatusCode::OK).as_u16() as i32;
    let mut headers: Vec<RingHeader> = wire_headers(&res);
    headers.push(RingHeader::new(HEADER_UPGRADE, RingUpgradeKind::Sse.as_str()));
    headers.push(RingHeader::new(HEADER_CHAN_DOWN, down_name));

    // Pump the body onto the channel in the background.
    let body = res.take_body();
    self.rt.spawn(async move {
      pump_body_to_channel(body, publisher).await;
    });

    Ok(RingHttpResponse {
      status,
      headers,
      body: Vec::new(),
    })
  }

  /// Upgrade the strong connection, or fail if the listener is shutting down.
  fn conn(&self) -> io::Result<Arc<Connection>> {
    self
      .conn
      .upgrade()
      .ok_or_else(|| io::Error::other("ring listener is shutting down"))
  }

  fn key(&self) -> Option<&str> {
    self.key.as_deref().map(|s| s.as_str())
  }

  /// WebSocket upgrade over Ring.
  ///
  /// The bytes of the upgraded connection flow over a pair of Ring channels (the
  /// client published its `up` channel and named it in the handshake; we publish
  /// the `down` channel here). We then drive the salvo pipeline over that
  /// channel pair as an ordinary HTTP/1.1 connection *with upgrades*, so salvo's
  /// native `WebSocketUpgrade` works unchanged — Ring only relays the bytes.
  async fn handle_websocket(&self, req: RingHttpRequest) -> io::Result<RingHttpResponse> {
    use impulse_client_ring::streaming::{RingDuplex, subscribe_by_name};
    use impulse_ring_http::{HEADER_CHAN_UP, HEADER_SESSION};
    use salvo::conn::http1;
    use salvo::rt::tokio::TokioIo;

    let conn = self.conn()?;
    let up_name = req_header(&req, HEADER_CHAN_UP)
      .ok_or_else(|| io::Error::other("WebSocket handshake is missing the up-channel header"))?
      .to_string();
    let session = req_header(&req, HEADER_SESSION)
      .and_then(|s| u64::from_str_radix(s, 16).ok())
      .unwrap_or_else(next_session_id);
    let down_name = stream_channel_name(&self.app_name, session, "down");

    // Server outbound on `down`, inbound from the client's `up`.
    let publisher = conn.publish_channel(&down_name, STREAM_SCHEMA, self.key())?;
    let subscriber = subscribe_by_name(&conn, &up_name, self.key())?;
    let duplex = RingDuplex::new(conn.clone(), publisher, subscriber);

    let hyper_handler = self
      .service
      .hyper_handler(SocketAddr::Unknown, SocketAddr::Unknown, Scheme::HTTP, None, None);
    self.rt.spawn(async move {
      let io = TokioIo::new(duplex);
      if let Err(e) = http1::Builder::new()
        .serve_connection(io, hyper_handler)
        .with_upgrades()
        .await
      {
        tracing::debug!("ring websocket connection ended: {e}");
      }
    });

    Ok(RingHttpResponse {
      status: StatusCode::SWITCHING_PROTOCOLS.as_u16() as i32,
      headers: vec![
        RingHeader::new(HEADER_UPGRADE, RingUpgradeKind::WebSocket.as_str()),
        RingHeader::new(HEADER_CHAN_DOWN, down_name),
      ],
      body: Vec::new(),
    })
  }

  /// WebTransport session over Ring.
  ///
  /// salvo's native WebTransport requires a QUIC connection, which Ring does not
  /// provide, so the session is handed to the [`RingWebTransportHandler`]
  /// registered on the listener instead. The real WebTransport-over-HTTP/3 edge
  /// (e.g. LBRP) terminates the protocol and relays the session here.
  async fn handle_webtransport(&self, req: RingHttpRequest) -> io::Result<RingHttpResponse> {
    use impulse_client_ring::streaming::{RingWebTransport, subscribe_by_name};
    use impulse_ring_http::{HEADER_CHAN_UP, HEADER_SESSION};

    let handler = self
      .wt_handler
      .clone()
      .ok_or_else(|| io::Error::other("this service does not handle WebTransport over Ring"))?;
    let conn = self.conn()?;
    let up_name = req_header(&req, HEADER_CHAN_UP)
      .ok_or_else(|| io::Error::other("WebTransport handshake is missing the up-channel header"))?
      .to_string();
    let session = req_header(&req, HEADER_SESSION)
      .and_then(|s| u64::from_str_radix(s, 16).ok())
      .unwrap_or_else(next_session_id);
    let down_name = stream_channel_name(&self.app_name, session, "down");

    let publisher = conn.publish_channel(&down_name, STREAM_SCHEMA, self.key())?;
    let subscriber = subscribe_by_name(&conn, &up_name, self.key())?;
    // The server is the non-initiator (odd stream ids).
    let wt = RingWebTransport::new(conn.clone(), publisher, subscriber, false);
    self.rt.spawn(handler(wt));

    Ok(RingHttpResponse {
      status: StatusCode::OK.as_u16() as i32,
      headers: vec![
        RingHeader::new(HEADER_UPGRADE, RingUpgradeKind::WebTransport.as_str()),
        RingHeader::new(HEADER_CHAN_DOWN, down_name),
      ],
      body: Vec::new(),
    })
  }
}

/// Detect a streaming/upgrade intent from the request headers.
fn upgrade_intent(req: &RingHttpRequest) -> Option<RingUpgradeKind> {
  // An explicit handshake header wins.
  if let Some(v) = req_header(req, HEADER_UPGRADE)
    && let Some(kind) = RingUpgradeKind::parse(v)
  {
    return Some(kind);
  }
  // Standard `Upgrade: websocket`.
  if req_header(req, "upgrade").is_some_and(|v| v.eq_ignore_ascii_case("websocket")) {
    return Some(RingUpgradeKind::WebSocket);
  }
  // `Accept: text/event-stream` requests an SSE stream.
  if req_header(req, "accept").is_some_and(|v| v.to_ascii_lowercase().contains("text/event-stream")) {
    return Some(RingUpgradeKind::Sse);
  }
  None
}

/// Case-insensitive request header lookup.
fn req_header<'a>(req: &'a RingHttpRequest, name: &str) -> Option<&'a str> {
  req
    .headers
    .iter()
    .find(|h| h.name.eq_ignore_ascii_case(name))
    .map(|h| h.value.as_str())
}

/// Whether a salvo response is a `text/event-stream`.
fn response_is_event_stream(res: &Response) -> bool {
  res
    .headers()
    .get(salvo::http::header::CONTENT_TYPE)
    .and_then(|v| v.to_str().ok())
    .is_some_and(|ct| ct.to_ascii_lowercase().contains("text/event-stream"))
}

/// Collect a salvo response's headers into the wire representation.
fn wire_headers(res: &Response) -> Vec<RingHeader> {
  res
    .headers()
    .iter()
    .map(|(name, value)| RingHeader::new(name.as_str(), String::from_utf8_lossy(value.as_bytes()).into_owned()))
    .collect()
}

/// Drain a response body, publishing each chunk as a `DATA` frame and a final
/// `CLOSE` frame on the given channel.
async fn pump_body_to_channel(mut body: salvo::http::ResBody, publisher: impulse_ring_connector::Publisher) {
  loop {
    match body.frame().await {
      Some(Ok(frame)) => {
        if let Ok(data) = frame.into_data()
          && !data.is_empty()
          && publisher.publish(&RingStreamFrame::data(data.to_vec())).is_err()
        {
          break; // subscriber gone
        }
      }
      Some(Err(_)) => break,
      None => break, // end of body
    }
  }
  let _ = publisher.publish(&RingStreamFrame::close());
}

/// A process-unique session id for a streaming upgrade.
fn next_session_id() -> u64 {
  use std::sync::atomic::{AtomicU64, Ordering};
  static SEQ: AtomicU64 = AtomicU64::new(0);
  let seq = SEQ.fetch_add(1, Ordering::Relaxed);
  let nanos = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_nanos() as u64)
    .unwrap_or(0);
  (nanos << 16) ^ ((std::process::id() as u64) << 8) ^ seq
}

/// Build a salvo [`Request`] from the wire record.
fn build_salvo_request(req: RingHttpRequest) -> io::Result<Request> {
  use salvo::http::{Method, header::HeaderName, header::HeaderValue};

  let method = Method::from_bytes(req.method.as_bytes()).map_err(io::Error::other)?;
  // A relative target is enough for routing; default to "/" when empty.
  let target = if req.uri.is_empty() { "/" } else { req.uri.as_str() };

  let mut builder = http::Request::builder().method(method).uri(target);
  for h in &req.headers {
    let name = HeaderName::from_bytes(h.name.as_bytes()).map_err(io::Error::other)?;
    let value = HeaderValue::from_str(&h.value).map_err(io::Error::other)?;
    builder = builder.header(name, value);
  }
  let hyper_req = builder.body(bytes::Bytes::from(req.body)).map_err(io::Error::other)?;

  Ok(Request::from_hyper(hyper_req, Scheme::HTTP))
}

/// Serialize a finalized salvo [`Response`] into the wire record.
async fn response_to_wire(res: &mut Response) -> io::Result<RingHttpResponse> {
  let status = res.status_code.unwrap_or(StatusCode::OK).as_u16() as i32;

  let headers = res
    .headers()
    .iter()
    .map(|(name, value)| RingHeader::new(name.as_str(), String::from_utf8_lossy(value.as_bytes()).into_owned()))
    .collect();

  let body = res
    .take_body()
    .collect()
    .await
    .map_err(io::Error::other)?
    .to_bytes()
    .to_vec();

  Ok(RingHttpResponse { status, headers, body })
}
