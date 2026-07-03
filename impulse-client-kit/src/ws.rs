//! WebSocket bindings for Leptos applications.
//!
//! Thin reactive wrapper around the browser [`WebSocket`] API. Connection state
//! and the most recent inbound frame are exposed as Leptos [`ReadSignal`]s so
//! they can be observed in components and effects.
//!
//! ```rust,ignore
//! use impulse_client_kit::ws::{use_websocket, WebSocketMessage, WebSocketReadyState};
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
//! # Automatic reconnection
//!
//! By default the socket is one-shot: once it closes it stays closed. Pass a
//! [`ReconnectOptions`] (via [`use_websocket_with_options`]) to have the handle
//! transparently re-open the connection after an unexpected drop, with
//! configurable backoff and an optional attempt cap:
//!
//! ```rust,ignore
//! use impulse_client_kit::ws::{use_websocket_with_options, WebSocketOptions};
//! use impulse_client_kit::reconnect::ReconnectOptions;
//!
//! let ws = use_websocket_with_options(
//!   "wss://example.com/socket",
//!   WebSocketOptions::default().with_reconnect(ReconnectOptions::enabled()),
//! )?;
//! ```
//!
//! While waiting between attempts the [`state`](WebSocketHandle::state) signal
//! reads [`WebSocketReadyState::Connecting`]. A close requested through
//! [`WebSocketHandle::close`] is treated as intentional and never reconnects.
//!
//! # Page lifecycle
//!
//! Reconnection can only react to a `close` event, and the browser does not
//! always deliver one. When a page is frozen into the back/forward cache
//! (bfcache) or a discarded tab, the socket's TCP connection is torn down but
//! the `close` event can be dropped while the document is frozen — on restore
//! the handle would otherwise sit on a dead socket forever, reporting
//! [`WebSocketReadyState::Open`] and never reconnecting.
//!
//! To recover, the handle listens for the page-lifecycle events that signal a
//! possibly-stale connection and revalidates the socket:
//!
//! * `pageshow` with `persisted == true` (a bfcache restore) forces a fresh
//!   reconnect unconditionally, since the restored socket is stale by
//!   definition even if it still reports `OPEN`.
//! * `online` and `visibilitychange` (becoming visible) reconnect only when the
//!   live socket is no longer `OPEN`/`CONNECTING`, recovering a missed `close`
//!   and collapsing any long pending backoff into an immediate attempt.
//!
//! These listeners are inert once [`close`](WebSocketHandle::close) has marked
//! the handle as intentionally closed.
//!
//! The socket is automatically closed and all event listeners detached when the
//! last [`WebSocketHandle`] clone is dropped.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use impulse_utils::page_lifecycle::{PageLifecycleListeners, on_page_restore};
use impulse_utils::prelude::*;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{BinaryType, CloseEvent, Event, MessageEvent, WebSocket};

use crate::reconnect::ReconnectOptions;

/// Future returned by a [`WsUrlProvider`], yielding the URL to connect to.
pub type WsUrlFuture = Pin<Box<dyn Future<Output = CResult<String>>>>;

/// Produces the connect URL for each (re)connect attempt.
///
/// Called once per attempt, so it can mint a fresh single-use token and embed
/// it in the URL — the handle never caches the URL across reconnects. A static
/// URL is just a provider that clones a constant (see [`use_websocket`]).
pub type WsUrlProvider = Rc<dyn Fn() -> WsUrlFuture>;

/// Connection lifecycle state mirroring `WebSocket.readyState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketReadyState {
  /// Handshake in progress — also reported while waiting to reconnect.
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

/// Construction-time configuration for [`use_websocket_with_options`].
#[derive(Clone, Debug, Default)]
pub struct WebSocketOptions {
  /// Subprotocols offered during the handshake. Empty means none.
  pub protocols: Vec<String>,
  /// Automatic reconnection policy. Disabled by default.
  pub reconnect: ReconnectOptions,
}

impl WebSocketOptions {
  /// Set the subprotocols offered during the handshake.
  pub fn with_protocols(mut self, protocols: impl IntoIterator<Item = impl Into<String>>) -> Self {
    self.protocols = protocols.into_iter().map(Into::into).collect();
    self
  }

  /// Set the automatic reconnection policy.
  pub fn with_reconnect(mut self, reconnect: ReconnectOptions) -> Self {
    self.reconnect = reconnect;
    self
  }
}

/// The live socket plus its event listeners. Replaced wholesale on each
/// (re)connect; dropping it detaches the listeners and closes the socket.
struct SocketSlot {
  socket: WebSocket,
  _on_open: Closure<dyn FnMut(Event)>,
  _on_message: Closure<dyn FnMut(MessageEvent)>,
  _on_error: Closure<dyn FnMut(Event)>,
  _on_close: Closure<dyn FnMut(CloseEvent)>,
}

impl Drop for SocketSlot {
  fn drop(&mut self) {
    self.socket.set_onopen(None);
    self.socket.set_onmessage(None);
    self.socket.set_onerror(None);
    self.socket.set_onclose(None);
    let _ = self.socket.close();
  }
}

struct WebSocketInner {
  url_provider: WsUrlProvider,
  protocols: Vec<String>,
  reconnect: ReconnectOptions,
  set_state: WriteSignal<WebSocketReadyState>,
  set_message: WriteSignal<Option<WebSocketMessage>>,
  /// Current socket and its listeners.
  slot: RefCell<Option<SocketSlot>>,
  /// Page-lifecycle listeners driving recovery from bfcache/frozen restores.
  lifecycle: RefCell<Option<PageLifecycleListeners>>,
  /// Consecutive failed/closed connections since the last successful open.
  failures: Cell<u32>,
  /// Set when the application requested a close, suppressing reconnection.
  manual_close: Cell<bool>,
  /// Set while a connect task is awaiting the URL provider, so overlapping
  /// attempts are never spawned.
  connecting: Cell<bool>,
  /// Bumped whenever an attempt is abandoned. The async connect task carries the
  /// generation it started with and bows out if it no longer matches, so a
  /// URL-provider future left wedged by a frozen tab can never install a stale
  /// socket or clobber the attempt that superseded it.
  generation: Cell<u32>,
  /// Handle for a scheduled reconnect, so it can be cancelled.
  pending: Cell<Option<TimeoutHandle>>,
  /// Handle for the per-attempt connect watchdog, so it can be cancelled once
  /// the socket opens (or the attempt is abandoned).
  watchdog: Cell<Option<TimeoutHandle>>,
}

impl Drop for WebSocketInner {
  fn drop(&mut self) {
    if let Some(handle) = self.pending.take() {
      handle.clear();
    }
    if let Some(handle) = self.watchdog.take() {
      handle.clear();
    }
    // The `SocketSlot` and the `PageLifecycleListeners` are dropped with `self`,
    // detaching their listeners and closing the socket.
  }
}

impl WebSocketInner {
  /// Kick off a (re)connect: await the URL provider, then open a socket.
  ///
  /// Asynchronous because the provider may need a network round-trip (e.g. to
  /// mint a fresh single-use token). A provider error or a failed open is
  /// funnelled through [`handle_close`](Self::handle_close) so backoff and the
  /// attempt cap keep applying.
  fn spawn_connect(self: &Rc<Self>) {
    if self.connecting.get() || self.manual_close.get() {
      return;
    }
    self.connecting.set(true);
    self.set_state.set(WebSocketReadyState::Connecting);

    let generation = self.generation.get();
    self.arm_watchdog(generation);

    let provider = self.url_provider.clone();
    let weak = Rc::downgrade(self);
    spawn_local(async move {
      let url_res = provider().await;
      let Some(inner) = weak.upgrade() else { return };
      // A newer attempt (or a close) superseded this one while we awaited the
      // provider — e.g. the watchdog fired on a wedged fetch, or a bfcache
      // restore forced a reconnect. Bow out without touching shared state.
      if inner.generation.get() != generation {
        return;
      }
      inner.connecting.set(false);
      if inner.manual_close.get() {
        return;
      }
      match url_res {
        Ok(url) => {
          if let Err(e) = inner.install_socket(&url) {
            log::error!("WebSocket failed to open: {e}");
            inner.handle_close();
          }
        }
        Err(e) => {
          log::error!("WebSocket URL provider failed: {e}");
          inner.handle_close();
        }
      }
    });
  }

  /// Abandon the in-flight connect attempt (if any): bump the generation so its
  /// async task bows out, clear the `connecting` guard so a fresh attempt may
  /// start, and cancel the watchdog covering it.
  fn abandon_connect(&self) {
    self.generation.set(self.generation.get().wrapping_add(1));
    self.connecting.set(false);
    if let Some(handle) = self.watchdog.take() {
      handle.clear();
    }
  }

  /// Whether the current socket reports `OPEN`.
  fn socket_is_open(&self) -> bool {
    self
      .slot
      .borrow()
      .as_ref()
      .is_some_and(|slot| slot.socket.ready_state() == WebSocket::OPEN)
  }

  /// Arm the per-attempt watchdog for the attempt tagged `generation`. If the
  /// socket has not opened by [`ReconnectOptions::connect_timeout`], the attempt
  /// is treated as failed and funnelled through [`handle_close`](Self::handle_close)
  /// for a backoff retry — this is what unsticks a URL-provider fetch or a
  /// socket handshake that stalls forever after a mobile tab is resumed.
  fn arm_watchdog(self: &Rc<Self>, generation: u32) {
    if let Some(handle) = self.watchdog.take() {
      handle.clear();
    }
    let Some(timeout) = self.reconnect.connect_timeout else {
      return;
    };
    let weak = Rc::downgrade(self);
    let handle = set_timeout_with_handle(
      move || {
        let Some(inner) = weak.upgrade() else { return };
        inner.watchdog.set(None);
        if inner.manual_close.get() || inner.generation.get() != generation || inner.socket_is_open() {
          return;
        }
        log::warn!("WebSocket connect timed out after {timeout:?}; retrying");
        // Supersede a possibly-wedged provider fetch and drop a stalled socket,
        // then fall into the normal close/backoff path.
        inner.abandon_connect();
        inner.slot.replace(None);
        inner.handle_close();
      },
      timeout,
    )
    .ok();
    self.watchdog.set(handle);
  }

  /// Open a fresh socket, wire up listeners, and install it as the current slot.
  fn install_socket(self: &Rc<Self>, url: &str) -> CResult<()> {
    let inner = self;
    let socket = build_socket(url, &inner.protocols)?;
    socket.set_binary_type(BinaryType::Arraybuffer);

    let set_state = inner.set_state;
    let set_message = inner.set_message;

    let weak = Rc::downgrade(inner);
    let on_open = Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
      if let Some(inner) = weak.upgrade() {
        inner.failures.set(0);
        // The attempt succeeded; stand the watchdog down.
        if let Some(handle) = inner.watchdog.take() {
          handle.clear();
        }
      }
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

    let weak = Rc::downgrade(inner);
    let on_close = Closure::<dyn FnMut(CloseEvent)>::new(move |_e: CloseEvent| {
      if let Some(inner) = weak.upgrade() {
        inner.handle_close();
      }
    });
    socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));

    inner.slot.replace(Some(SocketSlot {
      socket,
      _on_open: on_open,
      _on_message: on_message,
      _on_error: on_error,
      _on_close: on_close,
    }));
    set_state.set(WebSocketReadyState::Connecting);
    Ok(())
  }

  /// React to a closed socket: either settle as closed or schedule a reconnect.
  fn handle_close(self: &Rc<Self>) {
    // This attempt is over; a scheduled retry arms its own watchdog.
    if let Some(handle) = self.watchdog.take() {
      handle.clear();
    }
    if self.manual_close.get() {
      self.set_state.set(WebSocketReadyState::Closed);
      return;
    }

    let failures = self.failures.get();
    if !self.reconnect.should_retry(failures) {
      self.set_state.set(WebSocketReadyState::Closed);
      return;
    }

    let delay = self.reconnect.delay_for_attempt(failures);
    self.failures.set(failures + 1);
    self.set_state.set(WebSocketReadyState::Connecting);

    let weak = Rc::downgrade(self);
    let handle = set_timeout_with_handle(
      move || {
        if let Some(inner) = weak.upgrade() {
          inner.pending.set(None);
          if inner.manual_close.get() {
            return;
          }
          // `spawn_connect` awaits the URL provider; a provider error or failed
          // open re-enters `handle_close`, so backoff and the cap keep applying.
          inner.spawn_connect();
        }
      },
      delay,
    )
    .ok();
    self.pending.set(handle);
  }

  /// Whether the current socket is `OPEN` or `CONNECTING` — i.e. healthy or
  /// mid-handshake. A missing slot, `CLOSING`, or `CLOSED` counts as dead.
  fn socket_alive(&self) -> bool {
    self
      .slot
      .borrow()
      .as_ref()
      .is_some_and(|slot| matches!(slot.socket.ready_state(), WebSocket::OPEN | WebSocket::CONNECTING))
  }

  /// Recover a possibly-stale connection after a page-lifecycle wake-up.
  ///
  /// With `force` (a bfcache restore), the current socket is discarded and
  /// rebuilt regardless of its reported state — even a socket still reporting
  /// `OPEN` or stuck in `CONNECTING` is stale by definition — and any in-flight
  /// connect attempt is superseded, since its URL-provider fetch may have been
  /// wedged by the freeze.
  ///
  /// Without `force` (`online`/`visibilitychange`), a healthy or actively
  /// handshaking socket is left alone and an in-flight attempt is allowed to run
  /// (the watchdog bounds it); the reconnect only fires when the socket is dead,
  /// recovering a `close` event dropped while frozen and collapsing any pending
  /// backoff into an immediate attempt.
  fn revalidate(self: &Rc<Self>, force: bool) {
    if self.manual_close.get() {
      return;
    }
    if !force {
      // Leave a healthy or mid-handshake socket, and let an in-flight attempt
      // finish on its own — the connect watchdog is what unsticks a wedged one.
      if self.socket_alive() || self.connecting.get() {
        return;
      }
    }
    // Drop any scheduled retry, supersede any in-flight attempt, and discard the
    // stale socket (its `Drop` detaches listeners before closing, so there is no
    // spurious `handle_close`), then reconnect immediately with backoff reset.
    if let Some(handle) = self.pending.take() {
      handle.clear();
    }
    self.abandon_connect();
    self.slot.replace(None);
    self.failures.set(0);
    self.spawn_connect();
  }

  /// Attach the page-lifecycle listeners that recover the socket after the page
  /// is restored from bfcache, comes back online, or becomes visible again.
  ///
  /// The `force` flag maps straight onto [`revalidate`](Self::revalidate): a
  /// bfcache restore rebuilds unconditionally, while an `online`/visible wake
  /// only reconnects a socket that already looks dead.
  fn install_lifecycle_listeners(self: &Rc<Self>) {
    let weak = Rc::downgrade(self);
    let listeners = on_page_restore(move |force| {
      if let Some(inner) = weak.upgrade() {
        inner.revalidate(force);
      }
    });
    if listeners.is_none() {
      log::warn!("WebSocket: no window; skipping page-lifecycle recovery listeners");
    }
    self.lifecycle.replace(listeners);
  }
}

/// Cheap-to-clone handle around a single browser [`WebSocket`].
///
/// Cloning shares the underlying socket via [`Rc`]; the connection is closed
/// when the last clone is dropped. When reconnection is enabled the handle
/// transparently swaps in a new socket on each attempt — code observing
/// [`state`](Self::state) and [`message`](Self::message) keeps working across
/// reconnects.
#[derive(Clone)]
pub struct WebSocketHandle {
  /// Reactive connection state.
  pub state: ReadSignal<WebSocketReadyState>,
  /// Latest inbound frame, if any.
  pub message: ReadSignal<Option<WebSocketMessage>>,
  inner: Rc<WebSocketInner>,
}

impl WebSocketHandle {
  /// Read the live `readyState` directly off the current JS socket.
  pub fn ready_state(&self) -> WebSocketReadyState {
    match self.inner.slot.borrow().as_ref() {
      Some(slot) => match slot.socket.ready_state() {
        WebSocket::CONNECTING => WebSocketReadyState::Connecting,
        WebSocket::OPEN => WebSocketReadyState::Open,
        WebSocket::CLOSING => WebSocketReadyState::Closing,
        _ => WebSocketReadyState::Closed,
      },
      None => WebSocketReadyState::Closed,
    }
  }

  /// Send a UTF-8 text frame.
  pub fn send_text(&self, text: &str) -> CResult<()> {
    let slot = self.inner.slot.borrow();
    let slot = slot
      .as_ref()
      .ok_or_else(|| ClientError::from_str("WebSocket is not connected"))?;
    slot
      .socket
      .send_with_str(text)
      .map_err(|e| ClientError::from_str(format!("WebSocket text send failed: {e:?}")))
  }

  /// Send a binary frame.
  pub fn send_binary(&self, data: &[u8]) -> CResult<()> {
    let slot = self.inner.slot.borrow();
    let slot = slot
      .as_ref()
      .ok_or_else(|| ClientError::from_str("WebSocket is not connected"))?;
    slot
      .socket
      .send_with_u8_array(data)
      .map_err(|e| ClientError::from_str(format!("WebSocket binary send failed: {e:?}")))
  }

  /// Initiate a graceful close handshake with default code 1000.
  ///
  /// This is treated as an intentional close: any pending reconnect is
  /// cancelled and no further attempts are made.
  pub fn close(&self) -> CResult<()> {
    self.suppress_reconnect();
    let slot = self.inner.slot.borrow();
    let slot = slot
      .as_ref()
      .ok_or_else(|| ClientError::from_str("WebSocket is not connected"))?;
    slot
      .socket
      .close()
      .map_err(|e| ClientError::from_str(format!("WebSocket close failed: {e:?}")))
  }

  /// Close with an explicit status code and reason. Like [`Self::close`], this
  /// suppresses reconnection.
  pub fn close_with_reason(&self, code: u16, reason: &str) -> CResult<()> {
    self.suppress_reconnect();
    let slot = self.inner.slot.borrow();
    let slot = slot
      .as_ref()
      .ok_or_else(|| ClientError::from_str("WebSocket is not connected"))?;
    slot
      .socket
      .close_with_code_and_reason(code, reason)
      .map_err(|e| ClientError::from_str(format!("WebSocket close failed: {e:?}")))
  }

  /// Clone the current underlying [`WebSocket`] for advanced use cases.
  ///
  /// The returned object refers to the socket in use at call time; after a
  /// reconnect the handle points at a different socket.
  pub fn raw(&self) -> Option<WebSocket> {
    self.inner.slot.borrow().as_ref().map(|slot| slot.socket.clone())
  }

  fn suppress_reconnect(&self) {
    self.inner.manual_close.set(true);
    if let Some(handle) = self.inner.pending.take() {
      handle.clear();
    }
    if let Some(handle) = self.inner.watchdog.take() {
      handle.clear();
    }
  }
}

/// Build a [`WsUrlProvider`] that always yields the same constant URL.
fn constant_provider(url: impl AsRef<str>) -> WsUrlProvider {
  let url = url.as_ref().to_string();
  Rc::new(move || {
    let url = url.clone();
    Box::pin(async move { Ok(url) })
  })
}

/// Open a WebSocket to `url` (`ws://` or `wss://`).
pub fn use_websocket(url: impl AsRef<str>) -> CResult<WebSocketHandle> {
  use_websocket_with_options(url, WebSocketOptions::default())
}

/// Open a WebSocket negotiating one of the given subprotocols.
pub fn use_websocket_with_protocols(url: impl AsRef<str>, protocols: &[&str]) -> CResult<WebSocketHandle> {
  use_websocket_with_options(
    url,
    WebSocketOptions::default().with_protocols(protocols.iter().copied()),
  )
}

/// Open a WebSocket with full [`WebSocketOptions`], including an optional
/// automatic reconnection policy.
pub fn use_websocket_with_options(url: impl AsRef<str>, options: WebSocketOptions) -> CResult<WebSocketHandle> {
  use_websocket_with_provider(constant_provider(url), options)
}

/// Open a WebSocket whose URL is produced by `provider` on every (re)connect.
///
/// Unlike the static-URL constructors, `provider` is invoked once per attempt,
/// so each reconnect can mint a fresh single-use token and bake it into the
/// URL. For an ergonomic closure form see [`use_websocket_with_url_fn`].
pub fn use_websocket_with_provider(provider: WsUrlProvider, options: WebSocketOptions) -> CResult<WebSocketHandle> {
  let (state, set_state) = signal(WebSocketReadyState::Connecting);
  let (message, set_message) = signal::<Option<WebSocketMessage>>(None);

  let inner = Rc::new(WebSocketInner {
    url_provider: provider,
    protocols: options.protocols,
    reconnect: options.reconnect,
    set_state,
    set_message,
    slot: RefCell::new(None),
    lifecycle: RefCell::new(None),
    failures: Cell::new(0),
    manual_close: Cell::new(false),
    connecting: Cell::new(false),
    generation: Cell::new(0),
    pending: Cell::new(None),
    watchdog: Cell::new(None),
  });

  // Recover from bfcache/frozen restores where a `close` event never arrives.
  // Only worth wiring up when reconnection is enabled — a one-shot socket has
  // nothing to reconnect to.
  if options.reconnect.enabled {
    inner.install_lifecycle_listeners();
  }
  inner.spawn_connect();

  Ok(WebSocketHandle { state, message, inner })
}

/// Ergonomic wrapper over [`use_websocket_with_provider`] accepting an async
/// closure that yields the URL.
///
/// ```rust,ignore
/// let ws = use_websocket_with_url_fn(
///   || async move { Ok(format!("wss://host/ws?ticket={}", fetch_ticket().await?)) },
///   WebSocketOptions::default().with_reconnect(ReconnectOptions::enabled()),
/// )?;
/// ```
pub fn use_websocket_with_url_fn<F, Fut>(provider: F, options: WebSocketOptions) -> CResult<WebSocketHandle>
where
  F: Fn() -> Fut + 'static,
  Fut: Future<Output = CResult<String>> + 'static,
{
  let provider: WsUrlProvider = Rc::new(move || Box::pin(provider()));
  use_websocket_with_provider(provider, options)
}

fn build_socket(url: &str, protocols: &[String]) -> CResult<WebSocket> {
  let socket = if protocols.is_empty() {
    WebSocket::new(url)
  } else {
    let arr = js_sys::Array::new();
    for p in protocols {
      arr.push(&JsValue::from_str(p));
    }
    WebSocket::new_with_str_sequence(url, &arr)
  }
  .map_err(|e| ClientError::from_str(format!("Failed to open WebSocket {url}: {e:?}")))?;
  Ok(socket)
}
