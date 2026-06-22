//! Startup module.
//!
//! In most cases, you just need to use `start` function:
//!
//! ```rust,ignore
//! let (server, _) = start(app_state, app_config, router).await.unwrap();
//! server.await
//! ```
//!
//! The server listens on the *set* of protocols declared under `protocols:` in
//! the YAML config (see [`crate::setup::ProtocolConfig`]): any mix of HTTP/1.1,
//! HTTP/2, HTTP/3 (QUIC) and the Ring shared-memory bus, all at once.

use impulse_utils::errors::ServerError;
use impulse_utils::prelude::MResult;
use salvo::prelude::*;

use salvo::server::ServerHandle;
use std::future::Future;
use std::pin::Pin;
use std::process::Command;
use std::task::{Context, Poll};

use salvo::conn::tcp::{DynTcpAcceptors, TcpCoupler};
use salvo::conn::{Accepted, Acceptor, Holding};
use salvo::fuse::ArcFuseFactory;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

// `rustls` is always enabled for `salvo`, so the TLS config types are available
// regardless of the `http3` feature — TLS-over-TCP (HTTPS) needs them too.
use salvo::conn::rustls::{Keycert, RustlsConfig};
#[cfg(feature = "http3")]
use salvo::http::HeaderValue;
#[cfg(feature = "http3")]
use salvo::http::header::ALT_SVC;

use crate::setup::{GenericServerState, GenericSetup, ResolvedProtocol};

#[cfg(feature = "http3")]
static TLS13: &[&rustls::SupportedProtocolVersion] = &[&rustls::version::TLS13];

#[cfg(feature = "http3")]
#[handler]
/// HTTP2-to-HTTP3 switching header.
///
/// Installed automatically on the cleartext listeners when any `http3` protocol
/// is configured, so clients learn they can upgrade to QUIC.
pub async fn h3_header(depot: &mut Depot, res: &mut Response) {
  let port = depot
    .obtain::<GenericServerState>()
    .ok()
    .and_then(|s| s.http3_port())
    .unwrap_or(443);

  res
    .headers_mut()
    .insert(
      ALT_SVC,
      HeaderValue::from_str(&format!(r##"h3=":{port}"; ma=2592000"##)).unwrap(),
    )
    .unwrap();
}

/// Build a `RustlsConfig` from cert/key file paths.
///
/// Used for TLS-over-TCP (HTTPS on `http1`/`http2`) where the default TLS
/// version set (1.2 + 1.3) maximises client compatibility. QUIC narrows this to
/// TLS 1.3 via [`tlsv13`].
fn rustls_config_from_paths(certpath: impl AsRef<str>, keypath: impl AsRef<str>) -> MResult<RustlsConfig> {
  Ok(RustlsConfig::new(
    Keycert::new()
      .cert_from_path(certpath.as_ref())
      .map_err(|e| ServerError::from_private(e).with_500())?
      .key_from_path(keypath.as_ref())
      .map_err(|e| ServerError::from_private(e).with_500())?,
  ))
}

#[cfg(feature = "http3")]
fn tlsv13(certpath: impl AsRef<str>, keypath: impl AsRef<str>) -> MResult<RustlsConfig> {
  Ok(rustls_config_from_paths(certpath, keypath)?.tls_versions(TLS13))
}

#[cfg(feature = "otel")]
#[handler]
/// Default Server Kit OpenTelemetry metrics.
///
/// Installed by default with `get_root_router_autoinject` method.
pub async fn sk_default_metrics(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
  let meter = crate::otel::api::global::meter("sk_metrics");

  let request_counter = meter
    .u64_counter("sk_requests")
    .with_unit("1")
    .with_description("Total number of requests")
    .build();

  let request_duration = meter
    .f64_histogram("sk_request_duration")
    .with_unit("s")
    .with_description("HTTP request duration in seconds")
    .build();

  let active_connections = meter
    .i64_up_down_counter("sk_active_connections")
    .with_unit("1")
    .with_description("Number of active HTTP connections")
    .build();

  let host = req.uri().host().map(String::from);
  let path = req.uri().path().to_string();
  let method = req.method().as_str().to_string();

  let attributes = vec![
    opentelemetry::KeyValue::new("host", host.unwrap_or(String::from("unknown"))),
    opentelemetry::KeyValue::new("path", path),
    opentelemetry::KeyValue::new("method", method),
    opentelemetry::KeyValue::new("user_agent", req.header("user-agent").unwrap_or("unknown").to_string()),
  ];

  active_connections.add(1, &[]);
  active_connections.add(1, &attributes);
  let start = tokio::time::Instant::now();

  ctrl.call_next(req, depot, res).await;

  let duration = start.elapsed().as_secs_f64();

  let mut result_attributes = attributes.clone();
  let status = res.status_code.unwrap_or(StatusCode::OK).as_u16().to_string();
  result_attributes.push(opentelemetry::KeyValue::new("status", status));

  request_counter.add(1, &[]);
  request_counter.add(1, &result_attributes);
  request_duration.record(duration, &result_attributes);

  active_connections.add(-1, &attributes);
}

/// Returns preconfigured router with app state and OpenTelemetry metrics injected.
///
/// To get your `app_config` inside handler/endpoint, call
/// `depot.obtain::<YourAppConfigType>().unwrap()`.
pub fn get_root_router_autoinject<T: GenericSetup + Send + Sync + Clone + 'static>(
  app_state: &GenericServerState,
  app_config: T,
) -> Router {
  #[allow(unused_mut)]
  let mut router = Router::new().hoop(affix_state::inject(app_state.clone()).inject(app_config.clone()));

  let sec = &app_config.generic_values().security_headers;
  if sec.enabled {
    router = router.hoop(crate::security_headers::SecurityHeaders::new(sec));
  }

  #[cfg(feature = "http3")]
  if app_state.uses_http3() {
    router = router.hoop(h3_header);
  }

  #[cfg(feature = "otel")]
  if app_config.generic_values().tracing_options.otel_http_endpoint.is_some() {
    router = router.hoop(sk_default_metrics);
  }

  router
}

/// Returns preconfigured root router to use.
///
/// Usually it installs `h3_header` for switching protocol to QUIC, if any
/// `http3` protocol is configured.
#[allow(unused_variables)]
pub fn get_root_router(app_state: &GenericServerState) -> Router {
  #[allow(unused_mut)]
  let mut router = Router::new();

  #[cfg(feature = "http3")]
  if app_state.uses_http3() {
    router = router.hoop(h3_header);
  }

  router
}

#[cfg(feature = "oapi")]
#[allow(clippy::mut_from_ref, invalid_reference_casting)]
unsafe fn make_mut<T>(reference: &T) -> &mut T {
  let const_ptr = reference as *const T;
  let mut_ptr = const_ptr as *mut T;
  unsafe { &mut *mut_ptr }
}

/// Starts up HTTPS redirect server.
///
/// Example:
///
/// ```rust,ignore
/// let (server, _) = start(app_state, app_config, router).await.unwrap();
/// let (redirect, _) = start_force_https_redirect(80, 443).await.unwrap();
///
/// tracing::info!("Server is booted.");
///
/// tokio::select! {
///   _ = server   => tracing::info!("Server is shutdowned."),
///   _ = redirect => tracing::info!("Redirect is shutdowned."),
/// }
/// ```
#[cfg(feature = "force-https")]
pub async fn start_force_https_redirect(
  listen_port: u16,
  redirect_port: u16,
) -> MResult<(Pin<Box<dyn Future<Output = ()> + Send>>, ServerHandle)> {
  let service = Service::new(Router::new()).hoop(ForceHttps::new().https_port(redirect_port));
  let acceptor = TcpListener::new(format!("0.0.0.0:{listen_port}")).bind().await;
  let server = Server::new(acceptor);
  let handle = server.handle();
  let server = Box::pin(server.serve(service));
  Ok((server, handle))
}

/// Clone the configured [`Service`] so several listeners can share it.
///
/// `Service` is not `Clone`, but all of its parts are cheap to share, so this
/// rebuilds an equivalent service pointing at the same router/catcher/hoops.
fn clone_service(service: &Service) -> Service {
  let mut cloned = Service::new(service.router.clone());
  cloned.catcher = service.catcher.clone();
  cloned.hoops = service.hoops.clone();
  cloned.allowed_media_types = service.allowed_media_types.clone();
  cloned
}

/// Starts your application with provided service, if you predefined one by yourself.
///
/// For example, you can setup service with error catcher or any other middleware that
/// `salvo` provides.
pub async fn start_with_service(
  app_state: GenericServerState,
  app_config: &impl GenericSetup,
  #[allow(unused_mut)] mut service: Service,
) -> MResult<(Pin<Box<dyn Future<Output = ()> + Send>>, ServerHandle)> {
  tracing::info!("Server is starting...");

  // Идемпотентно: повторная установка дефолтного crypto-провайдера (например, при
  // reload-конфига, когда `start_with_service` входит снова) возвращает `Err`, потому
  // что провайдер уже установлен. Это не ошибка — игнорируем результат.
  let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
  let app_config = app_config.generic_values();

  if let Some(bin) = app_config.auto_migrate_bin.as_ref() {
    Command::new(bin)
      .spawn()
      .map_err(|e| ServerError::from_private(e).with_500())?;
  }

  #[cfg(feature = "oapi")]
  if app_config.allow_oapi_access.is_some_and(|v| v) {
    let doc = OpenApi::new(
      app_config.oapi_name.as_ref().unwrap(),
      app_config.oapi_ver.as_ref().unwrap(),
    )
    .merge_router(&service.router);

    let oapi_endpoint = if let Some(ftype) = app_config.oapi_frontend_type.as_ref() {
      match ftype.as_str() {
        "Scalar" => Some(
          Scalar::new(format!("{}/openapi.json", app_config.oapi_api_addr.as_ref().unwrap()))
            .title(format!(
              "{} - API @ Scalar",
              app_config.oapi_name.as_ref().unwrap_or(&app_config.app_name)
            ))
            .description(format!(
              "{} - API",
              app_config.oapi_name.as_ref().unwrap_or(&app_config.app_name)
            ))
            .into_router(app_config.oapi_api_addr.as_ref().unwrap()),
        ),
        "SwaggerUI" => Some(
          SwaggerUi::new(format!("{}/openapi.json", app_config.oapi_api_addr.as_ref().unwrap()))
            .title(format!(
              "{} - API @ SwaggerUI",
              app_config.oapi_name.as_ref().unwrap_or(&app_config.app_name)
            ))
            .description(format!(
              "{} - API",
              app_config.oapi_name.as_ref().unwrap_or(&app_config.app_name)
            ))
            .into_router(app_config.oapi_api_addr.as_ref().unwrap()),
        ),
        _ => None,
      }
    } else {
      None
    };

    let mut router = Router::new();
    router = router.push(doc.into_router(format!("{}/openapi.json", app_config.oapi_api_addr.as_ref().unwrap())));
    if let Some(oapi) = oapi_endpoint {
      router = router.push(oapi);
    }

    unsafe {
      let service_router = make_mut(service.router.as_ref());
      service_router.routers_mut().insert(0, router);
    }

    tracing::info!("API is available on {}", app_config.oapi_api_addr.as_ref().unwrap());
  }

  #[cfg(feature = "cors")]
  if let Some(domain) = &app_config.allow_cors_domain {
    let cors = salvo::cors::Cors::new()
      .allow_origin(domain)
      .allow_credentials(domain.as_str() != "*")
      .allow_headers(vec![
        "Authorization",
        "Accept",
        "Access-Control-Allow-Headers",
        "Content-Type",
        "Origin",
        "X-Requested-With",
        "Cookie",
      ])
      .expose_headers(vec!["Set-Cookie"])
      .allow_methods(vec![
        salvo::http::Method::GET,
        salvo::http::Method::POST,
        salvo::http::Method::PUT,
        salvo::http::Method::PATCH,
        salvo::http::Method::DELETE,
        salvo::http::Method::OPTIONS,
      ])
      .into_handler();

    service = service.hoop(cors);
  }

  serve_protocols(&app_state, service).await
}

/// Bring up every configured protocol and return a combined future plus a
/// control handle. Stopping the handle (e.g. on Ctrl+C) gracefully shuts every
/// listener down.
async fn serve_protocols(
  app_state: &GenericServerState,
  service: Service,
) -> MResult<(Pin<Box<dyn Future<Output = ()> + Send>>, ServerHandle)> {
  let master = CancellationToken::new();
  let mut real_handles: Vec<ServerHandle> = Vec::new();
  let mut tasks: JoinSet<()> = JoinSet::new();

  // HTTP/1.1 + HTTP/2 over TCP. Cleartext listeners share one TCP server (their
  // plain streams are type-compatible); each TLS-terminating listener gets its
  // own server, since a TLS stream isn't compatible with the cleartext acceptor
  // set in `DynTcpAcceptors`.
  let mut tcp_acceptors = Vec::new();
  for proto in &app_state.protocols {
    let (host, port, tls) = match proto {
      ResolvedProtocol::Http1 {
        host,
        port,
        ssl_key_path,
        ssl_crt_path,
      }
      | ResolvedProtocol::Http2 {
        host,
        port,
        ssl_key_path,
        ssl_crt_path,
      } => (host, *port, ssl_crt_path.as_ref().zip(ssl_key_path.as_ref())),
      _ => continue,
    };

    match tls {
      // HTTPS over TCP: terminate TLS on a dedicated server for this endpoint.
      Some((crt, key)) => {
        let rustls_config = rustls_config_from_paths(crt, key)?;
        let acceptor = TcpListener::new(format!("{host}:{port}"))
          .rustls(rustls_config)
          .try_bind()
          .await
          .map_err(|e| {
            ServerError::from_private(e)
              .with_public("Failed to bind a TLS (HTTPS) listener (port already in use?).")
              .with_500()
          })?;
        tracing::info!("Listening for HTTPS (TLS over TCP) on {host}:{port}.");
        let server = Server::new(acceptor);
        real_handles.push(server.handle());
        let svc = clone_service(&service);
        tasks.spawn(async move {
          server.serve(svc).await;
        });
      }
      // Cleartext: collect into the shared TCP server below.
      None => {
        let acceptor = TcpListener::new(format!("{host}:{port}"))
          .try_bind()
          .await
          .map_err(|e| {
            ServerError::from_private(e)
              .with_public("Failed to bind a TCP listener (port already in use?).")
              .with_500()
          })?;
        tracing::info!("Listening for HTTP (cleartext) over TCP on {host}:{port}.");
        tcp_acceptors.push(acceptor.into_boxed());
      }
    }
  }
  if !tcp_acceptors.is_empty() {
    let server = Server::new(DynTcpAcceptors::new(tcp_acceptors));
    real_handles.push(server.handle());
    let svc = clone_service(&service);
    tasks.spawn(async move {
      server.serve(svc).await;
    });
  }

  // HTTP/3 (QUIC), one server per configured endpoint.
  #[cfg(feature = "http3")]
  for proto in &app_state.protocols {
    if let ResolvedProtocol::Http3 {
      host,
      port,
      ssl_key_path,
      ssl_crt_path,
    } = proto
    {
      let rustls_config = tlsv13(ssl_crt_path, ssl_key_path)?;
      let quinn_config = rustls_config
        .build_quinn_config()
        .map_err(|e| ServerError::from_private(e).with_500())?;
      let acceptor = QuinnListener::new(quinn_config, format!("{host}:{port}")).bind().await;
      tracing::info!("Listening for HTTP/3 (QUIC) on {host}:{port}.");
      let server = Server::new(acceptor);
      real_handles.push(server.handle());
      let svc = clone_service(&service);
      tasks.spawn(async move {
        server.serve(svc).await;
      });
    }
  }

  // Ring shared-memory listeners.
  #[cfg(feature = "impulse-ring")]
  for proto in &app_state.protocols {
    if let ResolvedProtocol::ImpulseRing { app_name, access_key } = proto {
      let mut listener = crate::impulse_ring::ImpulseRingListener::new(app_name.clone());
      if let Some(key) = access_key {
        listener = listener.with_key(key.clone());
      }
      let svc = clone_service(&service);
      let token = master.clone();
      tasks.spawn(async move {
        if let Err(e) = crate::impulse_ring::serve_impulse_ring(listener, svc, token.cancelled_owned()).await {
          tracing::error!("Ring listener stopped with an error: {e:?}");
        }
      });
    }
  }

  // A connection-less control server: it never accepts, but it gives us a real
  // `ServerHandle` (so Ctrl+C works even for shared-memory-only deployments) and
  // is the single point that cascades shutdown to every other listener.
  let control = Server::new(NoopAcceptor::new());
  let handle = control.handle();
  let control_svc = clone_service(&service);
  let cascade = master.clone();
  tasks.spawn(async move {
    control.serve(control_svc).await;
    cascade.cancel();
  });

  // When shutdown is requested, gracefully stop every network server.
  let supervised = real_handles.clone();
  let supervisor_token = master.clone();
  tokio::spawn(async move {
    supervisor_token.cancelled().await;
    for h in &supervised {
      h.stop_graceful(None);
    }
  });

  let fut = async move { while tasks.join_next().await.is_some() {} };

  Ok((Box::pin(fut), handle))
}

/// Starts the server according to the configured protocols with the custom shutdown.
pub async fn start_clean(
  app_state: GenericServerState,
  app_config: &impl GenericSetup,
  router: Router,
) -> MResult<(Pin<Box<dyn Future<Output = ()> + Send>>, ServerHandle)> {
  start_with_service(app_state, app_config, Service::new(router)).await
}

/// Starts the server according to the configured protocols.
pub async fn start(
  app_state: GenericServerState,
  app_config: &impl GenericSetup,
  router: Router,
) -> MResult<(Pin<Box<dyn Future<Output = ()> + Send>>, ServerHandle)> {
  let (fut, handle) = start_clean(app_state, app_config, router).await?;
  let ctrl_c_handle = handle.clone();
  tokio::spawn(async move { shutdown_signal(ctrl_c_handle).await });
  Ok((fut, handle))
}

/// Signal to graceful shutdown.
///
/// Required to be manually awaited, if you start server with `start_clean`/`start_with_service` functions. Example:
///
/// ```rust,ignore
/// let (server, handle) = start_clean(app_state, app_config, router).await.unwrap();
/// let default_handle = tokio::spawn(async move { shutdown_signal(handle).await });
///
/// tracing::info!("Server is booted.");
///
/// tokio::select! {
///   _ = server         => tracing::info!("Server is shutdowned."),
///   _ = default_handle => std::process::exit(0),
/// }
/// ```
///
/// Graceful coroutine starts automatically with `start` function.
pub async fn shutdown_signal(handle: ServerHandle) {
  tokio::signal::ctrl_c().await.unwrap();
  tracing::info!("Shutdown with Ctrl+C requested.");
  handle.stop_graceful(None);
}

/// A `salvo` acceptor that never yields a connection.
///
/// Used as the control server's acceptor: it lets us obtain a [`ServerHandle`]
/// (and thus graceful-shutdown wiring) without binding any socket, which matters
/// for deployments that listen only over the Ring shared-memory bus.
struct NoopAcceptor {
  holdings: Vec<Holding>,
}

impl NoopAcceptor {
  fn new() -> Self {
    NoopAcceptor { holdings: Vec::new() }
  }
}

impl Acceptor for NoopAcceptor {
  type Coupler = TcpCoupler<NoopStream>;
  type Stream = NoopStream;

  fn holdings(&self) -> &[Holding] {
    &self.holdings
  }

  async fn accept(
    &mut self,
    _fuse_factory: Option<ArcFuseFactory>,
  ) -> std::io::Result<Accepted<Self::Coupler, Self::Stream>> {
    // Never produce a connection; the server stops via its graceful-stop token.
    std::future::pending().await
  }
}

/// The (never-instantiated) stream type backing [`NoopAcceptor`].
struct NoopStream;

impl AsyncRead for NoopStream {
  fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
    Poll::Pending
  }
}

impl AsyncWrite for NoopStream {
  fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &[u8]) -> Poll<std::io::Result<usize>> {
    Poll::Pending
  }
  fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
    Poll::Ready(Ok(()))
  }
  fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
    Poll::Ready(Ok(()))
  }
}
