//! Tauri-IPC backend for the WebSocket surface, selected under `cfg(tauri)`.
//!
//! In a Tauri webview the app can't hold a WebSocket to an arbitrary host, so the
//! socket lives in the native engine. This module exposes the same shape as
//! [`crate::ws`] — a [`WebSocketHandle`] with reactive `state`/`message` signals
//! and `send_text` — but backed by Tauri IPC: outgoing text is forwarded with
//! `invoke("ik_ws_send", { text })`, and incoming frames arrive as the Tauri
//! event `ik_ws_message` (payload = the frame text). An app swaps transports with
//! a one-line `cfg` on the import; nothing else changes.
//!
//! The engine side registers the `ik_ws_send` command and emits `ik_ws_message`
//! events (see an app's BUNDLE.md). Requires `withGlobalTauri` so
//! `window.__TAURI__` is present.

use impulse_utils::prelude::{CResult, ClientError};
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

// The value types are transport-agnostic; reuse them so call sites are identical.
pub use crate::ws::{WebSocketMessage, WebSocketOptions, WebSocketReadyState};

#[wasm_bindgen]
extern "C" {
  // Tauri v2 global bindings (require `withGlobalTauri: true`).
  #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
  fn invoke(cmd: &str, args: JsValue) -> JsValue;

  #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
  fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;
}

/// IPC-backed handle mirroring [`crate::ws::WebSocketHandle`]'s public surface.
///
/// The channel to the engine is always considered `Open` — the engine itself
/// tracks server connectivity and serves offline, so from the UI's point of view
/// the transport never drops.
#[derive(Clone)]
pub struct WebSocketHandle {
  /// Ready state (always `Open` for the local IPC channel).
  pub state: ReadSignal<WebSocketReadyState>,
  /// The most recent inbound frame.
  pub message: ReadSignal<Option<WebSocketMessage>>,
}

impl WebSocketHandle {
  /// Forwards a text frame to the engine via `invoke("ik_ws_send", { text })`.
  pub fn send_text(&self, text: &str) -> CResult<()> {
    #[derive(serde::Serialize)]
    struct Args<'a> {
      text: &'a str,
    }
    let args = serde_wasm_bindgen::to_value(&Args { text })
      .map_err(|e| ClientError::from_str(format!("IPC encode failed: {e:?}")))?;
    // Fire-and-forget: the command runs; we don't await its promise.
    let _ = invoke("ik_ws_send", args);
    Ok(())
  }

  /// No-op: the IPC channel isn't closed by the UI (the engine owns its lifecycle).
  pub fn close(&self) -> CResult<()> {
    Ok(())
  }
}

/// Opens the IPC channel to the engine. The URL `provider` and `options` are
/// accepted for signature parity with [`crate::ws::use_websocket_with_url_fn`]
/// but ignored here — the engine is the peer, not a URL.
pub fn use_websocket_with_url_fn<F, Fut>(_provider: F, _options: WebSocketOptions) -> CResult<WebSocketHandle>
where
  F: Fn() -> Fut + 'static,
  Fut: std::future::Future<Output = CResult<String>> + 'static,
{
  let (state, _set_state) = signal(WebSocketReadyState::Open);
  let (message, set_message) = signal(None::<WebSocketMessage>);

  // Push every `ik_ws_message` payload (the frame text) into the message signal.
  let handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
    if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload"))
      && let Some(text) = payload.as_string()
    {
      set_message.set(Some(WebSocketMessage::Text(text)));
    }
  });
  // `listen` returns a Promise<unlisten>; registration completes on the microtask
  // queue. We keep the closure alive for the page's lifetime.
  let _ = listen("ik_ws_message", &handler);
  handler.forget();

  Ok(WebSocketHandle { state, message })
}
