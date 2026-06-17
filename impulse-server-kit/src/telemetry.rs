//! Telemetry collection.
//!
//! Ingests batches of [`TelemetryEvent`]s produced by the client toolkit
//! (`impulse-client-kit`'s `telemetry` module) and hands each event to a
//! [`TelemetrySink`]. A sink decides what to do with events — persist them to a
//! database, forward them to a message broker, or simply surface them through
//! the existing tracing/OpenTelemetry stack.
//!
//! The default [`TracingTelemetrySink`] reuses Server Kit's observability layer:
//! every event is emitted as a `tracing` event and, when the `otel` feature is
//! on, counted as an OpenTelemetry metric — so client-side telemetry flows into
//! the same Jaeger/Prometheus pipelines as server metrics with zero extra wiring.
//!
//! # Wiring
//!
//! ```rust,ignore
//! use impulse_server_kit::prelude::*;
//! use impulse_server_kit::telemetry::default_telemetry_router;
//!
//! let router = get_root_router_autoinject(&state, setup.clone())
//!   // Collects POSTed MessagePack/JSON batches at `/api/telemetry`,
//!   // emitting them to tracing/OpenTelemetry.
//!   .push(default_telemetry_router("api/telemetry"));
//! ```
//!
//! To persist events yourself, implement [`TelemetrySink`] and pass it to
//! [`telemetry_router`].

use crate::prelude::*;
use std::sync::Arc;

pub use impulse_utils::telemetry::{TelemetryAttr, TelemetryBatch, TelemetryEvent, TelemetryEventKind, TelemetryLevel};

/// Per-request context made available to a [`TelemetrySink`].
///
/// Carries server-observed metadata that the client cannot be trusted to report
/// (or simply does not know), such as the connecting peer address.
#[derive(Debug, Clone, Default)]
pub struct TelemetryRequestCtx {
  /// Remote peer address as seen by the server.
  pub remote_addr: Option<String>,
  /// `User-Agent` header, if present.
  pub user_agent: Option<String>,
}

impl TelemetryRequestCtx {
  /// Extract collection context from an incoming request.
  pub fn from_request(req: &Request) -> Self {
    Self {
      remote_addr: Some(req.remote_addr().to_string()),
      user_agent: req.header::<String>("user-agent"),
    }
  }
}

/// A consumer of collected telemetry events.
///
/// Implement this and inject an `Arc<dyn TelemetrySink>` (via [`telemetry_router`])
/// to control how events are stored or forwarded. Implementations must be cheap
/// to clone behind an `Arc` and safe to call concurrently.
#[salvo::async_trait]
pub trait TelemetrySink: Send + Sync + 'static {
  /// Handle a single telemetry event.
  async fn record(&self, event: &TelemetryEvent, ctx: &TelemetryRequestCtx);
}

/// Stable string name for a telemetry event kind, used as a metric/label value.
fn kind_str(kind: TelemetryEventKind) -> &'static str {
  match kind {
    TelemetryEventKind::Click => "click",
    TelemetryEventKind::View => "view",
    TelemetryEventKind::Hover => "hover",
    TelemetryEventKind::Focus => "focus",
    TelemetryEventKind::Submit => "submit",
    TelemetryEventKind::Custom => "custom",
    TelemetryEventKind::PageView => "page_view",
    TelemetryEventKind::Log => "log",
    TelemetryEventKind::Metric => "metric",
    TelemetryEventKind::Span => "span",
  }
}

/// Default sink that forwards events into the `tracing` / OpenTelemetry stack.
///
/// - Every event is emitted as a `tracing` event under the `client_telemetry`
///   target; [`TelemetryEventKind::Log`] events use their reported severity,
///   everything else is logged at `INFO`.
/// - With the `otel` feature enabled, every event also increments the
///   `client_telemetry_events` counter (labelled by `kind`/`path`), and
///   [`TelemetryEventKind::Metric`]/[`TelemetryEventKind::Span`] values are
///   recorded into the `client_telemetry_values` histogram.
#[derive(Debug, Clone, Copy, Default)]
pub struct TracingTelemetrySink;

#[salvo::async_trait]
impl TelemetrySink for TracingTelemetrySink {
  async fn record(&self, event: &TelemetryEvent, ctx: &TelemetryRequestCtx) {
    let kind = kind_str(event.kind);
    let message = event.message.as_deref().unwrap_or_default();
    let session_id = event.session_id.as_deref().unwrap_or_default();
    let user_id = event.user_id.as_deref().unwrap_or_default();
    let path = event.path.as_deref().unwrap_or_default();
    let remote = ctx.remote_addr.as_deref().unwrap_or_default();

    macro_rules! emit {
      ($lvl:ident) => {
        tracing::$lvl!(
          target: "client_telemetry",
          kind,
          message,
          session_id,
          user_id,
          path,
          value = event.value,
          remote,
          "client telemetry event"
        )
      };
    }

    match (event.kind, event.level) {
      (TelemetryEventKind::Log, Some(TelemetryLevel::Trace)) => emit!(trace),
      (TelemetryEventKind::Log, Some(TelemetryLevel::Debug)) => emit!(debug),
      (TelemetryEventKind::Log, Some(TelemetryLevel::Warn)) => emit!(warn),
      (TelemetryEventKind::Log, Some(TelemetryLevel::Error)) => emit!(error),
      _ => emit!(info),
    }

    #[cfg(feature = "otel")]
    {
      use crate::otel::api::KeyValue;

      let meter = crate::otel::api::global::meter("client_telemetry");
      let mut attrs = vec![KeyValue::new("kind", kind), KeyValue::new("path", path.to_string())];
      for attr in &event.attributes {
        attrs.push(KeyValue::new(attr.key.clone(), attr.value.clone()));
      }

      meter
        .u64_counter("client_telemetry_events")
        .with_unit("1")
        .with_description("Total number of telemetry events received from clients")
        .build()
        .add(1, &attrs);

      if let Some(value) = event.value
        && matches!(event.kind, TelemetryEventKind::Metric | TelemetryEventKind::Span)
      {
        meter
          .f64_histogram("client_telemetry_values")
          .with_description("Numeric values reported by client metric/span telemetry events")
          .build()
          .record(value, &attrs);
      }
    }
  }
}

/// Parse a telemetry batch from the request body.
///
/// MessagePack is the canonical client transport; JSON is also accepted (handy
/// for debugging and non-Rust clients) based on the `Content-Type`.
async fn parse_batch(req: &mut Request) -> MResult<TelemetryBatch> {
  let is_msgpack = req
    .content_type()
    .is_some_and(|ct| ct.subtype() == salvo::http::mime::MSGPACK);
  if is_msgpack {
    req.parse_msgpack::<TelemetryBatch>().await
  } else {
    req.parse_json_simd::<TelemetryBatch>().await
  }
}

/// Telemetry collection endpoint.
///
/// Accepts a POSTed [`TelemetryBatch`] (MessagePack or JSON), then dispatches
/// every event to the `Arc<dyn TelemetrySink>` injected into the depot, falling
/// back to [`TracingTelemetrySink`] when none is present.
#[handler]
pub async fn collect_telemetry(req: &mut Request, depot: &mut Depot) -> MResult<OK> {
  let batch = parse_batch(req).await?;
  let ctx = TelemetryRequestCtx::from_request(req);

  if let Ok(sink) = depot.obtain::<Arc<dyn TelemetrySink>>() {
    let sink = sink.clone();
    for event in &batch.events {
      sink.record(event, &ctx).await;
    }
  } else {
    let sink = TracingTelemetrySink;
    for event in &batch.events {
      sink.record(event, &ctx).await;
    }
  }

  ok!()
}

/// Build a router that collects telemetry at `path` using the provided sink.
///
/// The sink is injected into the depot, so [`collect_telemetry`] picks it up for
/// every request handled by this sub-router.
pub fn telemetry_router(path: impl Into<String>, sink: Arc<dyn TelemetrySink>) -> Router {
  Router::with_path(path.into())
    .hoop(salvo::affix_state::inject(sink))
    .post(collect_telemetry)
}

/// Build a telemetry collection router backed by [`TracingTelemetrySink`].
pub fn default_telemetry_router(path: impl Into<String>) -> Router {
  telemetry_router(path, Arc::new(TracingTelemetrySink))
}
