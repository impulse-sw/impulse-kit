//! Client-side telemetry.
//!
//! Two complementary ways to collect usage data, both feeding the same
//! collection endpoint (see `impulse-server-kit`'s `telemetry` module):
//!
//! 1. **Monitor components** wrap any view and emit an event on interaction —
//!    [`ClickMonitor`], [`ViewMonitor`] (impressions via `IntersectionObserver`),
//!    [`HoverMonitor`], [`FocusMonitor`], [`SubmitMonitor`] and the generic
//!    [`EventMonitor`]:
//!
//!    ```rust,ignore
//!    view! {
//!      <ClickMonitor message="cta:signup">
//!        <Button>"Sign up"</Button>
//!      </ClickMonitor>
//!    }
//!    ```
//!
//! 2. **Imperative helpers** mirror `tracing`'s ergonomics for ad-hoc logs,
//!    metrics and spans: [`track_event`], [`track_log`], [`track_metric`] and
//!    [`track_span`].
//!
//! # Identity & privacy
//!
//! A [`TelemetryContext`] (provided once near the app root with
//! [`provide_telemetry`]) holds the endpoint and identity. By default it is
//! [`TelemetryMode::Anonymous`]: events carry only a random, session-scoped
//! `session_id`. Switch to [`TelemetryMode::Identified`] (or call
//! [`TelemetryContext::set_user_id`]) once a user is known to additionally
//! attach a `user_id`. In anonymous mode the `user_id` is never sent, even if
//! one is set.
//!
//! ```rust,ignore
//! use impulse_client_kit::telemetry::{provide_telemetry, TelemetryConfig};
//!
//! // near the app root:
//! provide_telemetry(TelemetryConfig::new("/api/telemetry"));
//! ```
//!
//! Events are delivered with `navigator.sendBeacon` (MessagePack body), which is
//! fire-and-forget and survives page unloads. On SSR every helper is a no-op and
//! monitors simply render their children.

use crate::router;
use impulse_utils::telemetry::{TelemetryBatch, TelemetryEvent};
use leptos::prelude::*;

pub use impulse_utils::telemetry::{TelemetryAttr, TelemetryEventKind, TelemetryLevel};

/// Identity mode for collected telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TelemetryMode {
  /// Only a random, session-scoped id is attached; `user_id` is never sent.
  #[default]
  Anonymous,
  /// The configured `user_id` (when set) is attached alongside the session id.
  Identified,
}

/// Configuration used to initialize a [`TelemetryContext`] via [`provide_telemetry`].
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
  /// Collection endpoint path (resolved against the backend with [`router::endpoint`]).
  pub endpoint: String,
  /// Initial identity mode.
  pub mode: TelemetryMode,
  /// Initial user id (only meaningful in [`TelemetryMode::Identified`]).
  pub user_id: Option<String>,
  /// Explicit session id; a random one is generated when `None`.
  pub session_id: Option<String>,
}

impl TelemetryConfig {
  /// Create an anonymous configuration pointing at `endpoint`.
  pub fn new(endpoint: impl Into<String>) -> Self {
    Self {
      endpoint: endpoint.into(),
      mode: TelemetryMode::Anonymous,
      user_id: None,
      session_id: None,
    }
  }

  /// Switch to identified mode with the given user id.
  pub fn identified(mut self, user_id: impl Into<String>) -> Self {
    self.mode = TelemetryMode::Identified;
    self.user_id = Some(user_id.into());
    self
  }

  /// Force anonymous mode and drop any user id.
  pub fn anonymous(mut self) -> Self {
    self.mode = TelemetryMode::Anonymous;
    self.user_id = None;
    self
  }

  /// Pin an explicit session id instead of generating a random one.
  pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
    self.session_id = Some(session_id.into());
    self
  }
}

impl Default for TelemetryConfig {
  fn default() -> Self {
    Self::new("/api/telemetry")
  }
}

/// Handle to the active telemetry configuration.
///
/// Cheap to copy and store; obtain it from context with [`use_telemetry`] or
/// create and publish one with [`provide_telemetry`].
#[derive(Debug, Clone, Copy)]
pub struct TelemetryContext {
  endpoint: StoredValue<String>,
  session_id: StoredValue<String>,
  user_id: RwSignal<Option<String>>,
  mode: RwSignal<TelemetryMode>,
}

impl TelemetryContext {
  /// The session-scoped anonymous identifier.
  pub fn session_id(&self) -> String {
    self.session_id.get_value()
  }

  /// The current identity mode.
  pub fn mode(&self) -> TelemetryMode {
    self.mode.get_untracked()
  }

  /// Set (or clear) the identified user id.
  pub fn set_user_id(&self, user_id: Option<String>) {
    self.user_id.set(user_id);
  }

  /// Set the identity mode.
  pub fn set_mode(&self, mode: TelemetryMode) {
    self.mode.set(mode);
  }

  /// Build an event pre-filled with identity, timestamp and the current path.
  pub fn build_event(&self, kind: TelemetryEventKind, message: Option<String>) -> TelemetryEvent {
    let mut event = TelemetryEvent::new(kind, now_ms());
    event.message = message;
    event.session_id = Some(self.session_id.get_value());
    if self.mode.get_untracked() == TelemetryMode::Identified {
      event.user_id = self.user_id.get_untracked();
    }
    event.path = router::get_path().ok();
    event
  }

  /// Emit a fully-built event to the configured endpoint.
  pub fn emit(&self, event: TelemetryEvent) {
    self.dispatch(event, None);
  }

  /// Track an interaction of `kind` with an optional message and endpoint override.
  pub fn track_to(&self, kind: TelemetryEventKind, message: Option<String>, endpoint_override: Option<String>) {
    let event = self.build_event(kind, message);
    self.dispatch(event, endpoint_override);
  }

  /// Track an interaction of `kind` carrying `message`.
  pub fn track(&self, kind: TelemetryEventKind, message: impl Into<String>) {
    self.track_to(kind, Some(message.into()), None);
  }

  /// Emit a structured log event at `level`.
  pub fn log(&self, level: TelemetryLevel, message: impl Into<String>) {
    let mut event = self.build_event(TelemetryEventKind::Log, Some(message.into()));
    event.level = Some(level);
    self.dispatch(event, None);
  }

  /// Emit a single numeric measurement named `name`.
  pub fn metric(&self, name: impl Into<String>, value: f64) {
    let mut event = self.build_event(TelemetryEventKind::Metric, Some(name.into()));
    event.value = Some(value);
    self.dispatch(event, None);
  }

  /// Begin a timed span; its duration is reported when the returned guard drops.
  pub fn span(&self, name: impl Into<String>) -> TelemetrySpan {
    TelemetrySpan {
      ctx: *self,
      name: name.into(),
      start_ms: now_ms(),
      finished: false,
    }
  }

  fn dispatch(&self, event: TelemetryEvent, endpoint_override: Option<String>) {
    let path = endpoint_override.unwrap_or_else(|| self.endpoint.get_value());
    let url = resolve_endpoint(&path);
    send_batch(&url, &TelemetryBatch::single(event));
  }
}

/// A running, timed telemetry span.
///
/// Reports a [`TelemetryEventKind::Span`] event with its duration (in
/// milliseconds) when [`finish`](Self::finish) is called or when it is dropped.
#[derive(Debug)]
pub struct TelemetrySpan {
  ctx: TelemetryContext,
  name: String,
  start_ms: u64,
  finished: bool,
}

impl TelemetrySpan {
  /// Finish the span now, reporting its duration. Equivalent to dropping it.
  pub fn finish(mut self) {
    self.finish_inner();
  }

  fn finish_inner(&mut self) {
    if self.finished {
      return;
    }
    self.finished = true;
    let duration = now_ms().saturating_sub(self.start_ms) as f64;
    let mut event = self.ctx.build_event(TelemetryEventKind::Span, Some(self.name.clone()));
    event.value = Some(duration);
    self.ctx.emit(event);
  }
}

impl Drop for TelemetrySpan {
  fn drop(&mut self) {
    self.finish_inner();
  }
}

/// Create a [`TelemetryContext`] from `config` and publish it via Leptos context.
///
/// Call once near the application root; descendant components and the imperative
/// helpers then pick it up through [`use_telemetry`].
pub fn provide_telemetry(config: TelemetryConfig) -> TelemetryContext {
  let session_id = config.session_id.unwrap_or_else(random_session_id);
  let ctx = TelemetryContext {
    endpoint: StoredValue::new(config.endpoint),
    session_id: StoredValue::new(session_id),
    user_id: RwSignal::new(config.user_id),
    mode: RwSignal::new(config.mode),
  };
  provide_context(ctx);
  ctx
}

/// Obtain the [`TelemetryContext`] from the reactive context, if one was provided.
pub fn use_telemetry() -> Option<TelemetryContext> {
  use_context::<TelemetryContext>()
}

/// Imperatively track an arbitrary event. No-op when no context is present.
pub fn track_event(kind: TelemetryEventKind, message: impl Into<String>) {
  if let Some(ctx) = use_telemetry() {
    ctx.track(kind, message);
  }
}

/// Imperatively emit a structured log event. No-op when no context is present.
pub fn track_log(level: TelemetryLevel, message: impl Into<String>) {
  if let Some(ctx) = use_telemetry() {
    ctx.log(level, message);
  }
}

/// Imperatively emit a numeric metric. No-op when no context is present.
pub fn track_metric(name: impl Into<String>, value: f64) {
  if let Some(ctx) = use_telemetry() {
    ctx.metric(name, value);
  }
}

/// Begin a timed span using the context from scope, if any.
///
/// Returns `None` when no context is present, so the span (and its report) is
/// skipped entirely.
pub fn track_span(name: impl Into<String>) -> Option<TelemetrySpan> {
  use_telemetry().map(|ctx| ctx.span(name))
}

/// Shared click/hover/focus/submit handler factory.
///
/// Returns a closure usable directly as a Leptos `on:*` handler; the DOM event
/// is ignored, so the same factory serves every bubbling interaction.
fn monitor_handler<E>(
  ctx: Option<TelemetryContext>,
  kind: TelemetryEventKind,
  message: String,
  endpoint: Option<String>,
) -> impl FnMut(E) {
  move |_event: E| {
    if let Some(ctx) = ctx {
      ctx.track_to(kind, Some(message.clone()), endpoint.clone());
    }
  }
}

/// Reports a [`TelemetryEventKind::Click`] when its children are clicked.
///
/// The wrapper uses `display:contents`, so it adds no box of its own.
#[component]
pub fn ClickMonitor(
  /// Message describing the tracked action (e.g. `"cta:signup"`).
  #[prop(into)]
  message: String,
  /// Optional endpoint path override for this monitor.
  #[prop(optional, into)]
  endpoint: Option<String>,
  /// The wrapped view.
  children: Children,
) -> impl IntoView {
  let ctx = use_telemetry();
  view! {
    <span
      style="display:contents"
      on:click=monitor_handler(ctx, TelemetryEventKind::Click, message, endpoint)
    >
      {children()}
    </span>
  }
}

/// Reports a [`TelemetryEventKind::Hover`] when the pointer enters its children.
#[component]
pub fn HoverMonitor(
  /// Message describing the tracked action.
  #[prop(into)]
  message: String,
  /// Optional endpoint path override for this monitor.
  #[prop(optional, into)]
  endpoint: Option<String>,
  /// The wrapped view.
  children: Children,
) -> impl IntoView {
  let ctx = use_telemetry();
  view! {
    <span
      style="display:contents"
      on:mouseenter=monitor_handler(ctx, TelemetryEventKind::Hover, message, endpoint)
    >
      {children()}
    </span>
  }
}

/// Reports a [`TelemetryEventKind::Focus`] when its children gain focus.
#[component]
pub fn FocusMonitor(
  /// Message describing the tracked action.
  #[prop(into)]
  message: String,
  /// Optional endpoint path override for this monitor.
  #[prop(optional, into)]
  endpoint: Option<String>,
  /// The wrapped view.
  children: Children,
) -> impl IntoView {
  let ctx = use_telemetry();
  view! {
    <span
      style="display:contents"
      on:focusin=monitor_handler(ctx, TelemetryEventKind::Focus, message, endpoint)
    >
      {children()}
    </span>
  }
}

/// Reports a [`TelemetryEventKind::Submit`] when a wrapped form is submitted.
#[component]
pub fn SubmitMonitor(
  /// Message describing the tracked action.
  #[prop(into)]
  message: String,
  /// Optional endpoint path override for this monitor.
  #[prop(optional, into)]
  endpoint: Option<String>,
  /// The wrapped view (expected to contain a `<form>`).
  children: Children,
) -> impl IntoView {
  let ctx = use_telemetry();
  view! {
    <span
      style="display:contents"
      on:submit=monitor_handler(ctx, TelemetryEventKind::Submit, message, endpoint)
    >
      {children()}
    </span>
  }
}

/// Reports a [`TelemetryEventKind::View`] the first time its children scroll into view.
///
/// Unlike the other monitors this renders a real `<div>` (an `IntersectionObserver`
/// needs a layout box); pass `class` to style it.
#[component]
pub fn ViewMonitor(
  /// Message describing the tracked impression.
  #[prop(into)]
  message: String,
  /// Optional endpoint path override for this monitor.
  #[prop(optional, into)]
  endpoint: Option<String>,
  /// Classes applied to the wrapper element.
  #[prop(optional, into)]
  class: String,
  /// The wrapped view.
  children: Children,
) -> impl IntoView {
  let node_ref = NodeRef::<leptos::html::Div>::new();

  #[cfg(any(feature = "csr", feature = "hydrate"))]
  {
    let ctx = use_telemetry();
    let message = message.clone();
    let endpoint = endpoint.clone();
    Effect::new(move |_| {
      let (Some(ctx), Some(element)) = (ctx, node_ref.get()) else {
        return;
      };
      observe_view(ctx, element, message.clone(), endpoint.clone());
    });
  }
  #[cfg(feature = "ssr")]
  let _ = (message, endpoint);

  view! {
    <div node_ref=node_ref class=class>
      {children()}
    </div>
  }
}

/// Reports a [`TelemetryEventKind::Custom`] when the named DOM `event` fires on its children.
///
/// Useful for events the dedicated monitors don't cover (e.g. `"change"`,
/// `"pointerdown"`, custom events).
#[component]
pub fn EventMonitor(
  /// DOM event name to listen for (e.g. `"change"`).
  #[prop(into)]
  event: String,
  /// Message describing the tracked action.
  #[prop(into)]
  message: String,
  /// Optional endpoint path override for this monitor.
  #[prop(optional, into)]
  endpoint: Option<String>,
  /// The wrapped view.
  children: Children,
) -> impl IntoView {
  let node_ref = NodeRef::<leptos::html::Div>::new();

  #[cfg(any(feature = "csr", feature = "hydrate"))]
  {
    let ctx = use_telemetry();
    let event = event.clone();
    let message = message.clone();
    let endpoint = endpoint.clone();
    Effect::new(move |_| {
      let (Some(ctx), Some(element)) = (ctx, node_ref.get()) else {
        return;
      };
      add_event_listener(ctx, element, &event, message.clone(), endpoint.clone());
    });
  }
  #[cfg(feature = "ssr")]
  let _ = (event, message, endpoint);

  view! {
    <div node_ref=node_ref style="display:contents">
      {children()}
    </div>
  }
}

// --- Platform helpers -------------------------------------------------------

#[cfg(any(feature = "csr", feature = "hydrate"))]
fn now_ms() -> u64 {
  js_sys::Date::now() as u64
}

#[cfg(feature = "ssr")]
fn now_ms() -> u64 {
  0
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
fn resolve_endpoint(path: &str) -> String {
  router::endpoint(path)
}

#[cfg(feature = "ssr")]
fn resolve_endpoint(path: &str) -> String {
  path.to_string()
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
fn random_session_id() -> String {
  let mut id = String::with_capacity(32);
  for _ in 0..4 {
    let chunk = (js_sys::Math::random() * (u32::MAX as f64)) as u32;
    id.push_str(&format!("{chunk:08x}"));
  }
  id
}

#[cfg(feature = "ssr")]
fn random_session_id() -> String {
  String::new()
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
fn send_batch(url: &str, batch: &TelemetryBatch) {
  use wasm_bindgen::JsCast;

  let bytes = match rmp_serde::to_vec_named(batch) {
    Ok(bytes) => bytes,
    Err(e) => {
      log::warn!("telemetry: failed to encode batch: {e}");
      return;
    }
  };

  let Some(window) = web_sys::window() else {
    return;
  };
  let navigator = window.navigator();

  let parts = js_sys::Array::new();
  parts.push(&js_sys::Uint8Array::from(bytes.as_slice()));

  let options = web_sys::BlobPropertyBag::new();
  options.set_type("application/msgpack");

  let blob = match web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options) {
    Ok(blob) => blob,
    Err(e) => {
      log::warn!("telemetry: failed to build blob: {e:?}");
      return;
    }
  };

  match navigator.send_beacon_with_opt_blob(url, Some(blob.unchecked_ref())) {
    Ok(true) => {}
    Ok(false) => log::warn!("telemetry: beacon to {url} was not queued"),
    Err(e) => log::warn!("telemetry: beacon error: {e:?}"),
  }
}

#[cfg(feature = "ssr")]
fn send_batch(_url: &str, _batch: &TelemetryBatch) {}

#[cfg(any(feature = "csr", feature = "hydrate"))]
fn observe_view(ctx: TelemetryContext, element: web_sys::HtmlDivElement, message: String, endpoint: Option<String>) {
  use std::cell::{Cell, RefCell};
  use std::rc::Rc;
  use wasm_bindgen::JsCast;
  use wasm_bindgen::closure::Closure;

  let observer_slot: Rc<RefCell<Option<web_sys::IntersectionObserver>>> = Rc::new(RefCell::new(None));
  let fired = Rc::new(Cell::new(false));

  let slot = observer_slot.clone();
  let callback = Closure::<dyn FnMut(js_sys::Array)>::new(move |entries: js_sys::Array| {
    if fired.get() {
      return;
    }
    let intersecting = entries.iter().any(|entry| {
      entry
        .dyn_into::<web_sys::IntersectionObserverEntry>()
        .map(|entry| entry.is_intersecting())
        .unwrap_or(false)
    });
    if intersecting {
      fired.set(true);
      ctx.track_to(TelemetryEventKind::View, Some(message.clone()), endpoint.clone());
      if let Some(observer) = slot.borrow().as_ref() {
        observer.disconnect();
      }
    }
  });

  if let Ok(observer) = web_sys::IntersectionObserver::new(callback.as_ref().unchecked_ref()) {
    observer.observe(element.as_ref());
    *observer_slot.borrow_mut() = Some(observer);
  }
  callback.forget();
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
fn add_event_listener(
  ctx: TelemetryContext,
  element: web_sys::HtmlDivElement,
  event: &str,
  message: String,
  endpoint: Option<String>,
) {
  use wasm_bindgen::JsCast;
  use wasm_bindgen::closure::Closure;

  let callback = Closure::<dyn FnMut()>::new(move || {
    ctx.track_to(TelemetryEventKind::Custom, Some(message.clone()), endpoint.clone());
  });
  let _ = element.add_event_listener_with_callback(event, callback.as_ref().unchecked_ref());
  callback.forget();
}
