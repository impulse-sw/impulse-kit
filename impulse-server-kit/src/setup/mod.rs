//! Setup module.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::io::Read;
#[cfg(feature = "leptos-ssr")]
use std::path::PathBuf;
use std::sync::Arc;

use impulse_utils::prelude::*;

pub mod port_achiever;
pub mod tracing_init;

use crate::setup::tracing_init::{TracingGuards, TracingOptions};

/// Provides at least values needed by Server Kit to start.
pub trait GenericSetup {
  /// Provides generic values; see `GenericValues`.
  fn generic_values(&self) -> &GenericValues;
  /// Provides mutable generic values; see `GenericValues`.
  fn generic_values_mut(&mut self) -> &mut GenericValues;
}

/// One listening protocol as written in YAML under `protocols:`.
///
/// The server always listens on a *set* of protocols. The first three are
/// served over the network and require a host and port; `impulse-ring` is served
/// over the shared-memory bus and is addressed by an application name.
///
/// ```yaml
/// protocols:
///   - type: http1           # HTTP/1.1 — required for WebSockets
///     host: 0.0.0.0
///     port: 8080
///   - type: http2           # HTTP/2 (cleartext h2c)
///     host: 0.0.0.0
///     port: 8081
///   - type: http3           # HTTP/3 (QUIC); needs TLS key + cert
///     host: 0.0.0.0
///     port: 8082
///     ssl_key_path: /etc/ssl/app.key
///     ssl_crt_path: /etc/ssl/app.crt
///   - type: impulse-ring    # shared-memory IPC, no host/port
///     app_name: my-service
/// ```
#[derive(Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ProtocolConfig {
  /// HTTP/1.1 (cleartext, over TCP). Required for WebSockets.
  #[serde(rename = "http1", alias = "http/1.1", alias = "http1.1", alias = "http_localhost")]
  Http1 {
    /// Bind host, e.g. `0.0.0.0` or `127.0.0.1`.
    host: String,
    /// Bind port.
    port: u16,
  },
  /// HTTP/2 (cleartext h2c, over TCP).
  #[serde(rename = "http2", alias = "http/2")]
  Http2 {
    /// Bind host.
    host: String,
    /// Bind port.
    port: u16,
  },
  /// HTTP/3 (QUIC, over TLS). Requires a TLS key and certificate.
  #[serde(rename = "http3", alias = "http/3", alias = "quic")]
  Http3 {
    /// Bind host.
    host: String,
    /// Bind port.
    port: u16,
    /// Path to the TLS private key.
    ssl_key_path: String,
    /// Path to the TLS certificate.
    ssl_crt_path: String,
  },
  /// Impulse Ring shared-memory listener, addressed by application name.
  #[serde(rename = "impulse-ring", alias = "impulse_ring", alias = "ring")]
  ImpulseRing {
    /// The application name clients connect to over the Ring bus.
    app_name: String,
    /// Optional access key required of callers (gated by the broker).
    #[serde(default)]
    access_key: Option<String>,
  },
}

/// A validated, ready-to-serve protocol (the resolved form of [`ProtocolConfig`]).
#[derive(Clone)]
pub enum ResolvedProtocol {
  /// HTTP/1.1 over TCP at `host:port`.
  Http1 {
    /// Bind host.
    host: String,
    /// Bind port.
    port: u16,
  },
  /// HTTP/2 over TCP at `host:port`.
  Http2 {
    /// Bind host.
    host: String,
    /// Bind port.
    port: u16,
  },
  /// HTTP/3 (QUIC) over TLS at `host:port`.
  Http3 {
    /// Bind host.
    host: String,
    /// Bind port.
    port: u16,
    /// Path to the TLS private key.
    ssl_key_path: String,
    /// Path to the TLS certificate.
    ssl_crt_path: String,
  },
  /// Impulse Ring shared-memory listener.
  ImpulseRing {
    /// Application name on the Ring bus.
    app_name: String,
    /// Optional access key.
    access_key: Option<String>,
  },
}

/// Server generic configuration.
#[derive(Clone, Deserialize, Default)]
pub struct GenericValues {
  /// Application name.
  ///
  /// You're not needed to write it in YAML configuration, instead you should send it to `load_generic_config` function.
  #[serde(skip)]
  pub app_name: String,

  /// The set of protocols to listen on simultaneously. Must be non-empty.
  ///
  /// This is the single way to choose how the server listens; see
  /// [`ProtocolConfig`].
  #[serde(default)]
  pub protocols: Vec<ProtocolConfig>,

  /// If you want to run any migration or anything else just before server's start, set to path to binary.
  pub auto_migrate_bin: Option<String>,

  #[cfg(feature = "cors")]
  /// CORS allowed domains
  pub allow_cors_domain: Option<String>,

  #[cfg(feature = "oapi")]
  /// Set this to `true` to enable OpenAPI endpoint.
  pub allow_oapi_access: Option<bool>,
  #[cfg(feature = "oapi")]
  /// Select `Scalar` or `SwaggerUI`.
  pub oapi_frontend_type: Option<String>,
  #[cfg(feature = "oapi")]
  /// By default, equals `app_name`; consider give expanded API name.
  pub oapi_name: Option<String>,
  #[cfg(feature = "oapi")]
  /// API version.
  pub oapi_ver: Option<String>,
  #[cfg(feature = "oapi")]
  /// API endpoint (with slash), e.g. `/api` or `/swagger`.
  pub oapi_api_addr: Option<String>,

  #[cfg(feature = "leptos-ssr")]
  /// Path to the front-end dist directory (the folder produced by
  /// `cargo-leptos` / `trunk`). The SSR handler serves every file under this
  /// directory at the URL that mirrors its on-disk path; unknown paths fall
  /// through to the SSR renderer.
  ///
  /// Resolution order (first hit wins): `IMPULSE_FRONTEND_DIST` env var,
  /// this field, `./dist`, `/usr/local/frontend-dist`.
  pub frontend_dist_path: Option<PathBuf>,
  #[cfg(feature = "leptos-ssr")]
  /// Output bundle name (matches `cargo-leptos`'s `output-name`). Used to
  /// build URLs for the wasm/JS/CSS bundle, e.g. `/pkg/<name>.js`.
  pub leptos_output_name: Option<String>,
  #[cfg(feature = "leptos-ssr")]
  /// Reserved for the upcoming server-functions support. Defaults to
  /// `/api/leptos`.
  pub leptos_server_fn_prefix: Option<String>,
  #[cfg(feature = "leptos-ssr")]
  /// SEO defaults injected as a Leptos context during SSR.
  pub leptos_seo: Option<crate::leptos_ssr::SeoDefaults>,

  /// Security response headers applied to every response when the
  /// router is built via `get_root_router_autoinject`. See
  /// [`crate::security_headers::SecurityHeadersOptions`].
  #[serde(default)]
  pub security_headers: crate::security_headers::SecurityHeadersOptions,

  #[serde(flatten)]
  /// Tracing options
  pub tracing_options: TracingOptions,
}

/// Server state.
#[derive(Clone)]
pub struct GenericServerState {
  /// The resolved, ready-to-serve set of protocols.
  pub protocols: Vec<ResolvedProtocol>,
  /// File log guard; needed to be handled the entire time the application is running.
  pub _guards: Arc<TracingGuards>,
}

impl GenericServerState {
  /// `true` if any resolved protocol speaks HTTP/3 (QUIC), so the HTTP/2→HTTP/3
  /// upgrade header should be installed.
  pub fn uses_http3(&self) -> bool {
    self
      .protocols
      .iter()
      .any(|p| matches!(p, ResolvedProtocol::Http3 { .. }))
  }

  /// The advertised HTTP/3 port, if any protocol serves QUIC. Used to build the
  /// `alt-svc` upgrade header on the cleartext listeners.
  pub fn http3_port(&self) -> Option<u16> {
    self.protocols.iter().find_map(|p| match p {
      ResolvedProtocol::Http3 { port, .. } => Some(*port),
      _ => None,
    })
  }

  /// `true` if every resolved protocol is shared-memory (`impulse-ring`), i.e.
  /// there are no network listeners at all.
  pub fn is_shared_memory_only(&self) -> bool {
    !self.protocols.is_empty()
      && self
        .protocols
        .iter()
        .all(|p| matches!(p, ResolvedProtocol::ImpulseRing { .. }))
  }
}

/// Loads the config from YAML file (`{app_name}.yaml`).
pub async fn load_generic_config<T: DeserializeOwned + GenericSetup + Default>(app_name: &str) -> MResult<T> {
  let mut file = std::fs::File::open(format!("{app_name}.yaml"));
  if file.is_err() {
    file = std::fs::File::open(format!("/etc/{app_name}.yaml"));
  }
  let mut file = file.map_err(|e| {
    ServerError::from_private(e)
      .with_public("The server configuration could not be found.")
      .with_500()
  })?;

  let mut buffer = String::new();
  file.read_to_string(&mut buffer).map_err(|e| {
    ServerError::from_private(e)
      .with_public("Failed to read the contents of the server configuration file.")
      .with_500()
  })?;
  let mut config: T = serde_pretty_yaml::from_str(&buffer).map_err(|e| {
    ServerError::from_private(e)
      .with_public("Failed to parse the contents of the server configuration file.")
      .with_500()
  })?;

  let data = config.generic_values_mut();
  data.app_name = app_name.to_string();

  #[cfg(feature = "oapi")]
  if data.allow_oapi_access.is_some_and(|v| v) {
    if data.oapi_name.is_none() {
      ServerError::from_public("The API name for OAPI is not specified.")
        .with_500()
        .bail()?;
    }
    if data.oapi_ver.is_none() {
      ServerError::from_public("The API version for OAPI is not specified.")
        .with_500()
        .bail()?;
    }
    if data.oapi_api_addr.is_none() {
      ServerError::from_public("The path to OAPI was not specified.")
        .with_500()
        .bail()?;
    }
  }

  Ok(config)
}

/// Loads the server's state: initializes the logging and checks YAML config for misconfigurations and errors.
///
/// You should call this function only once at startup because of logging setup.
pub async fn load_generic_state<T: GenericSetup>(setup: &T, init_logging: bool) -> MResult<GenericServerState> {
  let data = setup.generic_values();

  let guards = if init_logging {
    data.tracing_options.init(&data.app_name)?
  } else {
    Default::default()
  };

  let protocols = resolve_protocols(&data.protocols)?;
  if protocols.is_empty() {
    ServerError::from_public(
      "No listening protocols configured. Set a non-empty `protocols:` list (see the `ProtocolConfig` docs).",
    )
    .with_500()
    .bail()?;
  }

  Ok(GenericServerState {
    protocols,
    _guards: Arc::new(guards),
  })
}

/// Validate and lower every [`ProtocolConfig`] into a [`ResolvedProtocol`],
/// rejecting protocols whose Cargo feature is disabled.
fn resolve_protocols(protocols: &[ProtocolConfig]) -> MResult<Vec<ResolvedProtocol>> {
  let mut resolved = Vec::with_capacity(protocols.len());
  for proto in protocols {
    let r = match proto.clone() {
      ProtocolConfig::Http1 { host, port } => ResolvedProtocol::Http1 { host, port },
      ProtocolConfig::Http2 { host, port } => ResolvedProtocol::Http2 { host, port },
      ProtocolConfig::Http3 {
        host,
        port,
        ssl_key_path,
        ssl_crt_path,
      } => {
        #[cfg(not(feature = "http3"))]
        {
          let _ = (&host, &port, &ssl_key_path, &ssl_crt_path);
          ServerError::from_public("The `http3` protocol requires the `http3` feature of `impulse-server-kit`.")
            .with_500()
            .bail()?
        }
        #[cfg(feature = "http3")]
        ResolvedProtocol::Http3 {
          host,
          port,
          ssl_key_path,
          ssl_crt_path,
        }
      }
      ProtocolConfig::ImpulseRing { app_name, access_key } => {
        #[cfg(not(feature = "impulse-ring"))]
        {
          let _ = (&app_name, &access_key);
          ServerError::from_public(
            "The `impulse-ring` protocol requires the `impulse-ring` feature of `impulse-server-kit`.",
          )
          .with_500()
          .bail()?
        }
        #[cfg(feature = "impulse-ring")]
        ResolvedProtocol::ImpulseRing { app_name, access_key }
      }
    };
    resolved.push(r);
  }
  Ok(resolved)
}
