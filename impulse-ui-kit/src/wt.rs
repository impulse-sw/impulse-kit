//! WebTransport bindings for Leptos applications.
//!
//! Reactive wrapper around the browser [WebTransport] API. Provides
//! datagram send/receive helpers and direct access to the underlying
//! bidirectional/unidirectional stream constructors so applications can layer
//! their own framing on top.
//!
//! [WebTransport]: https://developer.mozilla.org/en-US/docs/Web/API/WebTransport
//!
//! ```rust,ignore
//! use impulse_ui_kit::wt::{use_webtransport, WebTransportState};
//!
//! let wt = use_webtransport("https://example.com/path")?;
//!
//! Effect::new(move |_| {
//!   if wt.state.get() == WebTransportState::Open {
//!     leptos::task::spawn_local({
//!       let wt = wt.clone();
//!       async move {
//!         let _ = wt.send_datagram(b"ping").await;
//!       }
//!     });
//!   }
//! });
//! ```
//!
//! # Automatic reconnection
//!
//! Pass a [`ReconnectOptions`] (via [`use_webtransport_with_reconnect`] or
//! [`use_webtransport_with_options_and_reconnect`]) to have the handle rebuild
//! the session after an unexpected failure, with configurable backoff and an
//! optional attempt cap. Sends, stream constructors, and a registered
//! [`datagram_signal`](WebTransportHandle::datagram_signal) all transparently
//! follow the current session, so application code keeps working across
//! reconnects. A close requested through [`WebTransportHandle::close`], and a
//! graceful close by either peer, are treated as final and never reconnect.
//!
//! ```rust,ignore
//! use impulse_ui_kit::wt::use_webtransport_with_reconnect;
//! use impulse_ui_kit::reconnect::ReconnectOptions;
//!
//! let wt = use_webtransport_with_reconnect(
//!   "https://example.com/path",
//!   ReconnectOptions::enabled(),
//! )?;
//! ```
//!
//! # Build configuration
//!
//! The browser `WebTransport` API is gated by `web-sys` behind
//! `--cfg=web_sys_unstable_apis`. Downstream consumers must add this flag
//! when the `webtransport` feature is enabled, e.g. in `.cargo/config.toml`:
//!
//! ```toml,ignore
//! [target.wasm32-unknown-unknown]
//! rustflags = ["--cfg=web_sys_unstable_apis"]
//! ```

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use impulse_utils::prelude::*;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
  ReadableStream, ReadableStreamDefaultReader, WebTransport, WebTransportBidirectionalStream, WebTransportCloseInfo,
  WebTransportOptions, WebTransportSendStream, WritableStream, WritableStreamDefaultWriter,
};

use crate::reconnect::ReconnectOptions;

/// Lifecycle state of a [`WebTransportHandle`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebTransportState {
  /// `ready` promise has not resolved yet — also reported while waiting to
  /// reconnect.
  Connecting,
  /// `ready` resolved successfully — datagrams and streams may be used.
  Open,
  /// Session closed gracefully.
  Closed,
  /// `ready` rejected, or `closed` resolved with an error.
  Failed,
}

/// How to (re)build a [`WebTransport`] session.
struct WebTransportConfig {
  url: String,
  options: Option<WebTransportOptions>,
  reconnect: ReconnectOptions,
}

struct WebTransportInner {
  config: WebTransportConfig,
  set_state: WriteSignal<WebTransportState>,
  /// Current session. Swapped in place on each reconnect.
  transport: RefCell<WebTransport>,
  /// Set when the application requested a close, suppressing reconnection.
  manual_close: Cell<bool>,
  /// Sink installed by [`WebTransportHandle::datagram_signal`], re-attached to
  /// each new session.
  datagram_sink: RefCell<Option<WriteSignal<Option<Vec<u8>>>>>,
  /// Guards against running more than one datagram reader at a time.
  reader_running: Cell<bool>,
}

impl Drop for WebTransportInner {
  fn drop(&mut self) {
    self.transport.borrow().close();
  }
}

/// Cheap-to-clone handle around a single [`WebTransport`] session.
///
/// Cloning shares the underlying session via [`Rc`]; the session is closed
/// when the last clone is dropped.
#[derive(Clone)]
pub struct WebTransportHandle {
  /// Reactive lifecycle state.
  pub state: ReadSignal<WebTransportState>,
  inner: Rc<WebTransportInner>,
}

/// Open a WebTransport session to `url` (must be `https://`).
pub fn use_webtransport(url: impl AsRef<str>) -> CResult<WebTransportHandle> {
  build_handle(WebTransportConfig {
    url: url.as_ref().to_string(),
    options: None,
    reconnect: ReconnectOptions::default(),
  })
}

/// Open a WebTransport session with custom [`WebTransportOptions`].
pub fn use_webtransport_with_options(
  url: impl AsRef<str>,
  options: &WebTransportOptions,
) -> CResult<WebTransportHandle> {
  build_handle(WebTransportConfig {
    url: url.as_ref().to_string(),
    options: Some(options.clone()),
    reconnect: ReconnectOptions::default(),
  })
}

/// Open a WebTransport session that reconnects according to `reconnect`.
pub fn use_webtransport_with_reconnect(
  url: impl AsRef<str>,
  reconnect: ReconnectOptions,
) -> CResult<WebTransportHandle> {
  build_handle(WebTransportConfig {
    url: url.as_ref().to_string(),
    options: None,
    reconnect,
  })
}

/// Open a WebTransport session with custom [`WebTransportOptions`] and a
/// reconnection policy.
pub fn use_webtransport_with_options_and_reconnect(
  url: impl AsRef<str>,
  options: &WebTransportOptions,
  reconnect: ReconnectOptions,
) -> CResult<WebTransportHandle> {
  build_handle(WebTransportConfig {
    url: url.as_ref().to_string(),
    options: Some(options.clone()),
    reconnect,
  })
}

fn build_handle(config: WebTransportConfig) -> CResult<WebTransportHandle> {
  let transport = build_transport(&config)?;
  let (state, set_state) = signal(WebTransportState::Connecting);

  let inner = Rc::new(WebTransportInner {
    config,
    set_state,
    transport: RefCell::new(transport),
    manual_close: Cell::new(false),
    datagram_sink: RefCell::new(None),
    reader_running: Cell::new(false),
  });

  spawn_local(supervise(Rc::downgrade(&inner)));

  Ok(WebTransportHandle { state, inner })
}

fn build_transport(config: &WebTransportConfig) -> CResult<WebTransport> {
  match &config.options {
    Some(options) => WebTransport::new_with_options(&config.url, options),
    None => WebTransport::new(&config.url),
  }
  .map_err(|e| ClientError::from_str(format!("Failed to construct WebTransport for {}: {e:?}", config.url)))
}

/// Drive a single session through its `ready`/`closed` lifecycle, reconnecting
/// as the policy allows. Holds only a [`Weak`] reference so the handle's
/// [`Drop`] still tears the session down.
async fn supervise(weak: Weak<WebTransportInner>) {
  let mut failures: u32 = 0;

  loop {
    // Snapshot the current session and reset the visible state.
    let (ready, closed, transport) = match weak.upgrade() {
      Some(inner) => {
        if inner.manual_close.get() {
          return;
        }
        let transport = inner.transport.borrow().clone();
        inner.set_state.set(WebTransportState::Connecting);
        let ready: js_sys::Promise = transport.ready().unchecked_into();
        let closed: js_sys::Promise = transport.closed().unchecked_into();
        (ready, closed, transport)
      }
      None => return,
    };

    // Await the handshake.
    if let Err(e) = JsFuture::from(ready).await {
      match weak.upgrade() {
        Some(inner) if !inner.manual_close.get() => {
          log::error!("WebTransport ready failed: {e:?}");
          inner.set_state.set(WebTransportState::Failed);
        }
        _ => return,
      }
      if reconnect(&weak, &mut failures).await {
        continue;
      }
      return;
    }

    // Session is live.
    match weak.upgrade() {
      Some(inner) if !inner.manual_close.get() => {
        failures = 0;
        inner.set_state.set(WebTransportState::Open);
        ensure_reader(&inner, &transport);
      }
      _ => return,
    }

    // Await teardown.
    let closed_res = JsFuture::from(closed).await;
    match weak.upgrade() {
      Some(inner) => {
        if inner.manual_close.get() {
          inner.set_state.set(WebTransportState::Closed);
          return;
        }
        match closed_res {
          Ok(_) => {
            // A graceful close from either peer is final.
            inner.set_state.set(WebTransportState::Closed);
            return;
          }
          Err(e) => {
            log::warn!("WebTransport closed with error: {e:?}");
            inner.set_state.set(WebTransportState::Failed);
          }
        }
      }
      None => return,
    }

    if reconnect(&weak, &mut failures).await {
      continue;
    }
    return;
  }
}

/// If the policy permits another attempt, wait the backoff delay, build a fresh
/// session, and install it. Returns whether the supervisor should loop again.
async fn reconnect(weak: &Weak<WebTransportInner>, failures: &mut u32) -> bool {
  let delay = match weak.upgrade() {
    Some(inner) if !inner.manual_close.get() && inner.config.reconnect.should_retry(*failures) => {
      inner.config.reconnect.delay_for_attempt(*failures)
    }
    _ => return false,
  };
  *failures += 1;

  sleep(delay).await;

  match weak.upgrade() {
    Some(inner) if !inner.manual_close.get() => match build_transport(&inner.config) {
      Ok(transport) => {
        *inner.transport.borrow_mut() = transport;
        true
      }
      Err(e) => {
        log::error!("WebTransport reconnect construction failed: {e}");
        inner.set_state.set(WebTransportState::Failed);
        false
      }
    },
    _ => false,
  }
}

/// Start a datagram reader for `transport` if a sink is registered and one is
/// not already running. The reader stops when its session ends; the supervisor
/// starts a fresh one on the next session.
fn ensure_reader(inner: &Rc<WebTransportInner>, transport: &WebTransport) {
  if inner.reader_running.get() {
    return;
  }
  let sink = match *inner.datagram_sink.borrow() {
    Some(sink) => sink,
    None => return,
  };
  inner.reader_running.set(true);

  let transport = transport.clone();
  let weak = Rc::downgrade(inner);
  spawn_local(async move {
    read_datagrams(transport, sink).await;
    if let Some(inner) = weak.upgrade() {
      inner.reader_running.set(false);
    }
  });
}

/// Pump inbound datagrams from `transport` into `sink` until the session ends.
async fn read_datagrams(transport: WebTransport, sink: WriteSignal<Option<Vec<u8>>>) {
  let readable: ReadableStream = transport.datagrams().readable();
  let reader = match ReadableStreamDefaultReader::new(&readable) {
    Ok(reader) => reader,
    Err(e) => {
      log::warn!("Failed to acquire datagram reader: {e:?}");
      return;
    }
  };

  loop {
    match JsFuture::from(reader.read()).await {
      Ok(result) => {
        let done = js_sys::Reflect::get(&result, &JsValue::from_str("done"))
          .ok()
          .and_then(|v| v.as_bool())
          .unwrap_or(true);
        if done {
          break;
        }
        if let Ok(value) = js_sys::Reflect::get(&result, &JsValue::from_str("value")) {
          let arr = js_sys::Uint8Array::new(&value);
          let mut buf = vec![0u8; arr.length() as usize];
          arr.copy_to(&mut buf);
          sink.set(Some(buf));
        }
      }
      Err(e) => {
        log::warn!("Datagram read error: {e:?}");
        break;
      }
    }
  }
}

/// Resolve after `duration` using `setTimeout`.
async fn sleep(duration: Duration) {
  let millis = duration.as_millis().min(i32::MAX as u128) as i32;
  let promise = js_sys::Promise::new(&mut |resolve, _reject| {
    let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis);
  });
  let _ = JsFuture::from(promise).await;
}

impl WebTransportHandle {
  /// Clone the current underlying [`WebTransport`] for advanced use cases.
  ///
  /// The returned object refers to the session in use at call time; after a
  /// reconnect the handle points at a different session.
  pub fn raw(&self) -> WebTransport {
    self.inner.transport.borrow().clone()
  }

  /// Close the session immediately. Treated as intentional: reconnection is
  /// suppressed.
  pub fn close(&self) {
    self.inner.manual_close.set(true);
    self.inner.transport.borrow().close();
  }

  /// Close the session with an explicit close code and reason. Like
  /// [`Self::close`], this suppresses reconnection.
  pub fn close_with_info(&self, info: &WebTransportCloseInfo) {
    self.inner.manual_close.set(true);
    self.inner.transport.borrow().close_with_close_info(info);
  }

  /// Send a single datagram. Acquires the duplex writer for the call and
  /// releases it before returning.
  pub async fn send_datagram(&self, data: &[u8]) -> CResult<()> {
    let writable: WritableStream = self.inner.transport.borrow().datagrams().writable();
    let writer: WritableStreamDefaultWriter = writable
      .get_writer()
      .map_err(|e| ClientError::from_str(format!("Failed to acquire datagram writer: {e:?}")))?;
    let chunk: JsValue = js_sys::Uint8Array::from(data).into();
    let promise: js_sys::Promise = writer.write_with_chunk(&chunk).unchecked_into();
    let result = JsFuture::from(promise).await;
    writer.release_lock();
    result
      .map(|_| ())
      .map_err(|e| ClientError::from_str(format!("Failed to send datagram: {e:?}")))
  }

  /// Open a new bidirectional stream for application framing.
  pub async fn open_bidirectional_stream(&self) -> CResult<WebTransportBidirectionalStream> {
    let promise: js_sys::Promise = self.inner.transport.borrow().create_bidirectional_stream().unchecked_into();
    let val = JsFuture::from(promise)
      .await
      .map_err(|e| ClientError::from_str(format!("Failed to open bidirectional stream: {e:?}")))?;
    val
      .dyn_into::<WebTransportBidirectionalStream>()
      .map_err(|_| ClientError::from_str("Unexpected bidirectional stream type"))
  }

  /// Open a new outbound unidirectional stream.
  pub async fn open_unidirectional_stream(&self) -> CResult<WebTransportSendStream> {
    let promise: js_sys::Promise = self.inner.transport.borrow().create_unidirectional_stream().unchecked_into();
    let val = JsFuture::from(promise)
      .await
      .map_err(|e| ClientError::from_str(format!("Failed to open unidirectional stream: {e:?}")))?;
    val
      .dyn_into::<WebTransportSendStream>()
      .map_err(|_| ClientError::from_str("Unexpected unidirectional stream type"))
  }

  /// Spawn a background task that pumps inbound datagrams into a reactive
  /// signal, returning that signal.
  ///
  /// The signal updates with `Some(bytes)` for each received datagram. When
  /// reconnection is enabled the reader is re-attached to each new session
  /// automatically. Locking the readable side of the duplex stream is
  /// exclusive — call this at most once per handle.
  pub fn datagram_signal(&self) -> CResult<ReadSignal<Option<Vec<u8>>>> {
    if self.inner.datagram_sink.borrow().is_some() {
      return Err(ClientError::from_str("datagram_signal already registered for this handle"));
    }
    let (sig, set_sig) = signal::<Option<Vec<u8>>>(None);
    *self.inner.datagram_sink.borrow_mut() = Some(set_sig);

    // If the session is already open the supervisor has passed the point where
    // it would have started a reader, so start one now. Otherwise the
    // supervisor will start it when the session opens.
    if self.state.get_untracked() == WebTransportState::Open {
      let transport = self.inner.transport.borrow().clone();
      ensure_reader(&self.inner, &transport);
    }
    Ok(sig)
  }
}
