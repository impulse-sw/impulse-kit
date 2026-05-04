//! UI Kit prelude.

pub use console_error_panic_hook;
pub use impulse_utils;
pub use leptos::prelude::*;

pub use console_log;
pub use log;

pub use crate::setup_app;

#[cfg(feature = "websocket")]
pub use crate::ws::{
  WebSocketHandle, WebSocketMessage, WebSocketReadyState, use_websocket, use_websocket_with_protocols,
};
#[cfg(feature = "webtransport")]
pub use crate::wt::{WebTransportHandle, WebTransportState, use_webtransport, use_webtransport_with_options};
