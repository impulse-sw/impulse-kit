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
//! use impulse_client_kit::wt::{use_webtransport, WebTransportState};
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
//! use impulse_client_kit::wt::use_webtransport_with_reconnect;
//! use impulse_client_kit::reconnect::ReconnectOptions;
//!
//! let wt = use_webtransport_with_reconnect(
//!   "https://example.com/path",
//!   ReconnectOptions::enabled(),
//! )?;
//! ```
//!
//! # Page lifecycle
//!
//! The supervisor normally only rebuilds a session when its `closed` promise
//! settles, but the browser does not always deliver that: when a page is frozen
//! into the back/forward cache (bfcache) or a discarded tab, the transport is
//! torn down while the `closed` promise stays pending, leaving the supervisor
//! parked on a dead session forever after restore. To recover, the handle
//! listens for page-lifecycle events and, when appropriate, wakes the
//! supervisor to abandon the current session and reconnect immediately:
//!
//! * `pageshow` with `persisted == true` (a bfcache restore) always reconnects,
//!   since the restored session is stale even if it still reports `Open`.
//! * `online` and `visibilitychange` (becoming visible) reconnect only when the
//!   session is not already `Open`/`Connecting`, recovering a `closed` promise
//!   that never resolved and collapsing any pending backoff into an immediate
//!   attempt.
//!
//! The wake is distinct from a graceful close, so it reconnects where a peer's
//! `close()` would (correctly) be final. These listeners are inert once
//! [`close`](WebTransportHandle::close) has marked the handle as intentionally
//! closed, and are only wired up when reconnection is enabled.
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
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::time::Duration;

use impulse_utils::prelude::*;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
  Event, PageTransitionEvent, ReadableStream, ReadableStreamDefaultReader, WebTransport,
  WebTransportBidirectionalStream, WebTransportCloseInfo, WebTransportOptions, WebTransportSendStream, WritableStream,
  WritableStreamDefaultWriter,
};

use crate::reconnect::ReconnectOptions;

/// A one-shot, re-armable notification bridged to JS so it can be raced against
/// another promise with [`js_sys::Promise::race`]. [`WebTransportInner`] holds
/// the live [`Wake`]; a page-lifecycle handler resolves its promise (with the
/// inner's sentinel) to interrupt the supervisor's `closed`/backoff wait and
/// force an immediate reconnect. The supervisor re-arms a fresh one afterwards.
struct Wake {
  promise: js_sys::Promise,
  resolve: js_sys::Function,
}

impl Wake {
  fn new() -> Self {
    let mut resolve_slot = None;
    // `Promise::new` runs its executor synchronously, so `resolve` is set here.
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
      resolve_slot = Some(resolve);
    });
    Self {
      promise,
      resolve: resolve_slot.expect("Promise executor runs synchronously"),
    }
  }
}

/// Outcome of awaiting a promise that was raced against the lifecycle [`Wake`].
enum Raced {
  /// A lifecycle wake fired first: abandon the current session and reconnect.
  Woken,
  /// The awaited promise settled normally, carrying its result.
  Settled(Result<JsValue, JsValue>),
}

/// Window/document listeners for page-lifecycle events, kept alive for the
/// lifetime of the handle and detached on drop.
struct LifecycleSlot {
  _on_pageshow: Closure<dyn FnMut(PageTransitionEvent)>,
  _on_online: Closure<dyn FnMut(Event)>,
  _on_visibility: Closure<dyn FnMut(Event)>,
}

impl Drop for LifecycleSlot {
  fn drop(&mut self) {
    let Some(win) = web_sys::window() else { return };
    let _ = win.remove_event_listener_with_callback("pageshow", self._on_pageshow.as_ref().unchecked_ref());
    let _ = win.remove_event_listener_with_callback("online", self._on_online.as_ref().unchecked_ref());
    if let Some(doc) = win.document() {
      let _ = doc.remove_event_listener_with_callback("visibilitychange", self._on_visibility.as_ref().unchecked_ref());
    }
  }
}

/// Future returned by a [`WtUrlProvider`], yielding the URL to connect to.
pub type WtUrlFuture = Pin<Box<dyn Future<Output = CResult<String>>>>;

/// Produces the connect URL for each (re)connect of a WebTransport session.
///
/// Invoked once per attempt, so it can mint a fresh single-use token and embed
/// it in the URL. A static URL is just a provider that clones a constant.
pub type WtUrlProvider = Rc<dyn Fn() -> WtUrlFuture>;

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
  url_provider: WtUrlProvider,
  options: Option<WebTransportOptions>,
  reconnect: ReconnectOptions,
}

struct WebTransportInner {
  config: WebTransportConfig,
  set_state: WriteSignal<WebTransportState>,
  /// Synchronous mirror of the reactive state, readable from lifecycle handlers
  /// without touching the reactive system. Kept in step via [`Self::update_state`].
  current_state: Cell<WebTransportState>,
  /// Current session. `None` until the first session is built; swapped in place
  /// on each reconnect.
  transport: RefCell<Option<WebTransport>>,
  /// Set when the application requested a close, suppressing reconnection.
  manual_close: Cell<bool>,
  /// Sink installed by [`WebTransportHandle::datagram_signal`], re-attached to
  /// each new session.
  datagram_sink: RefCell<Option<WriteSignal<Option<Vec<u8>>>>>,
  /// Guards against running more than one datagram reader at a time.
  reader_running: Cell<bool>,
  /// Live wake used to interrupt the supervisor's `closed`/backoff wait on a
  /// page-lifecycle event. Re-armed by the supervisor after each firing.
  wake: RefCell<Wake>,
  /// Unique object resolved through the [`Wake`] promise, so the supervisor can
  /// tell a wake apart from the raced promise settling on its own.
  sentinel: JsValue,
  /// Page-lifecycle listeners driving recovery from bfcache/frozen restores.
  lifecycle: RefCell<Option<LifecycleSlot>>,
}

impl WebTransportInner {
  /// Update both the reactive state signal and its synchronous mirror.
  fn update_state(&self, state: WebTransportState) {
    self.current_state.set(state);
    self.set_state.set(state);
  }

  /// Resolve the live wake promise, interrupting whichever wait the supervisor
  /// is parked in and forcing an immediate reconnect.
  fn request_wake(&self) {
    let _ = self.wake.borrow().resolve.call1(&JsValue::UNDEFINED, &self.sentinel);
  }

  /// Install a fresh, unfired wake for the next wait.
  fn rearm_wake(&self) {
    *self.wake.borrow_mut() = Wake::new();
  }

  /// Wake the supervisor only if the session looks dead — used by the `online`
  /// and `visibilitychange` handlers, which must not disturb a healthy or
  /// in-progress session.
  fn wake_if_dead(&self) {
    if self.manual_close.get() {
      return;
    }
    if matches!(
      self.current_state.get(),
      WebTransportState::Open | WebTransportState::Connecting
    ) {
      return;
    }
    self.request_wake();
  }
}

impl Drop for WebTransportInner {
  fn drop(&mut self) {
    if let Some(transport) = self.transport.borrow().as_ref() {
      transport.close();
    }
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

/// Build a [`WtUrlProvider`] that always yields the same constant URL.
fn constant_provider(url: impl AsRef<str>) -> WtUrlProvider {
  let url = url.as_ref().to_string();
  Rc::new(move || {
    let url = url.clone();
    Box::pin(async move { Ok(url) })
  })
}

/// Open a WebTransport session to `url` (must be `https://`).
pub fn use_webtransport(url: impl AsRef<str>) -> CResult<WebTransportHandle> {
  build_handle(WebTransportConfig {
    url_provider: constant_provider(url),
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
    url_provider: constant_provider(url),
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
    url_provider: constant_provider(url),
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
    url_provider: constant_provider(url),
    options: Some(options.clone()),
    reconnect,
  })
}

/// Open a WebTransport session whose URL is produced by `provider` on every
/// (re)connect, so each attempt can mint a fresh single-use token.
pub fn use_webtransport_with_provider(
  provider: WtUrlProvider,
  options: Option<WebTransportOptions>,
  reconnect: ReconnectOptions,
) -> CResult<WebTransportHandle> {
  build_handle(WebTransportConfig {
    url_provider: provider,
    options,
    reconnect,
  })
}

/// Ergonomic wrapper over [`use_webtransport_with_provider`] accepting an async
/// closure that yields the URL.
pub fn use_webtransport_with_url_fn<F, Fut>(
  provider: F,
  options: Option<WebTransportOptions>,
  reconnect: ReconnectOptions,
) -> CResult<WebTransportHandle>
where
  F: Fn() -> Fut + 'static,
  Fut: Future<Output = CResult<String>> + 'static,
{
  let provider: WtUrlProvider = Rc::new(move || Box::pin(provider()));
  use_webtransport_with_provider(provider, options, reconnect)
}

fn build_handle(config: WebTransportConfig) -> CResult<WebTransportHandle> {
  let (state, set_state) = signal(WebTransportState::Connecting);

  let reconnect_enabled = config.reconnect.enabled;
  let inner = Rc::new(WebTransportInner {
    config,
    set_state,
    current_state: Cell::new(WebTransportState::Connecting),
    transport: RefCell::new(None),
    manual_close: Cell::new(false),
    datagram_sink: RefCell::new(None),
    reader_running: Cell::new(false),
    wake: RefCell::new(Wake::new()),
    sentinel: js_sys::Object::new().into(),
    lifecycle: RefCell::new(None),
  });

  // Recover from bfcache/frozen restores where `closed` never resolves. Only
  // worth wiring up when reconnection is enabled — a one-shot session has
  // nothing to reconnect to.
  if reconnect_enabled {
    install_lifecycle_listeners(&inner);
  }

  spawn_local(supervise(Rc::downgrade(&inner)));

  Ok(WebTransportHandle { state, inner })
}

/// Attach the window/document listeners that wake the supervisor after the page
/// is restored from bfcache, comes back online, or becomes visible again.
fn install_lifecycle_listeners(inner: &Rc<WebTransportInner>) {
  let Some(win) = web_sys::window() else {
    log::warn!("WebTransport: no window; skipping page-lifecycle recovery listeners");
    return;
  };

  // A bfcache restore (`persisted`) always reconnects; a normal initial
  // `pageshow` (`persisted == false`) is a no-op.
  let weak = Rc::downgrade(inner);
  let on_pageshow = Closure::<dyn FnMut(PageTransitionEvent)>::new(move |e: PageTransitionEvent| {
    if e.persisted()
      && let Some(inner) = weak.upgrade()
      && !inner.manual_close.get()
    {
      inner.request_wake();
    }
  });
  let _ = win.add_event_listener_with_callback("pageshow", on_pageshow.as_ref().unchecked_ref());

  let weak = Rc::downgrade(inner);
  let on_online = Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
    if let Some(inner) = weak.upgrade() {
      inner.wake_if_dead();
    }
  });
  let _ = win.add_event_listener_with_callback("online", on_online.as_ref().unchecked_ref());

  let weak = Rc::downgrade(inner);
  let on_visibility = Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
    let hidden = web_sys::window().and_then(|w| w.document()).is_some_and(|d| d.hidden());
    if !hidden && let Some(inner) = weak.upgrade() {
      inner.wake_if_dead();
    }
  });
  if let Some(doc) = win.document() {
    let _ = doc.add_event_listener_with_callback("visibilitychange", on_visibility.as_ref().unchecked_ref());
  }

  inner.lifecycle.replace(Some(LifecycleSlot {
    _on_pageshow: on_pageshow,
    _on_online: on_online,
    _on_visibility: on_visibility,
  }));
}

/// Await `promise`, racing it against the live lifecycle [`Wake`]. Returns
/// [`Raced::Woken`] (re-arming the wake) if a lifecycle event fired first,
/// otherwise [`Raced::Settled`] with the promise's own result.
async fn await_or_wake(weak: &Weak<WebTransportInner>, promise: js_sys::Promise) -> Raced {
  let (wake_promise, sentinel) = match weak.upgrade() {
    Some(inner) => (inner.wake.borrow().promise.clone(), inner.sentinel.clone()),
    None => return Raced::Woken,
  };
  let raced: js_sys::Promise = js_sys::Promise::race(&js_sys::Array::of2(&promise, &wake_promise));
  match JsFuture::from(raced).await {
    Ok(val) if js_sys::Object::is(&val, &sentinel) => {
      if let Some(inner) = weak.upgrade() {
        inner.rearm_wake();
      }
      Raced::Woken
    }
    other => Raced::Settled(other),
  }
}

fn build_transport(url: &str, options: &Option<WebTransportOptions>) -> CResult<WebTransport> {
  match options {
    Some(options) => WebTransport::new_with_options(url, options),
    None => WebTransport::new(url),
  }
  .map_err(|e| ClientError::from_str(format!("Failed to construct WebTransport for {url}: {e:?}")))
}

/// Drive a single session through its `ready`/`closed` lifecycle, reconnecting
/// as the policy allows. Holds only a [`Weak`] reference so the handle's
/// [`Drop`] still tears the session down.
async fn supervise(weak: Weak<WebTransportInner>) {
  let mut failures: u32 = 0;

  loop {
    // Await the URL provider for this attempt (may mint a fresh token).
    let url = match weak.upgrade() {
      Some(inner) => {
        if inner.manual_close.get() {
          return;
        }
        inner.update_state(WebTransportState::Connecting);
        let provider = inner.config.url_provider.clone();
        drop(inner);
        match provider().await {
          Ok(url) => url,
          Err(e) => {
            log::error!("WebTransport URL provider failed: {e}");
            if backoff(&weak, &mut failures).await {
              continue;
            }
            return;
          }
        }
      }
      None => return,
    };

    // Build a fresh session and install it.
    let (ready, closed, transport) = match weak.upgrade() {
      Some(inner) => {
        if inner.manual_close.get() {
          return;
        }
        match build_transport(&url, &inner.config.options) {
          Ok(transport) => {
            *inner.transport.borrow_mut() = Some(transport.clone());
            let ready: js_sys::Promise = transport.ready().unchecked_into();
            let closed: js_sys::Promise = transport.closed().unchecked_into();
            (ready, closed, transport)
          }
          Err(e) => {
            log::error!("WebTransport construction failed: {e}");
            inner.update_state(WebTransportState::Failed);
            drop(inner);
            if backoff(&weak, &mut failures).await {
              continue;
            }
            return;
          }
        }
      }
      None => return,
    };

    // Await the handshake, bounded by the connect watchdog and pre-emptible by a
    // page-lifecycle wake: a `ready` promise that never settles — a handshake
    // stalled after the network dropped under a suspended tab — would otherwise
    // park the supervisor here forever.
    let ready_bounded = match weak.upgrade() {
      Some(inner) => match inner.config.reconnect.connect_timeout {
        Some(timeout) => js_sys::Promise::race(&js_sys::Array::of2(&ready, &timeout_reject_promise(timeout))),
        None => ready,
      },
      None => return,
    };
    match await_or_wake(&weak, ready_bounded).await {
      Raced::Woken => {
        match weak.upgrade() {
          Some(inner) if !inner.manual_close.get() => {
            if let Some(transport) = inner.transport.borrow().as_ref() {
              transport.close();
            }
            failures = 0;
            inner.update_state(WebTransportState::Connecting);
          }
          Some(inner) => {
            inner.update_state(WebTransportState::Closed);
            return;
          }
          None => return,
        }
        continue;
      }
      Raced::Settled(Ok(_)) => {}
      Raced::Settled(Err(e)) => {
        match weak.upgrade() {
          Some(inner) if !inner.manual_close.get() => {
            log::error!("WebTransport ready failed or timed out: {e:?}");
            inner.update_state(WebTransportState::Failed);
          }
          _ => return,
        }
        if backoff(&weak, &mut failures).await {
          continue;
        }
        return;
      }
    }

    // Session is live.
    match weak.upgrade() {
      Some(inner) if !inner.manual_close.get() => {
        failures = 0;
        inner.update_state(WebTransportState::Open);
        ensure_reader(&inner, &transport);
      }
      _ => return,
    }

    // Await teardown, but let a page-lifecycle wake pre-empt it: on a bfcache
    // restore the `closed` promise may never resolve, so without this the
    // supervisor would park here forever on a dead session.
    let closed_res = match await_or_wake(&weak, closed).await {
      Raced::Woken => {
        // The current session is stale/forced-stale. Close it, reset backoff,
        // and reconnect immediately.
        match weak.upgrade() {
          Some(inner) if !inner.manual_close.get() => {
            if let Some(transport) = inner.transport.borrow().as_ref() {
              transport.close();
            }
            failures = 0;
            inner.update_state(WebTransportState::Connecting);
          }
          Some(inner) => {
            inner.update_state(WebTransportState::Closed);
            return;
          }
          None => return,
        }
        continue;
      }
      Raced::Settled(res) => res,
    };
    match weak.upgrade() {
      Some(inner) => {
        if inner.manual_close.get() {
          inner.update_state(WebTransportState::Closed);
          return;
        }
        match closed_res {
          Ok(_) => {
            // A graceful close from either peer is final.
            inner.update_state(WebTransportState::Closed);
            return;
          }
          Err(e) => {
            log::warn!("WebTransport closed with error: {e:?}");
            inner.update_state(WebTransportState::Failed);
          }
        }
      }
      None => return,
    }

    if backoff(&weak, &mut failures).await {
      continue;
    }
    return;
  }
}

/// If the policy permits another attempt, wait the backoff delay. Returns
/// whether the supervisor should loop again (and build a fresh session).
///
/// A page-lifecycle wake pre-empts the wait and resets the backoff, so a
/// restored page retries at once instead of sitting out a long capped delay.
async fn backoff(weak: &Weak<WebTransportInner>, failures: &mut u32) -> bool {
  let delay = match weak.upgrade() {
    Some(inner) if !inner.manual_close.get() && inner.config.reconnect.should_retry(*failures) => {
      inner.config.reconnect.delay_for_attempt(*failures)
    }
    _ => return false,
  };
  *failures += 1;

  if let Raced::Woken = await_or_wake(weak, sleep_promise(delay)).await {
    *failures = 0;
  }

  matches!(weak.upgrade(), Some(inner) if !inner.manual_close.get())
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

/// A promise that resolves after `duration` using `setTimeout`.
fn sleep_promise(duration: Duration) -> js_sys::Promise {
  let millis = duration.as_millis().min(i32::MAX as u128) as i32;
  js_sys::Promise::new(&mut |resolve, _reject| {
    let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis);
  })
}

/// A promise that *rejects* after `duration`, used to bound an otherwise
/// unbounded handshake by racing it against `ready`.
fn timeout_reject_promise(duration: Duration) -> js_sys::Promise {
  let millis = duration.as_millis().min(i32::MAX as u128) as i32;
  js_sys::Promise::new(&mut |_resolve, reject| {
    let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(&reject, millis);
  })
}

impl WebTransportHandle {
  /// Clone the current underlying [`WebTransport`] for advanced use cases.
  ///
  /// Returns `None` before the first session is established. The returned object
  /// refers to the session in use at call time; after a reconnect the handle
  /// points at a different session.
  pub fn raw(&self) -> Option<WebTransport> {
    self.inner.transport.borrow().clone()
  }

  /// Close the session immediately. Treated as intentional: reconnection is
  /// suppressed.
  pub fn close(&self) {
    self.inner.manual_close.set(true);
    if let Some(transport) = self.inner.transport.borrow().as_ref() {
      transport.close();
    }
  }

  /// Close the session with an explicit close code and reason. Like
  /// [`Self::close`], this suppresses reconnection.
  pub fn close_with_info(&self, info: &WebTransportCloseInfo) {
    self.inner.manual_close.set(true);
    if let Some(transport) = self.inner.transport.borrow().as_ref() {
      transport.close_with_close_info(info);
    }
  }

  /// Send a single datagram. Acquires the duplex writer for the call and
  /// releases it before returning.
  pub async fn send_datagram(&self, data: &[u8]) -> CResult<()> {
    let transport = self
      .inner
      .transport
      .borrow()
      .clone()
      .ok_or_else(|| ClientError::from_str("WebTransport session is not connected"))?;
    let writable: WritableStream = transport.datagrams().writable();
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
    let transport = self
      .inner
      .transport
      .borrow()
      .clone()
      .ok_or_else(|| ClientError::from_str("WebTransport session is not connected"))?;
    let promise: js_sys::Promise = transport.create_bidirectional_stream().unchecked_into();
    let val = JsFuture::from(promise)
      .await
      .map_err(|e| ClientError::from_str(format!("Failed to open bidirectional stream: {e:?}")))?;
    val
      .dyn_into::<WebTransportBidirectionalStream>()
      .map_err(|_| ClientError::from_str("Unexpected bidirectional stream type"))
  }

  /// Open a new outbound unidirectional stream.
  pub async fn open_unidirectional_stream(&self) -> CResult<WebTransportSendStream> {
    let transport = self
      .inner
      .transport
      .borrow()
      .clone()
      .ok_or_else(|| ClientError::from_str("WebTransport session is not connected"))?;
    let promise: js_sys::Promise = transport.create_unidirectional_stream().unchecked_into();
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
      return Err(ClientError::from_str(
        "datagram_signal already registered for this handle",
      ));
    }
    let (sig, set_sig) = signal::<Option<Vec<u8>>>(None);
    *self.inner.datagram_sink.borrow_mut() = Some(set_sig);

    // If the session is already open the supervisor has passed the point where
    // it would have started a reader, so start one now. Otherwise the
    // supervisor will start it when the session opens.
    if self.state.get_untracked() == WebTransportState::Open
      && let Some(transport) = self.inner.transport.borrow().clone()
    {
      ensure_reader(&self.inner, &transport);
    }
    Ok(sig)
  }
}
