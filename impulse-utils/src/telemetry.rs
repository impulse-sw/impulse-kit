//! Shared telemetry wire types.
//!
//! These types describe the payload exchanged between the client toolkit
//! (`impulse-client-kit`, which produces events in the browser) and the server
//! toolkit (`impulse-server-kit`, which ingests them through a collection
//! endpoint). They are intentionally platform-agnostic — only `serde` is used,
//! so the very same definitions compile on `wasm32` and on the server.
//!
//! The canonical transport is MessagePack (compact, cheap for high-frequency
//! events), but since these are plain `serde` types they serialize to JSON just
//! as well, which is handy for debugging.
//!
//! Enable with the `telemetry` feature.

use serde::{Deserialize, Serialize};

/// What kind of interaction a [`TelemetryEvent`] describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryEventKind {
  /// A click on a monitored element.
  Click,
  /// A monitored element became visible in the viewport (impression).
  View,
  /// The pointer entered a monitored element.
  Hover,
  /// A monitored element (or one of its children) received focus.
  Focus,
  /// A monitored form was submitted.
  Submit,
  /// An arbitrary DOM event captured by a generic monitor.
  Custom,
  /// A page/route view.
  PageView,
  /// A free-form structured log line emitted imperatively.
  Log,
  /// A single numeric measurement emitted imperatively.
  Metric,
  /// A timed span; [`TelemetryEvent::value`] carries its duration in milliseconds.
  Span,
}

/// Severity for [`TelemetryEventKind::Log`] events, mirroring `tracing`/`log` levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryLevel {
  /// Verbose, fine-grained diagnostic information.
  Trace,
  /// Diagnostic information useful while debugging.
  Debug,
  /// Informational, expected events.
  Info,
  /// Something unexpected that is still recoverable.
  Warn,
  /// An error condition.
  Error,
}

/// A single key/value attribute attached to a [`TelemetryEvent`].
///
/// A `Vec` of these is used instead of a map so the wire format stays stable and
/// trivially (de)serializable in MessagePack without relying on map ordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryAttr {
  /// Attribute name.
  pub key: String,
  /// Attribute value, always stringified to keep the schema flat.
  pub value: String,
}

impl TelemetryAttr {
  /// Build an attribute from anything string-like.
  pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
    Self {
      key: key.into(),
      value: value.into(),
    }
  }
}

/// A single telemetry event.
///
/// Identity handling is deliberately explicit: [`Self::session_id`] is an
/// anonymous, session-scoped identifier, while [`Self::user_id`] is only present
/// when the application has opted into identified collection. In anonymous mode
/// the client never fills `user_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetryEvent {
  /// The interaction kind.
  pub kind: TelemetryEventKind,
  /// Human-readable message describing the event (e.g. the `message` prop of a monitor).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub message: Option<String>,
  /// Severity, set for [`TelemetryEventKind::Log`] events.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub level: Option<TelemetryLevel>,
  /// Numeric payload: metric value for [`TelemetryEventKind::Metric`],
  /// duration in milliseconds for [`TelemetryEventKind::Span`].
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub value: Option<f64>,
  /// Path/route the event originated from.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
  /// Anonymous, session-scoped identifier. Stable for the lifetime of a page session.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub session_id: Option<String>,
  /// Identified user id. Only present in identified mode.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub user_id: Option<String>,
  /// Milliseconds since the Unix epoch when the event was produced.
  pub timestamp_ms: u64,
  /// Arbitrary extra attributes.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub attributes: Vec<TelemetryAttr>,
}

impl TelemetryEvent {
  /// Create a bare event of the given kind with a timestamp.
  ///
  /// Identity (`session_id`/`user_id`), `message`, `path` and `attributes` are
  /// expected to be filled in by the producing layer.
  pub fn new(kind: TelemetryEventKind, timestamp_ms: u64) -> Self {
    Self {
      kind,
      message: None,
      level: None,
      value: None,
      path: None,
      session_id: None,
      user_id: None,
      timestamp_ms,
      attributes: Vec::new(),
    }
  }

  /// Builder-style setter for the message.
  pub fn with_message(mut self, message: impl Into<String>) -> Self {
    self.message = Some(message.into());
    self
  }

  /// Builder-style setter for the numeric value.
  pub fn with_value(mut self, value: f64) -> Self {
    self.value = Some(value);
    self
  }

  /// Builder-style setter for the severity level.
  pub fn with_level(mut self, level: TelemetryLevel) -> Self {
    self.level = Some(level);
    self
  }

  /// Builder-style setter for the originating path.
  pub fn with_path(mut self, path: impl Into<String>) -> Self {
    self.path = Some(path.into());
    self
  }

  /// Append an attribute.
  pub fn with_attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.attributes.push(TelemetryAttr::new(key, value));
    self
  }
}

/// A batch of telemetry events, the unit of transport for the collection endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TelemetryBatch {
  /// The collected events.
  pub events: Vec<TelemetryEvent>,
}

impl TelemetryBatch {
  /// Create an empty batch.
  pub fn new() -> Self {
    Self::default()
  }

  /// Create a batch from a single event.
  pub fn single(event: TelemetryEvent) -> Self {
    Self { events: vec![event] }
  }

  /// Whether the batch carries no events.
  pub fn is_empty(&self) -> bool {
    self.events.is_empty()
  }
}
