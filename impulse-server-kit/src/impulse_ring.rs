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
use std::sync::Arc;

use http_body_util::BodyExt;
use impulse_ring_connector::Connection;
use impulse_ring_http::{REQUEST_SCHEMA, RESPONSE_SCHEMA, RingHeader, RingHttpRequest, RingHttpResponse, http_fn_name};
use salvo::Service;
use salvo::conn::SocketAddr;
use salvo::http::uri::Scheme;
use salvo::http::{Request, Response, StatusCode};

use impulse_utils::errors::ServerError;
use impulse_utils::prelude::MResult;

/// A handle describing a Ring HTTP application to serve.
///
/// Construct it with the application name that clients will use to reach the
/// server, then hand it to [`serve_impulse_ring`] together with the built
/// [`Service`].
#[derive(Clone, Debug)]
pub struct ImpulseRingListener {
  app_name: String,
  access_key: Option<String>,
}

impl ImpulseRingListener {
  /// Create a listener for the application named `app_name`.
  ///
  /// `app_name` is what clients pass to `ImpulseRingClient::connect`.
  pub fn new(app_name: impl Into<String>) -> Self {
    ImpulseRingListener {
      app_name: app_name.into(),
      access_key: None,
    }
  }

  /// Require callers to present this access key (gated by the broker).
  #[must_use]
  pub fn with_key(mut self, key: impl Into<String>) -> Self {
    self.access_key = Some(key.into());
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

  // A handler clonable into the blocking service thread. It owns everything it
  // needs and drives the salvo pipeline on the ambient Tokio runtime.
  let handler = RingHttpHandler::new(service);
  let rt = tokio::runtime::Handle::current();

  // The connector is blocking and thread-based; set it up off the async
  // runtime, then keep the connection alive for the lifetime of the server.
  let conn = tokio::task::spawn_blocking(move || -> io::Result<Connection> {
    let conn = Connection::connect(&format!("{app_name}-ring-server"))?;
    let handler = handler.clone();
    let rt = rt.clone();
    conn.expose_function::<RingHttpRequest, RingHttpResponse, _>(
      &fn_name,
      REQUEST_SCHEMA,
      RESPONSE_SCHEMA,
      key.as_deref(),
      move |req| rt.block_on(handler.handle(req)),
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
#[derive(Clone)]
struct RingHttpHandler {
  service: Arc<Service>,
}

impl RingHttpHandler {
  fn new(service: Service) -> Self {
    RingHttpHandler {
      service: Arc::new(service),
    }
  }

  /// Convert a wire request into a salvo response.
  async fn handle(&self, req: RingHttpRequest) -> RingHttpResponse {
    match self.try_handle(req).await {
      Ok(resp) => resp,
      Err(e) => RingHttpResponse {
        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
        headers: vec![RingHeader::new("content-type", "text/plain; charset=utf-8")],
        body: format!("ring listener error: {e}").into_bytes(),
      },
    }
  }

  async fn try_handle(&self, req: RingHttpRequest) -> io::Result<RingHttpResponse> {
    let salvo_req = build_salvo_request(req)?;
    // Build the hyper handler per request (cheap: it just clones `Arc`s) and run
    // the full routing / middleware / catcher pipeline.
    let mut res = self
      .service
      .hyper_handler(SocketAddr::Unknown, SocketAddr::Unknown, Scheme::HTTP, None, None)
      .handle(salvo_req)
      .await;
    response_to_wire(&mut res).await
  }
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
