#![allow(missing_docs, dead_code)]

//! A Stripe-style perspective "error wobble".
//!
//! Two ways to use it:
//!
//! * Declarative [`Shake`] wrapper — replays the animation whenever the value
//!   of `signal` changes to an "active" value (a non-empty `String`, `true`,
//!   `Some(_)`, a non-zero counter, …):
//!
//!   ```ignore
//!   let error = RwSignal::new(String::new());
//!   view! { <Shake signal=error><LoginForm /></Shake> }
//!   ```
//!
//! * Imperative [`use_shake`] hook — attach the returned `node_ref` to your own
//!   element and call `shake.run(())` to replay it. Render [`ShakeStyles`] once
//!   yourself when going this route:
//!
//!   ```ignore
//!   let ShakeHandle { node_ref, shake } = use_shake();
//!   view! { <ShakeStyles /><form node_ref=node_ref>…</form> }
//!   ```

use impulse_client_kit::utils::cn;
use leptos::prelude::*;
use web_sys::HtmlElement;

/// The keyframes backing the `.shake-error` class. Rendered automatically by
/// [`Shake`]; render it yourself once when driving the animation through
/// [`use_shake`] on your own element.
const SHAKE_STYLES: &str = r#"
@keyframes shake-error {
  0% { transform: perspective(700px) translate3d(0, 0, 0) rotateY(0deg); }
  12% { transform: perspective(700px) translate3d(-7px, 0, 0) rotateY(-5deg); }
  26% { transform: perspective(700px) translate3d(6px, 0, 0) rotateY(4deg); }
  41% { transform: perspective(700px) translate3d(-5px, 0, 0) rotateY(-3deg); }
  56% { transform: perspective(700px) translate3d(4px, 0, 0) rotateY(2deg); }
  70% { transform: perspective(700px) translate3d(-2px, 0, 0) rotateY(-1deg); }
  84% { transform: perspective(700px) translate3d(1px, 0, 0) rotateY(0.5deg); }
  100% { transform: perspective(700px) translate3d(0, 0, 0) rotateY(0deg); }
}

.shake-error {
  animation: shake-error 0.5s cubic-bezier(0.36, 0.07, 0.19, 0.97) both;
  transform-style: preserve-3d;
  backface-visibility: hidden;
}

@media (prefers-reduced-motion: reduce) {
  .shake-error { animation: none; }
}
"#;

/// Injects the `shake-error` keyframes. Safe to render more than once.
#[component]
pub fn ShakeStyles() -> impl IntoView {
  view! { <style inner_html=SHAKE_STYLES></style> }
}

/// Handle returned by [`use_shake`]: attach `node_ref` to an element and call
/// `shake.run(())` to replay the wobble.
#[derive(Clone, Copy)]
pub struct ShakeHandle {
  pub node_ref: NodeRef<leptos::html::Div>,
  pub shake: Callback<()>,
}

/// Imperative API. Attach the returned `node_ref` to a `<div>` and call
/// `shake.run(())` to replay the animation. Remember to render [`ShakeStyles`]
/// once when using the hook on its own.
pub fn use_shake() -> ShakeHandle {
  let node_ref = NodeRef::<leptos::html::Div>::new();
  let shake = Callback::new(move |_: ()| {
    if let Some(el) = node_ref.get_untracked() {
      let el: &HtmlElement = el.as_ref();
      let class_list = el.class_list();
      let _ = class_list.remove_1("shake-error");
      // Force a reflow so re-adding the class restarts the animation every time.
      let _ = el.offset_width();
      let _ = class_list.add_1("shake-error");
    }
  });
  ShakeHandle { node_ref, shake }
}

/// Values that can drive a [`Shake`]. A shake fires when the value *changes* to
/// one that is "active" (mirrors the truthiness check of the original React
/// component).
pub trait ShakeTrigger: PartialEq + Clone + Send + Sync + 'static {
  /// Whether a transition *into* this value should replay the shake.
  fn shake_active(&self) -> bool;
}

impl ShakeTrigger for bool {
  fn shake_active(&self) -> bool {
    *self
  }
}

impl ShakeTrigger for String {
  fn shake_active(&self) -> bool {
    !self.is_empty()
  }
}

impl ShakeTrigger for u64 {
  fn shake_active(&self) -> bool {
    *self != 0
  }
}

impl ShakeTrigger for usize {
  fn shake_active(&self) -> bool {
    *self != 0
  }
}

impl<T: PartialEq + Clone + Send + Sync + 'static> ShakeTrigger for Option<T> {
  fn shake_active(&self) -> bool {
    self.is_some()
  }
}

/// Declarative wrapper. Replays the shake whenever `signal` changes to an
/// active value (see [`ShakeTrigger`]). Respects `prefers-reduced-motion` via
/// the injected styles.
///
/// The component is generic over the *signal source* `T` (e.g. an `RwSignal`,
/// `ReadSignal`, `Memo` or `Signal`) rather than the value it yields. This
/// lets the value type be deduced from `T`'s associated `Get::Value`, so a call
/// like `<Shake signal=error>` infers without a turbofish. Taking `Signal<S>` with
/// `#[prop(into)]` instead would be ambiguous: `RwSignal<String>` converts into
/// both `Signal<String>` and `Signal<RwSignal<String>>`, and the `ShakeTrigger`
/// bound is not consulted to disambiguate the `into`, which surfaces as `E0283`.
#[component]
pub fn Shake<T>(
  /// The reactive trigger (any readable signal). The shake replays on each change
  /// to an active value.
  signal: T,
  /// Extra classes for the wrapper element.
  #[prop(into, optional)]
  class: String,
  children: Children,
) -> impl IntoView
where
  T: Get + Copy + Send + Sync + 'static,
  T: GetUntracked<Value = <T as Get>::Value>,
  <T as Get>::Value: ShakeTrigger,
{
  let ShakeHandle { node_ref, shake } = use_shake();
  let prev = RwSignal::new(signal.get_untracked());

  Effect::new(move |_| {
    let cur = signal.get();
    if prev.get_untracked() != cur && cur.shake_active() {
      shake.run(());
    }
    prev.set(cur);
  });

  view! {
    <ShakeStyles />
    <div node_ref=node_ref data-slot="shake" class=cn(&[class.as_str()])>
      {children()}
    </div>
  }
}
