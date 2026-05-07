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

use std::rc::Rc;

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

/// Lifecycle state of a [`WebTransportHandle`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebTransportState {
  /// `ready` promise has not resolved yet.
  Connecting,
  /// `ready` resolved successfully — datagrams and streams may be used.
  Open,
  /// Session closed gracefully.
  Closed,
  /// `ready` rejected, or `closed` resolved with an error.
  Failed,
}

struct WebTransportInner {
  transport: WebTransport,
}

impl Drop for WebTransportInner {
  fn drop(&mut self) {
    self.transport.close();
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
  let url = url.as_ref();
  let transport = WebTransport::new(url)
    .map_err(|e| ClientError::from_str(format!("Failed to construct WebTransport for {url}: {e:?}")))?;
  Ok(setup_handle(transport))
}

/// Open a WebTransport session with custom [`WebTransportOptions`].
pub fn use_webtransport_with_options(
  url: impl AsRef<str>,
  options: &WebTransportOptions,
) -> CResult<WebTransportHandle> {
  let url = url.as_ref();
  let transport = WebTransport::new_with_options(url, options)
    .map_err(|e| ClientError::from_str(format!("Failed to construct WebTransport for {url}: {e:?}")))?;
  Ok(setup_handle(transport))
}

fn setup_handle(transport: WebTransport) -> WebTransportHandle {
  let (state, set_state) = signal(WebTransportState::Connecting);

  let ready: js_sys::Promise = transport.ready().unchecked_into();
  let closed: js_sys::Promise = transport.closed().unchecked_into();

  spawn_local(async move {
    if let Err(e) = JsFuture::from(ready).await {
      log::error!("WebTransport ready failed: {e:?}");
      set_state.set(WebTransportState::Failed);
      return;
    }
    set_state.set(WebTransportState::Open);
    match JsFuture::from(closed).await {
      Ok(_) => set_state.set(WebTransportState::Closed),
      Err(e) => {
        log::warn!("WebTransport closed with error: {e:?}");
        set_state.set(WebTransportState::Failed);
      }
    }
  });

  WebTransportHandle {
    state,
    inner: Rc::new(WebTransportInner { transport }),
  }
}

impl WebTransportHandle {
  /// Borrow the underlying [`WebTransport`] for advanced use cases.
  pub fn raw(&self) -> &WebTransport {
    &self.inner.transport
  }

  /// Close the session immediately.
  pub fn close(&self) {
    self.inner.transport.close();
  }

  /// Close the session with an explicit close code and reason.
  pub fn close_with_info(&self, info: &WebTransportCloseInfo) {
    self.inner.transport.close_with_close_info(info);
  }

  /// Send a single datagram. Acquires the duplex writer for the call and
  /// releases it before returning.
  pub async fn send_datagram(&self, data: &[u8]) -> CResult<()> {
    let writable: WritableStream = self.inner.transport.datagrams().writable();
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
    let promise: js_sys::Promise = self.inner.transport.create_bidirectional_stream().unchecked_into();
    let val = JsFuture::from(promise)
      .await
      .map_err(|e| ClientError::from_str(format!("Failed to open bidirectional stream: {e:?}")))?;
    val
      .dyn_into::<WebTransportBidirectionalStream>()
      .map_err(|_| ClientError::from_str("Unexpected bidirectional stream type"))
  }

  /// Open a new outbound unidirectional stream.
  pub async fn open_unidirectional_stream(&self) -> CResult<WebTransportSendStream> {
    let promise: js_sys::Promise = self.inner.transport.create_unidirectional_stream().unchecked_into();
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
  /// The signal updates with `Some(bytes)` for each received datagram.
  /// Locking the readable side of the duplex stream is exclusive — call this
  /// at most once per handle.
  pub fn datagram_signal(&self) -> CResult<ReadSignal<Option<Vec<u8>>>> {
    let (sig, set_sig) = signal::<Option<Vec<u8>>>(None);
    let readable: ReadableStream = self.inner.transport.datagrams().readable();
    let reader = ReadableStreamDefaultReader::new(&readable)
      .map_err(|e| ClientError::from_str(format!("Failed to acquire datagram reader: {e:?}")))?;
    spawn_local(async move {
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
              set_sig.set(Some(buf));
            }
          }
          Err(e) => {
            log::warn!("Datagram read error: {e:?}");
            break;
          }
        }
      }
    });
    Ok(sig)
  }
}
