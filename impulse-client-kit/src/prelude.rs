//! Client Kit prelude.

pub use impulse_utils;
pub use leptos::prelude::*;
pub use log;

#[cfg(any(feature = "csr", feature = "hydrate"))]
pub use console_error_panic_hook;
#[cfg(any(feature = "csr", feature = "hydrate"))]
pub use console_log;

pub use crate::setup_app;

#[cfg(feature = "telemetry")]
pub use crate::telemetry::{
  ClickMonitor, EventMonitor, FocusMonitor, HoverMonitor, SubmitMonitor, TelemetryConfig, TelemetryContext,
  TelemetryEventKind, TelemetryLevel, TelemetryMode, TelemetrySpan, ViewMonitor, provide_telemetry, track_event,
  track_log, track_metric, track_span, use_telemetry,
};

#[cfg(feature = "ssr")]
pub use crate::ssr::{InitialTheme, LeptosResponseOptions, RequestUrlCtx};
#[cfg(feature = "ssr")]
pub use leptos_meta;

#[cfg(any(feature = "websocket", feature = "webtransport"))]
pub use crate::reconnect::ReconnectOptions;
#[cfg(feature = "websocket")]
pub use crate::ws::{
  WebSocketHandle, WebSocketMessage, WebSocketOptions, WebSocketReadyState, use_websocket, use_websocket_with_options,
  use_websocket_with_protocols,
};
#[cfg(feature = "webtransport")]
pub use crate::wt::{
  WebTransportHandle, WebTransportState, use_webtransport, use_webtransport_with_options,
  use_webtransport_with_options_and_reconnect, use_webtransport_with_reconnect,
};
