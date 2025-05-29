//! Startup module.
//!
//! In most cases, you just need to use `start` function:
//!
//! ```rust,ignore
//! let (server, _) = start(app_state, app_config, router).await.unwrap();
//! server.await
//! ```

use cc_utils::errors::ServerError;
use cc_utils::prelude::MResult;
use salvo::prelude::*;

use salvo::conn::rustls::{Keycert, RustlsConfig};
use salvo::server::ServerHandle;
use std::future::Future;
use std::pin::Pin;
use std::process::Command;

#[cfg(feature = "http3")]
use salvo::http::HeaderValue;
#[cfg(feature = "http3")]
use salvo::http::header::ALT_SVC;

use crate::generic_setup::{GenericServerState, GenericSetup, StartupVariant};

static TLS13: &[&rustls::SupportedProtocolVersion] = &[&rustls::version::TLS13];

#[cfg(feature = "http3")]
#[handler]
/// HTTP2-to-HTTP3 switching header.
///
/// Usage is `router.hoop(h3_header)`.
pub async fn h3_header(depot: &mut Depot, res: &mut Response) {
  use crate::generic_setup::GenericValues;

  let server_port = match depot.obtain::<GenericValues>() {
    Ok(app_config) => app_config.server_port.unwrap(),
    Err(_) => 443,
  };

  res
    .headers_mut()
    .insert(
      ALT_SVC,
      HeaderValue::from_str(&format!(r##"h3=":{}"; ma=2592000"##, server_port)).unwrap(),
    )
    .unwrap();
}

fn tlsv13(certpath: impl AsRef<str>, keypath: impl AsRef<str>) -> MResult<RustlsConfig> {
  Ok(
    RustlsConfig::new(
      Keycert::new()
        .cert_from_path(certpath.as_ref())
        .map_err(|e| ServerError::from_private(e).with_500())?
        .key_from_path(keypath.as_ref())
        .map_err(|e| ServerError::from_private(e).with_500())?,
    )
    .tls_versions(TLS13),
  )
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

  #[cfg(all(feature = "http3", feature = "acme"))]
  if app_state.startup_variant == StartupVariant::QuinnAcme {
    router = router.hoop(h3_header);
  }

  #[cfg(feature = "http3")]
  if app_state.startup_variant == StartupVariant::Quinn || app_state.startup_variant == StartupVariant::QuinnOnly {
    router = router.hoop(h3_header);
  }

  #[cfg(feature = "otel")]
  if app_config.generic_values().otel_http_endpoint.is_some() {
    router = router.hoop(sk_default_metrics);
  }

  router
}

/// Returns preconfigured root router to use.
///
/// Usually it installs application config and state in `affix_state` and installs `h3_header` for switching protocol to QUIC, if used.
#[allow(unused_variables)]
pub fn get_root_router(app_state: &GenericServerState) -> Router {
  #[allow(unused_mut)]
  let mut router = Router::new();

  #[cfg(all(feature = "http3", feature = "acme"))]
  if app_state.startup_variant == StartupVariant::QuinnAcme {
    router = router.hoop(h3_header);
  }

  #[cfg(feature = "http3")]
  if app_state.startup_variant == StartupVariant::Quinn || app_state.startup_variant == StartupVariant::QuinnOnly {
    router = router.hoop(h3_header);
  }

  router
}

#[cfg(any(feature = "oapi", feature = "acme"))]
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
  let acceptor = TcpListener::new(format!("0.0.0.0:{}", listen_port)).bind().await;
  let server = Server::new(acceptor);
  let handle = server.handle();
  let server = Box::pin(server.serve(service));
  Ok((server, handle))
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

  rustls::crypto::aws_lc_rs::default_provider()
    .install_default()
    .map_err(|_| ServerError::from_private_str("Can't install default crypto provider!").with_500())?;
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

  let handle;

  let server = match app_state.startup_variant {
    StartupVariant::HttpLocalhost => {
      let acceptor = TcpListener::new(format!("127.0.0.1:{}", app_config.server_port.unwrap()))
        .bind()
        .await;
      let server = Server::new(acceptor);
      handle = server.handle();
      Box::pin(server.serve(service)) as Pin<Box<dyn Future<Output = ()> + Send>>
    }
    StartupVariant::UnsafeHttp => {
      let acceptor = TcpListener::new(format!(
        "{}:{}",
        app_config.server_host.as_ref().unwrap(),
        app_config.server_port.unwrap()
      ))
      .bind()
      .await;
      let server = Server::new(acceptor);
      handle = server.handle();
      Box::pin(server.serve(service))
    }
    #[cfg(feature = "acme")]
    StartupVariant::HttpsAcme => {
      let acceptor = TcpListener::new(format!(
        "{}:{}",
        app_config.server_host.as_ref().unwrap(),
        app_config.server_port.unwrap()
      ))
      .acme()
      .cache_path("tmp/letsencrypt")
      .add_domain(app_config.acme_domain.as_ref().unwrap())
      .bind()
      .await;
      let server = Server::new(acceptor);
      handle = server.handle();
      Box::pin(server.serve(service))
    }
    StartupVariant::HttpsOnly => {
      let rustls_config = tlsv13(
        app_config.ssl_crt_path.as_ref().unwrap(),
        app_config.ssl_key_path.as_ref().unwrap(),
      )?;
      let listener = TcpListener::new(format!(
        "{}:{}",
        app_config.server_host.as_ref().unwrap(),
        app_config.server_port.unwrap()
      ))
      .rustls(rustls_config)
      .bind()
      .await;

      let server = Server::new(listener);
      handle = server.handle();
      Box::pin(server.serve(service))
    }
    #[cfg(all(feature = "http3", feature = "acme"))]
    StartupVariant::QuinnAcme => {
      let acceptor = TcpListener::new(format!(
        "{}:{}",
        app_config.server_host.as_ref().unwrap(),
        app_config.server_port.unwrap()
      ))
      .acme()
      .cache_path("tmp/letsencrypt")
      .add_domain(app_config.acme_domain.as_ref().unwrap())
      .quinn(format!(
        "{}:{}",
        app_config.server_host.as_ref().unwrap(),
        app_config.server_port.unwrap()
      ))
      .bind()
      .await;
      let server = Server::new(acceptor);
      handle = server.handle();
      Box::pin(server.serve(service))
    }
    #[cfg(feature = "http3")]
    StartupVariant::Quinn => {
      let rustls_config = tlsv13(
        app_config.ssl_crt_path.as_ref().unwrap(),
        app_config.ssl_key_path.as_ref().unwrap(),
      )?;
      let listener = TcpListener::new(format!(
        "{}:{}",
        app_config.server_host.as_ref().unwrap(),
        app_config.server_port.unwrap()
      ))
      .rustls(rustls_config.clone());

      let quinn_config = rustls_config
        .build_quinn_config()
        .map_err(|e| ServerError::from_private(e).with_500())?;
      let acceptor = QuinnListener::new(
        quinn_config,
        format!(
          "{}:{}",
          app_config.server_host.as_ref().unwrap(),
          app_config.server_port.unwrap()
        ),
      )
      .join(listener)
      .bind()
      .await;

      let server = Server::new(acceptor);
      handle = server.handle();
      Box::pin(server.serve(service))
    }
    #[cfg(feature = "http3")]
    StartupVariant::QuinnOnly => {
      let quinn_config = tlsv13(
        app_config.ssl_crt_path.as_ref().unwrap(),
        app_config.ssl_key_path.as_ref().unwrap(),
      )?
      .build_quinn_config()
      .map_err(|e| ServerError::from_private(e).with_500())?;
      let acceptor = QuinnListener::new(
        quinn_config,
        format!(
          "{}:{}",
          app_config.server_host.as_ref().unwrap(),
          app_config.server_port.unwrap()
        ),
      )
      .bind()
      .await;

      let server = Server::new(acceptor);
      handle = server.handle();
      Box::pin(server.serve(service))
    }
  };

  Ok((server, handle))
}

/// Starts the server according to the startup variant provided with the custom shutdown.
pub async fn start_clean(
  app_state: GenericServerState,
  app_config: &impl GenericSetup,
  router: Router,
) -> MResult<(Pin<Box<dyn Future<Output = ()> + Send>>, ServerHandle)> {
  start_with_service(app_state, app_config, Service::new(router)).await
}

/// Starts the server according to the startup variant provided.
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
