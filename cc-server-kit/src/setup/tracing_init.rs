//! Tracing initializer module.

use cc_utils::prelude::*;
use serde::Deserialize;
use tracing_appender::non_blocking::WorkerGuard as TracingFileGuard;
use tracing_rfc_5424::transport::{
  Error as SyslogError, TcpTransport, Transport, UdpTransport, UnixSocket, UnixSocketStream,
};

#[derive(Clone, Deserialize, Default)]
/// Tracing options.
pub struct TracingOptions {
  /// Enable I/O logs.
  pub enable_io_logs: Option<bool>,
  /// Log level; for no logging delete the line in YAML completely.
  pub io_log_level: Option<String>,

  /// Enable file logs.
  pub enable_file_logs: Option<bool>,
  /// File's log level. Defaults to `log_level`.
  pub file_log_level: Option<String>,
  /// File rolling rotation, if you have a ton of logs and need to split them.
  pub file_log_rotation: Option<String>,
  /// Files limitation for autoremove.
  pub file_log_max_rolling_files: Option<u32>,

  /// Enable RFC 5424 logging.
  pub enable_syslog_logs: Option<bool>,
  /// Address to send logs via TCP/UDP or into UNIX sockets.
  pub syslog_addr: Option<String>,
  /// Syslog's log level. Defaults to `log_level`.
  pub syslog_log_level: Option<String>,

  /// Enable Elastic Common Schema structured logging.
  pub enable_ecs_logs: Option<bool>,
  /// ECS log level. Defaults to `log_level`.
  pub ecs_log_level: Option<String>,
  /// ECS rolling rotation, if you have a ton of ECS logs and need to split them.
  pub ecs_rotation: Option<String>,
  /// ECS files limitation for autoremove.
  pub ecs_max_rolling_files: Option<u32>,

  #[cfg(feature = "otel")]
  /// Endpoint to export OpenTelemetry via gRPC (e.g., Jaeger).
  pub otel_grpc_endpoint: Option<String>,
  #[cfg(feature = "otel")]
  /// Endpoint to export OpenTelemetry via HTTP binary protocol (e.g., Prometheus).
  pub otel_http_endpoint: Option<String>,
  #[cfg(feature = "otel")]
  /// OpenTelemetry log level. Defaults to `log_level`.
  pub otel_log_level: Option<String>,
}

#[derive(Default)]
#[allow(dead_code)]
/// Tracing guards.
///
/// Holds the file guards to write logs into them.
pub struct TracingGuards {
  file_log_guard: Option<TracingFileGuard>,
  ecs_log_guard: Option<TracingFileGuard>,
}

fn match_log_level(log_level: &Option<String>) -> MResult<tracing::Level> {
  if log_level.is_some() {
    Ok(match log_level.as_ref().unwrap().as_str() {
      "error" => tracing::Level::ERROR,
      "warn" => tracing::Level::WARN,
      "info" => tracing::Level::INFO,
      "debug" => tracing::Level::DEBUG,
      "trace" => tracing::Level::TRACE,
      _ => ServerError::from_public("Incorrect logging level.").with_500().bail()?,
    })
  } else if cfg!(debug_assertions) {
    Ok(tracing::Level::DEBUG)
  } else {
    ServerError::from_public("Logging is disabled").with_500().bail()
  }
}

fn match_log_file_rolling(log_rolling: &Option<String>) -> MResult<tracing_appender::rolling::Rotation> {
  if let Some(log_rolling) = log_rolling {
    Ok(match log_rolling.as_str() {
      "never" => tracing_appender::rolling::Rotation::NEVER,
      "daily" => tracing_appender::rolling::Rotation::DAILY,
      "hourly" => tracing_appender::rolling::Rotation::HOURLY,
      "minutely" => tracing_appender::rolling::Rotation::MINUTELY,
      _ => ServerError::from_public(
        "Incorrect level of log rotation. Choose one of the options: `never`, `daily`, `hourly`, `minutely`.",
      )
      .with_500()
      .bail()?,
    })
  } else {
    Ok(tracing_appender::rolling::Rotation::NEVER)
  }
}

enum SyslogTransportWrapper {
  Udp(UdpTransport),
  Tcp(TcpTransport),
  Unix(UnixSocket),
  UnixStream(UnixSocketStream),
}

impl<F: tracing_rfc_5424::formatter::SyslogFormatter> Transport<F> for SyslogTransportWrapper {
  type Error = SyslogError;

  fn send(&self, buf: F::Output) -> Result<(), Self::Error> {
    match self {
      SyslogTransportWrapper::Udp(t) => {
        <tracing_rfc_5424::transport::UdpTransport as tracing_rfc_5424::transport::Transport<F>>::send(t, buf)
      }
      SyslogTransportWrapper::Tcp(t) => {
        <tracing_rfc_5424::transport::TcpTransport as tracing_rfc_5424::transport::Transport<F>>::send(t, buf)
      }
      SyslogTransportWrapper::Unix(t) => {
        <tracing_rfc_5424::transport::UnixSocket as tracing_rfc_5424::transport::Transport<F>>::send(t, buf)
      }
      SyslogTransportWrapper::UnixStream(t) => {
        <tracing_rfc_5424::transport::UnixSocketStream as tracing_rfc_5424::transport::Transport<F>>::send(t, buf)
      }
    }
  }
}

fn match_syslog_addr(addr: &Option<String>) -> MResult<SyslogTransportWrapper> {
  use tracing_rfc_5424::transport::*;

  let addr = addr
    .as_ref()
    .ok_or(ServerError::from_public("Syslog export address is empty!").with_500())?;

  match addr.as_str() {
    s if s.starts_with("udp://") => UdpTransport::new(&s[6..])
      .map(SyslogTransportWrapper::Udp)
      .map_err(ServerError::from_private),
    s if s.starts_with("tcp://") => TcpTransport::new(&s[6..])
      .map(SyslogTransportWrapper::Tcp)
      .map_err(ServerError::from_private),
    s if s.starts_with("unix://") => UnixSocket::new(&s[7..])
      .map(SyslogTransportWrapper::Unix)
      .map_err(ServerError::from_private),
    s if s.starts_with("ustream://") => UnixSocketStream::new(&s[10..])
      .map(SyslogTransportWrapper::UnixStream)
      .map_err(ServerError::from_private),
    _ => Err(ServerError::from_public(
      "Can't init syslog because of incorrect address; your address should start with `udp://`, `tcp://`, `unix://` or `ustream://`.",
    )),
  }
}

#[allow(dead_code)]
fn log_filter(metadata: &tracing::Metadata) -> bool {
  #[cfg(not(feature = "log-without-filtering"))]
  {
    metadata.module_path().is_none_or(|p| {
      !(p.contains("salvo")
        || p.contains("hyper_util")
        || p.contains("tower")
        || p.contains("quinn")
        || p.contains("h2"))
    })
  }
  #[cfg(feature = "log-without-filtering")]
  {
    true
  }
}

impl TracingOptions {
  /// Inits logging application-wide.
  pub fn init(&self, app_name: &str) -> MResult<TracingGuards> {
    use tracing_appender::rolling;
    #[allow(unused_imports)]
    use tracing_subscriber::filter::{LevelFilter, filter_fn};
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, registry};

    #[cfg(feature = "otel")]
    use crate::otel::api::trace::TracerProvider;
    #[cfg(feature = "otel")]
    use crate::otel::exporter::WithExportConfig;
    #[cfg(feature = "otel")]
    use crate::otel::sdk::{Resource, trace::RandomIdGenerator};

    let format = fmt::format()
      .with_level(true)
      .with_target(true)
      .with_thread_ids(false)
      .with_thread_names(false)
      .with_file(false)
      .with_line_number(true)
      .compact();

    let io_tracer = if self.enable_io_logs.is_some_and(|v| v) {
      let io_log_level = match_log_level(&self.io_log_level)?;

      let io_tracer = fmt::layer()
        .event_format(format.clone())
        .with_writer(std::io::stdout)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(LevelFilter::from_level(io_log_level))
        .with_filter(filter_fn(log_filter));
      Some(io_tracer)
    } else {
      None
    };

    let (file_tracer, file_log_guard) = if self.enable_file_logs.is_some_and(|v| v) {
      let file_log_level = match_log_level(&self.file_log_level).or_else(|_| match_log_level(&self.io_log_level))?;
      let file_log_rotation = match_log_file_rolling(&self.file_log_rotation)?;
      let file_log_max_rolling_files = self.file_log_max_rolling_files.unwrap_or(5) as usize;

      let file_appender = rolling::RollingFileAppender::builder()
        .rotation(file_log_rotation)
        .filename_suffix(app_name)
        .max_log_files(file_log_max_rolling_files)
        .build("logs")
        .map_err(|e| {
          ServerError::from_private(e)
            .with_public("Failed to initialize logging to file!")
            .with_500()
        })?;
      let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

      let file_tracer = fmt::layer()
        .event_format(format)
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(LevelFilter::from_level(file_log_level))
        .with_filter(filter_fn(log_filter));

      (Some(file_tracer), Some(guard))
    } else {
      (None, None)
    };

    let syslog_tracer = if self.enable_syslog_logs.is_some_and(|v| v) {
      let syslog_log_level =
        match_log_level(&self.syslog_log_level).or_else(|_| match_log_level(&self.io_log_level))?;
      let transport = match_syslog_addr(&self.syslog_addr)?;

      let format = tracing_rfc_5424::rfc5424::Rfc5424::builder()
        .appname_as_string(app_name.to_string())
        .map_err(ServerError::from_private)?
        .facility(tracing_rfc_5424::facility::Facility::LOG_USER)
        .build();
      let syslog_tracer = tracing_rfc_5424::layer::Layer::with_transport_and_syslog_formatter(transport, format)
        .with_filter(LevelFilter::from_level(syslog_log_level))
        .with_filter(filter_fn(log_filter));

      Some(syslog_tracer)
    } else {
      None
    };

    let (ecs_tracer, ecs_log_guard) = if self.enable_ecs_logs.is_some_and(|v| v) {
      let ecs_log_level = match_log_level(&self.ecs_log_level).or_else(|_| match_log_level(&self.io_log_level))?;
      let ecs_log_rotation = match_log_file_rolling(&self.ecs_rotation)?;
      let ecs_log_max_rolling_files = self.ecs_max_rolling_files.unwrap_or(5) as usize;

      let file_appender = rolling::RollingFileAppender::builder()
        .rotation(ecs_log_rotation)
        .filename_suffix(app_name)
        .max_log_files(ecs_log_max_rolling_files)
        .build("ecs-logs")
        .map_err(|e| {
          ServerError::from_private(e)
            .with_public("Failed to initialize ECS logging to file!")
            .with_500()
        })?;
      let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

      let ecs_tracer = tracing_ecs::ECSLayerBuilder::default()
        .normalize_json(false)
        .with_span_events(FmtSpan::CLOSE)
        .build_with_writer(non_blocking)
        .with_filter(LevelFilter::from_level(ecs_log_level))
        .with_filter(filter_fn(log_filter));

      (Some(ecs_tracer), Some(guard))
    } else {
      (None, None)
    };

    #[cfg(feature = "otel")]
    let otel_tracer = if let Some(otel_grpc_endpoint) = &self.otel_grpc_endpoint {
      let otel_log_level = match_log_level(&self.otel_log_level).or_else(|_| match_log_level(&self.io_log_level))?;

      let otel_span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_protocol(opentelemetry_otlp::Protocol::Grpc)
        .with_endpoint(otel_grpc_endpoint.as_str())
        .with_timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| {
          ServerError::from_private(e)
            .with_public("Failed to initialize OTEL gRPC telemetry!")
            .with_500()
        })?;
      let otel_tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(otel_span_exporter)
        .with_id_generator(RandomIdGenerator::default())
        .with_max_events_per_span(32)
        .with_max_attributes_per_span(64)
        .with_resource(Resource::builder().with_service_name(app_name.to_string()).build())
        .build()
        .tracer(app_name.to_owned());

      let opentelemetry = tracing_opentelemetry::layer()
        .with_tracer(otel_tracer_provider)
        .with_filter(LevelFilter::from_level(otel_log_level))
        .with_filter(filter_fn(log_filter));

      Some(opentelemetry)
    } else {
      None
    };

    #[cfg(feature = "otel")]
    if let Some(otel_http_endpoint) = &self.otel_http_endpoint {
      use opentelemetry_otlp::WithHttpConfig;

      let otel_metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_http_client(reqwest::blocking::Client::new())
        .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
        .with_endpoint(otel_http_endpoint.as_str())
        .with_timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| {
          ServerError::from_private(e)
            .with_public("Failed to initialize OTEL HTTP telemetry!")
            .with_500()
        })?;
      let otel_metric_reader = opentelemetry_sdk::metrics::PeriodicReader::builder(otel_metric_exporter)
        .with_interval(std::time::Duration::from_secs(5))
        .build();
      let meter_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_resource(Resource::builder().with_service_name(app_name.to_string()).build())
        .with_reader(otel_metric_reader)
        .build();
      opentelemetry::global::set_meter_provider(meter_provider.clone());
    }

    let collector = registry()
      .with(file_tracer)
      .with(io_tracer)
      .with(syslog_tracer)
      .with(ecs_tracer);
    #[cfg(feature = "otel")]
    let collector = collector.with(otel_tracer);

    tracing::subscriber::set_global_default(collector).map_err(|e| {
      ServerError::from_private(e)
        .with_public("Can't init global default log collector!")
        .with_500()
    })?;

    Ok(TracingGuards {
      file_log_guard,
      ecs_log_guard,
    })
  }
}
