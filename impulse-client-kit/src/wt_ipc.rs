//! Tauri-IPC backend for the WebTransport surface, selected under `cfg(tauri)`.
//!
//! The WebTransport analogue of [`crate::ws_ipc`]. In a Tauri webview the app
//! can't open a WebTransport session to an arbitrary host, so the session lives
//! in the native engine and the UI talks to it over IPC: outgoing frames are
//! forwarded with `invoke("ik_wt_send", { text })` and incoming frames arrive as
//! the Tauri event `ik_wt_message` (payload = the frame text). The handle mirrors
//! [`crate::wt::WebTransportHandle`]'s essentials — a reactive `state`/`message`
//! and `send_text` — so an app swaps transports with a one-line `cfg` on the
//! import.
//!
//! The native side is transport-agnostic: the same `impulse_tauri_engine::WsEngine`
//! that backs `ik_ws_send`/`ik_ws_message` also drives a WebTransport socket — an
//! app just registers a second command/emit pair (`ik_wt_send` / `ik_wt_message`)
//! against a second engine whose `WsRemote` is a WebTransport session. Requires
//! `withGlobalTauri` so `window.__TAURI__` is present.

use impulse_utils::prelude::{CResult, ClientError};
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

// The lifecycle enum is shared with the real transport so call sites are identical.
pub use crate::wt::WebTransportState;

#[wasm_bindgen]
extern "C" {
  // Tauri v2 global bindings (require `withGlobalTauri: true`).
  #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
  fn invoke(cmd: &str, args: JsValue) -> JsValue;

  #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
  fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;
}

/// IPC-backed handle mirroring [`crate::wt::WebTransportHandle`]'s essential
/// surface.
///
/// The channel to the engine is always considered `Open` — the engine itself
/// tracks server connectivity and serves offline, so from the UI's point of view
/// the transport never drops.
#[derive(Clone)]
pub struct WebTransportHandle {
  /// Ready state (always `Open` for the local IPC channel).
  pub state: ReadSignal<WebTransportState>,
  /// The most recent inbound frame text.
  pub message: ReadSignal<Option<String>>,
}

impl WebTransportHandle {
  /// Forwards a text frame to the engine via `invoke("ik_wt_send", { text })`.
  pub fn send_text(&self, text: &str) -> CResult<()> {
    #[derive(serde::Serialize)]
    struct Args<'a> {
      text: &'a str,
    }
    let args = serde_wasm_bindgen::to_value(&Args { text })
      .map_err(|e| ClientError::from_str(format!("IPC encode failed: {e:?}")))?;
    // Fire-and-forget: the command runs; we don't await its promise.
    let _ = invoke("ik_wt_send", args);
    Ok(())
  }

  /// No-op: the IPC channel isn't closed by the UI (the engine owns its lifecycle).
  pub fn close(&self) -> CResult<()> {
    Ok(())
  }
}

/// Opens the IPC channel to the engine. The URL `provider` is accepted for
/// signature parity with [`crate::wt::use_webtransport_with_url_fn`] but ignored
/// here — the engine is the peer, not a URL.
pub fn use_webtransport_with_url_fn<F, Fut>(_provider: F) -> CResult<WebTransportHandle>
where
  F: Fn() -> Fut + 'static,
  Fut: std::future::Future<Output = CResult<String>> + 'static,
{
  let (state, _set_state) = signal(WebTransportState::Open);
  let (message, set_message) = signal(None::<String>);

  // Push every `ik_wt_message` payload (the frame text) into the message signal.
  let handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
    if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload"))
      && let Some(text) = payload.as_string()
    {
      set_message.set(Some(text));
    }
  });
  // `listen` returns a Promise<unlisten>; registration completes on the microtask
  // queue. We keep the closure alive for the page's lifetime.
  let _ = listen("ik_wt_message", &handler);
  handler.forget();

  Ok(WebTransportHandle { state, message })
}
