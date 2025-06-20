//! Setup module.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use cc_utils::prelude::*;

pub mod port_achiever;
pub mod tracing_init;

use crate::setup::port_achiever::port_file_watcher;
use crate::setup::tracing_init::{TracingGuards, TracingOptions};

/// Provides at least values needed by Server Kit to start.
pub trait GenericSetup {
  /// Provides generic values; see `GenericValues`.
  fn generic_values(&self) -> &GenericValues;
  /// Provides mutable generic values; see `GenericValues`.
  fn generic_values_mut(&mut self) -> &mut GenericValues;
}

/// Server startup variants.
///
/// These are the hardcoded variants; by default, `salvo` can much more than this.
#[derive(Clone, Eq, PartialEq)]
pub enum StartupVariant {
  /// Will listen `http://127.0.0.1:{port}` only.
  HttpLocalhost,
  /// Will listen `http://{host}:{port}`. Not recommended.
  UnsafeHttp,
  #[cfg(feature = "acme")]
  /// Will listen `https://{host}:{port}` with automatic SSL certificate acquiring.
  HttpsAcme,
  #[cfg(all(feature = "http3", feature = "acme"))]
  /// Will listen `https|quic://{host}:{port}` with automatic SSL certificate acquiring.
  QuinnAcme,
  /// Will listen `https://{host}:{port}` with your SSL cert and key.
  HttpsOnly,
  #[cfg(feature = "http3")]
  /// Will listen `https|quic://{host}:{port}` with your SSL cert and key.
  Quinn,
  #[cfg(feature = "http3")]
  /// Will listen `quic://{host}:{port}` only, with your SSL cert and key.
  QuinnOnly,
}

/// Server generic configuration.
#[derive(Clone, Deserialize, Default)]
pub struct GenericValues {
  /// Application name.
  ///
  /// You're not needed to write it in YAML configuration, instead you should send it to `load_generic_config` function.
  #[serde(skip)]
  pub app_name: String,
  /// Startup variant. Converts to `StartupVariant`.
  pub startup_type: String,
  /// Server host.
  pub server_host: Option<String>,
  /// Server port. For no reverse proxy and Internet usage, set to `80` for HTTP and `443` for HTTPS/QUIC.
  pub server_port: Option<u16>,
  /// ACME origin; see [`salvo/conn/acme` docs](https://docs.rs/salvo/latest/salvo/conn/acme/index.html).
  pub acme_domain: Option<String>,
  /// Path to SSL key.
  pub ssl_key_path: Option<String>,
  /// Path to SSL certificate.
  pub ssl_crt_path: Option<String>,
  /// If you want to run any migration or anything else just before server's start, set to path to binary.
  pub auto_migrate_bin: Option<String>,
  /// Use text file to find out which port to listen to.
  pub server_port_achiever: Option<PathBuf>,

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

  #[serde(flatten)]
  /// Tracing options
  pub tracing_options: TracingOptions,
}

/// Server state.
#[derive(Clone)]
pub struct GenericServerState {
  /// Converted startup variant, ready to launch.
  pub startup_variant: StartupVariant,
  /// File log guard; needed to be handled the entire time the application is running.
  pub _guards: Arc<TracingGuards>,
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
  let mut config: T = serde_yaml::from_str(&buffer).map_err(|e| {
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

  if let Some(achiever) = &data.server_port_achiever {
    let port = port_file_watcher(achiever.as_path()).await?;
    data.server_port = Some(port);
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

  let state = GenericServerState {
    startup_variant: match &*data.startup_type {
      "http_localhost" => {
        if data.server_host.is_some() {
          ServerError::from_public("Server will only listen `127.0.0.1` address because of `http_localhost` startup variant. Consider to move to `https_only` or `quinn`.").with_500().bail()?;
        }
        StartupVariant::HttpLocalhost
      }
      "unsafe_http" => {
        if data.server_host.is_none() {
          ServerError::from_public("Choose server's host, e.g. `0.0.0.0`.")
            .with_500()
            .bail()?;
        }
        StartupVariant::UnsafeHttp
      }
      #[cfg(feature = "acme")]
      "https_acme" => {
        if data.server_host.is_none() {
          ServerError::from_public("Choose server's host, e.g. `0.0.0.0`.")
            .with_500()
            .bail()?;
        }
        if data.acme_domain.is_none() {
          ServerError::from_public("Choose ACME's domain!").with_500().bail()?;
        }
        StartupVariant::HttpsAcme
      }
      "https_only" => {
        if data.server_host.is_none() {
          ServerError::from_public("Choose server's host, e.g. `0.0.0.0`.")
            .with_500()
            .bail()?;
        }
        if data.ssl_key_path.is_none() {
          ServerError::from_public("Choose SSL key path.").with_500().bail()?;
        }
        if data.ssl_crt_path.is_none() {
          ServerError::from_public("Choose SSL cert path.").with_500().bail()?;
        }
        StartupVariant::HttpsOnly
      }
      #[cfg(all(feature = "http3", feature = "acme"))]
      "quinn_acme" => {
        if data.server_host.is_none() {
          ServerError::from_public("Choose server's host, e.g. `0.0.0.0`.")
            .with_500()
            .bail()?;
        }
        if data.acme_domain.is_none() {
          ServerError::from_public("Choose ACME's domain!").with_500().bail()?;
        }
        StartupVariant::QuinnAcme
      }
      #[cfg(feature = "http3")]
      "quinn" => {
        if data.server_host.is_none() {
          ServerError::from_public("Choose server's host, e.g. `0.0.0.0`.")
            .with_500()
            .bail()?;
        }
        if data.ssl_key_path.is_none() {
          ServerError::from_public("Choose SSL key path.").with_500().bail()?;
        }
        if data.ssl_crt_path.is_none() {
          ServerError::from_public("Choose SSL cert path.").with_500().bail()?;
        }
        StartupVariant::Quinn
      }
      #[cfg(feature = "http3")]
      "quinn_only" => {
        if data.server_host.is_none() {
          ServerError::from_public("Choose server's host, e.g. `0.0.0.0`.")
            .with_500()
            .bail()?;
        }
        if data.ssl_key_path.is_none() {
          ServerError::from_public("Choose SSL key path.").with_500().bail()?;
        }
        if data.ssl_crt_path.is_none() {
          ServerError::from_public("Choose SSL cert path.").with_500().bail()?;
        }
        StartupVariant::QuinnOnly
      }
      _ => ServerError::from_public(
        "The server deployment method could not be determined. Read the documentation on the `startup_variant` field.",
      )
      .with_500()
      .bail()?,
    },
    _guards: Arc::new(guards),
  };
  Ok(state)
}
