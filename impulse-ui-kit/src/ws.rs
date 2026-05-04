//! WebSocket bindings for Leptos applications.
//!
//! Thin reactive wrapper around the browser [`WebSocket`] API. Connection state
//! and the most recent inbound frame are exposed as Leptos [`ReadSignal`]s so
//! they can be observed in components and effects.
//!
//! ```rust,ignore
//! use impulse_ui_kit::ws::{use_websocket, WebSocketMessage, WebSocketReadyState};
//!
//! let ws = use_websocket("wss://example.com/socket")?;
//!
//! Effect::new(move |_| {
//!   if ws.state.get() == WebSocketReadyState::Open {
//!     let _ = ws.send_text("hello");
//!   }
//! });
//!
//! Effect::new(move |_| {
//!   if let Some(WebSocketMessage::Text(text)) = ws.message.get() {
//!     log::info!("got: {text}");
//!   }
//! });
//! ```
//!
//! The socket is automatically closed and event listeners detached when the
//! last [`WebSocketHandle`] clone is dropped.

use std::rc::Rc;

use impulse_utils::prelude::*;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{BinaryType, CloseEvent, Event, MessageEvent, WebSocket};

/// Connection lifecycle state mirroring `WebSocket.readyState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketReadyState {
  /// Handshake in progress.
  Connecting,
  /// Open and ready for I/O.
  Open,
  /// `close()` was called and the closing handshake is in flight.
  Closing,
  /// Connection is fully closed.
  Closed,
}

/// Inbound payload kind delivered through [`WebSocketHandle::message`].
#[derive(Clone, Debug)]
pub enum WebSocketMessage {
  /// UTF-8 text frame.
  Text(String),
  /// Binary frame as raw bytes.
  Binary(Vec<u8>),
}

struct WebSocketInner {
  socket: WebSocket,
  _on_open: Closure<dyn FnMut(Event)>,
  _on_message: Closure<dyn FnMut(MessageEvent)>,
  _on_error: Closure<dyn FnMut(Event)>,
  _on_close: Closure<dyn FnMut(CloseEvent)>,
}

impl Drop for WebSocketInner {
  fn drop(&mut self) {
    self.socket.set_onopen(None);
    self.socket.set_onmessage(None);
    self.socket.set_onerror(None);
    self.socket.set_onclose(None);
    let _ = self.socket.close();
  }
}

/// Cheap-to-clone handle around a single browser [`WebSocket`].
///
/// Cloning shares the underlying socket via [`Rc`]; the connection is closed
/// when the last clone is dropped.
#[derive(Clone)]
pub struct WebSocketHandle {
  /// Reactive connection state.
  pub state: ReadSignal<WebSocketReadyState>,
  /// Latest inbound frame, if any.
  pub message: ReadSignal<Option<WebSocketMessage>>,
  inner: Rc<WebSocketInner>,
}

impl WebSocketHandle {
  /// Read the live `readyState` directly off the JS object.
  pub fn ready_state(&self) -> WebSocketReadyState {
    match self.inner.socket.ready_state() {
      WebSocket::CONNECTING => WebSocketReadyState::Connecting,
      WebSocket::OPEN => WebSocketReadyState::Open,
      WebSocket::CLOSING => WebSocketReadyState::Closing,
      _ => WebSocketReadyState::Closed,
    }
  }

  /// Send a UTF-8 text frame.
  pub fn send_text(&self, text: &str) -> CResult<()> {
    self
      .inner
      .socket
      .send_with_str(text)
      .map_err(|e| ClientError::from_str(format!("WebSocket text send failed: {e:?}")))
  }

  /// Send a binary frame.
  pub fn send_binary(&self, data: &[u8]) -> CResult<()> {
    self
      .inner
      .socket
      .send_with_u8_array(data)
      .map_err(|e| ClientError::from_str(format!("WebSocket binary send failed: {e:?}")))
  }

  /// Initiate a graceful close handshake with default code 1000.
  pub fn close(&self) -> CResult<()> {
    self
      .inner
      .socket
      .close()
      .map_err(|e| ClientError::from_str(format!("WebSocket close failed: {e:?}")))
  }

  /// Close with an explicit status code and reason.
  pub fn close_with_reason(&self, code: u16, reason: &str) -> CResult<()> {
    self
      .inner
      .socket
      .close_with_code_and_reason(code, reason)
      .map_err(|e| ClientError::from_str(format!("WebSocket close failed: {e:?}")))
  }

  /// Borrow the underlying [`WebSocket`] for advanced use cases.
  pub fn raw(&self) -> &WebSocket {
    &self.inner.socket
  }
}

/// Open a WebSocket to `url` (`ws://` or `wss://`).
pub fn use_websocket(url: impl AsRef<str>) -> CResult<WebSocketHandle> {
  use_websocket_inner(url.as_ref(), None)
}

/// Open a WebSocket negotiating one of the given subprotocols.
pub fn use_websocket_with_protocols(url: impl AsRef<str>, protocols: &[&str]) -> CResult<WebSocketHandle> {
  let arr = js_sys::Array::new();
  for p in protocols {
    arr.push(&JsValue::from_str(p));
  }
  use_websocket_inner(url.as_ref(), Some(arr.into()))
}

fn use_websocket_inner(url: &str, protocols: Option<JsValue>) -> CResult<WebSocketHandle> {
  let socket = match protocols {
    Some(p) => WebSocket::new_with_str_sequence(url, &p),
    None => WebSocket::new(url),
  }
  .map_err(|e| ClientError::from_str(format!("Failed to open WebSocket {url}: {e:?}")))?;

  socket.set_binary_type(BinaryType::Arraybuffer);

  let (state, set_state) = signal(WebSocketReadyState::Connecting);
  let (message, set_message) = signal::<Option<WebSocketMessage>>(None);

  let on_open = Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
    set_state.set(WebSocketReadyState::Open);
  });
  socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));

  let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
    let data = e.data();
    if let Some(text) = data.as_string() {
      set_message.set(Some(WebSocketMessage::Text(text)));
    } else if data.is_instance_of::<js_sys::ArrayBuffer>() {
      let arr = js_sys::Uint8Array::new(&data);
      let mut buf = vec![0u8; arr.length() as usize];
      arr.copy_to(&mut buf);
      set_message.set(Some(WebSocketMessage::Binary(buf)));
    } else {
      log::warn!("WebSocket received unsupported message data type");
    }
  });
  socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

  let on_error = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
    log::error!("WebSocket error: {e:?}");
  });
  socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));

  let on_close = Closure::<dyn FnMut(CloseEvent)>::new(move |_e: CloseEvent| {
    set_state.set(WebSocketReadyState::Closed);
  });
  socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));

  Ok(WebSocketHandle {
    state,
    message,
    inner: Rc::new(WebSocketInner {
      socket,
      _on_open: on_open,
      _on_message: on_message,
      _on_error: on_error,
      _on_close: on_close,
    }),
  })
}
