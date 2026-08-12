//! Going back: the Android system button, the browser's Back, the Escape key.
//!
//! An app made of panels, sheets, editors and drill-down pages has a "back" in
//! it whether or not anything on screen says so — every one of those opens over
//! what came before, and closing it is the only way out. On the web the browser
//! lends its own Back button to that; inside a Tauri window on Android the
//! system button *is* the only one the user has, and left unhandled it does not
//! close the panel — it closes the app.
//!
//! [`BackGuard`] is how a layer says "if the user asks to go back, this is what
//! back means here". Put one next to whatever state opens the layer:
//!
//! ```rust,ignore
//! let open = RwSignal::new(false);
//! view! {
//!   <BackGuard when=open on_back=Callback::new(move |_| open.set(false)) />
//!   <Show when=move || open.get()>…</Show>
//! }
//! ```
//!
//! Nesting works without any coordination: guards form a stack, and one press
//! closes exactly the layer on top. Nothing needs to be mounted at the app root
//! — the first guard to open installs the listeners.
//!
//! ## How it reaches the Android button
//!
//! There is no Android API in here, and there doesn't need to be one. An open
//! guard pushes a history entry, and Tauri's activity already routes the system
//! back button to the webview's history (`canGoBack()` → `goBack()`) before it
//! considers finishing the activity. So a pressed back button arrives as an
//! ordinary `popstate`, the top guard closes, and the app stays open; press it
//! with nothing open and there is no entry to pop, so the app closes — which is
//! what the button is for. The browser's Back and Escape land in the same place,
//! which is why this is worth having on the web too.
//!
//! A layer closed by its own button (an ✕, a menu item, picking a value)
//! unwinds its entry as it goes, so a later back press is never spent on
//! something already closed.
//!
//! ## Why the history is reconciled rather than driven
//!
//! The stack is the truth, and the history is made to match it once per task —
//! never a push here and a `go()` there as each guard opens and closes. That
//! indirection is the whole design, because the two halves of the browser API
//! run on different clocks: `pushState` takes effect immediately, while `go()`
//! is a *queued traversal* whose delta is resolved against the entry that was
//! current when it was called, not the one current when it lands.
//!
//! One gesture routinely closes one layer and opens another in the same tick —
//! leaving a document for another tab closes the document's guard and opens the
//! tab's — and driving the history directly meant that tick issued a `go(-1)`
//! and a `pushState` together. The traversal then landed one entry *below* the
//! entry just pushed, the `popstate` read a depth shallower than the stack, and
//! the guard that had only just opened was closed as though the user had asked
//! for it: the new tab appeared and snapped straight back. Which of the two
//! orders a tick happened to take decided whether it broke, and engines differ
//! on how several queued traversals coalesce, so the same build misbehaved on
//! one platform and looked fine on another.
//!
//! Reconciling removes the question. Guards only add and remove slots;
//! [`sync`](imp) then compares the depth recorded on the current entry with the
//! stack's and closes the gap with pushes or with a single `go()` — and never
//! while a traversal of its own is still in flight. A tick that closes one layer
//! and opens another therefore nets out to no history traffic at all.
//!
//! One constraint follows from using history state: the entries pushed here
//! carry `{ ikBackDepth: n }` as their state and keep the current URL. That is
//! invisible to an app that navigates by signals — every app in this workspace —
//! but it is not something to combine with a router that keeps state of its own
//! in the same entries.

#[cfg(any(feature = "csr", feature = "hydrate"))]
mod imp {
  use std::cell::RefCell;

  use leptos::prelude::*;
  use leptos::wasm_bindgen::prelude::Closure;
  use leptos::wasm_bindgen::{JsCast, JsValue};

  /// Where a pushed entry records how many layers were open when it was pushed.
  /// Namespaced because a history entry's state is shared with whoever else
  /// writes one.
  const DEPTH_KEY: &str = "ikBackDepth";

  /// One registered layer. `close` is `None` once the layer has gone away by
  /// other means and only its history entry is left to unwind.
  struct Slot {
    id: u64,
    close: Option<Callback<()>>,
    escape: bool,
  }

  #[derive(Default)]
  struct Stack {
    listening: bool,
    next_id: u64,
    /// Open layers, oldest first. The index of a slot **is** the depth its
    /// history entry records, which is what lets a `popstate` say how many
    /// layers the user just asked to close.
    slots: Vec<Slot>,
    /// A traversal [`sync`] asked for and has not seen land yet. While one is in
    /// flight the current entry is not the one the history will settle on, so
    /// there is nothing useful to reconcile against — and pushing now is exactly
    /// the mistake the module docs describe.
    traversing: bool,
    /// Lets a traversal that never reports back stop wedging the flag; see
    /// [`begin_traversal`].
    traversal_timeout: Option<TimeoutHandle>,
    /// Whether a [`sync`] is already queued for the end of this task, so a tick
    /// touching several guards still reconciles once.
    syncing: bool,
  }

  thread_local! {
    static STACK: RefCell<Stack> = RefCell::new(Stack::default());
  }

  fn history() -> Option<web_sys::History> {
    window().history().ok()
  }

  /// The state object marking an entry as ours, at `depth`.
  fn depth_state(depth: usize) -> JsValue {
    let state = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
      &state,
      &JsValue::from_str(DEPTH_KEY),
      &JsValue::from_f64(depth as f64),
    );
    state.into()
  }

  /// How many layers the entry the page is sitting on was pushed with. An entry
  /// nobody here pushed — the one the app started on, or one from a router —
  /// reads as 0, i.e. "nothing open".
  fn current_depth() -> usize {
    let Some(state) = history().and_then(|h| h.state().ok()) else {
      return 0;
    };
    js_sys::Reflect::get(&state, &JsValue::from_str(DEPTH_KEY))
      .ok()
      .and_then(|depth| depth.as_f64())
      .map(|depth| depth as usize)
      .unwrap_or(0)
  }

  /// Adds a layer. Returns the registration to hand back to [`unregister`].
  ///
  /// The history entry it needs is not pushed here — [`sync`] gives it one at
  /// the end of the task, together with whatever else the same tick changed.
  fn register(close: Callback<()>, escape: bool) -> u64 {
    listen_once();
    let id = STACK.with_borrow_mut(|stack| {
      stack.next_id += 1;
      let id = stack.next_id;
      stack.slots.push(Slot {
        id,
        close: Some(close),
        escape,
      });
      id
    });
    schedule_sync();
    id
  }

  /// Drops a layer that closed on its own.
  ///
  /// Slots are only dropped from the top: a layer closing out of order (an
  /// inner one still open above it) stays as a dead slot, to be swept together
  /// with the ones above it when they go. Every press of Back then still closes
  /// something, which is the property worth preserving. The entries the swept
  /// slots held are given back by [`sync`], not here.
  fn unregister(id: u64) {
    STACK.with_borrow_mut(|stack| {
      let Some(index) = stack.slots.iter().position(|slot| slot.id == id) else {
        return;
      };
      stack.slots[index].close = None;
      while stack.slots.last().is_some_and(|slot| slot.close.is_none()) {
        stack.slots.pop();
      }
    });
    schedule_sync();
  }

  /// Queues the reconcile that follows a batch of stack changes, once per task.
  ///
  /// A microtask, so it runs after the whole of the effect flush that opened and
  /// closed the layers — the point of coalescing is that a tab switch, which
  /// closes one guard and opens another, is reconciled as the single net change
  /// it is rather than as two opposed history operations racing each other.
  fn schedule_sync() {
    let queued = STACK.with_borrow_mut(|stack| std::mem::replace(&mut stack.syncing, true));
    if queued {
      return;
    }
    queue_microtask(|| {
      STACK.with_borrow_mut(|stack| stack.syncing = false);
      sync();
    });
  }

  /// Makes the history match the stack: one entry per slot, and the current
  /// entry recording exactly as many as there are.
  ///
  /// This is the only place that pushes or traverses.
  fn sync() {
    let Some(history) = history() else {
      return;
    };
    // A traversal of ours in flight means the current entry is about to change
    // anyway. Its `popstate` runs this again, against the entry it lands on.
    let Some(target) = STACK.with_borrow(|stack| (!stack.traversing).then_some(stack.slots.len())) else {
      return;
    };
    let current = current_depth();
    if current < target {
      // No URL: the address bar has nothing to say about a panel being open, and
      // rewriting it would make a reload land somewhere that doesn't exist. Only
      // the entries themselves matter.
      for depth in (current + 1)..=target {
        let _ = history.push_state_with_url(&depth_state(depth), "", None);
      }
    } else if current > target {
      // One traversal for the whole distance, rather than a step per layer: two
      // queued `go()`s are two chances for the history to move somewhere neither
      // of them meant, and engines disagree about where that is.
      begin_traversal();
      let _ = history.go_with_delta(-((current - target) as i32));
    }
  }

  /// Marks a traversal of ours as in flight and arms a fallback in case its
  /// `popstate` never arrives.
  ///
  /// Nothing should be able to swallow it — the entries we traverse to are ones
  /// we pushed, and back entries are never dropped from under us — but a flag
  /// stuck on would leave [`sync`] refusing to push for the rest of the session,
  /// and a layer with no entry behind it is one the Android button closes the
  /// app instead of. The fallback is generous: a same-document traversal takes
  /// milliseconds, so a second of silence means it is not coming.
  fn begin_traversal() {
    let handle = set_timeout_with_handle(
      || {
        STACK.with_borrow_mut(|stack| {
          stack.traversing = false;
          stack.traversal_timeout = None;
        });
        sync();
      },
      std::time::Duration::from_secs(1),
    )
    .ok();
    STACK.with_borrow_mut(|stack| {
      stack.traversing = true;
      stack.traversal_timeout = handle;
    });
  }

  /// The traversal landed; stand the fallback down.
  fn end_traversal() {
    STACK.with_borrow_mut(|stack| {
      stack.traversing = false;
      if let Some(handle) = stack.traversal_timeout.take() {
        handle.clear();
      }
    });
  }

  /// A back gesture: close every layer the entry we landed on is below.
  fn on_popstate() {
    if STACK.with_borrow(|stack| stack.traversing) {
      // Our own reconcile arriving, not the user asking for anything. The stack
      // was trimmed before the traversal was issued; there is nothing to close.
      end_traversal();
      schedule_sync();
      return;
    }
    let depth = current_depth();
    let closing = STACK.with_borrow_mut(|stack| {
      let mut closing = Vec::new();
      while stack.slots.len() > depth {
        if let Some(slot) = stack.slots.pop()
          && let Some(close) = slot.close
        {
          closing.push(close);
        }
      }
      closing
    });
    // Innermost first, and outside the borrow: closing a layer runs app code,
    // which may well open or close another one.
    for close in closing {
      close.run(());
    }
    // Landing deeper than the stack reaches — on an entry this app pushed in an
    // earlier life, before a reload, or one the user came back to with Forward —
    // closes nothing, and stopping there would leave Back doing nothing at all.
    // The reconcile walks the rest of the way in one traversal; the oldest entry
    // has nowhere to go, which on Android is what closes the app.
    schedule_sync();
  }

  /// Escape, on a desktop where there is no back button to press. Closes the top
  /// layer directly, the way its own ✕ would; the entry it was holding is given
  /// back by the reconcile that follows. Going through `history.back()` instead
  /// would be one more caller racing [`sync`] for the same entry, and there is
  /// nothing left for it to buy.
  fn on_keydown(ev: web_sys::KeyboardEvent) {
    if ev.key() != "Escape" || ev.default_prevented() {
      return;
    }
    let close = STACK.with_borrow(|stack| stack.slots.last().filter(|slot| slot.escape).and_then(|slot| slot.close));
    if let Some(close) = close {
      close.run(());
    }
  }

  /// Installs the two listeners, once per document. They are deliberately never
  /// removed: they belong to the page, not to whichever layer happened to open
  /// first, and the stack they read is empty when nothing is open.
  fn listen_once() {
    let first = STACK.with_borrow_mut(|stack| !std::mem::replace(&mut stack.listening, true));
    if !first {
      return;
    }
    listen("popstate", |_| on_popstate());
    listen("keydown", |ev| {
      if let Ok(ev) = ev.dyn_into::<web_sys::KeyboardEvent>() {
        on_keydown(ev);
      }
    });
  }

  fn listen(event: &str, handler: impl Fn(web_sys::Event) + 'static) {
    let handler = Closure::<dyn Fn(web_sys::Event)>::new(handler).into_js_value();
    let _ = window().add_event_listener_with_callback(event, handler.unchecked_ref());
  }

  /// Runs `f` once the current task is done — after the effect flush that
  /// scheduled it, and before the browser gets to render or to deliver events.
  fn queue_microtask(f: impl FnOnce() + 'static) {
    let f = Closure::once_into_js(f);
    window().queue_microtask(f.unchecked_ref());
  }

  /// The hook behind [`BackGuard`](super::BackGuard), for a call site that
  /// already has somewhere to put it.
  ///
  /// `when` is the layer's own open state — the same signal the view reads —
  /// and `on_back` is called when the user asks to go back to what was there
  /// before. Closing the layer any other way (setting `when` false) is
  /// accounted for; `on_back` is not called then.
  pub fn use_back_guard(when: Signal<bool>, on_back: Callback<()>, escape: bool) {
    let registered = StoredValue::new(None::<u64>);
    Effect::new(move |_| {
      if when.get() {
        if registered.get_value().is_none() {
          registered.set_value(Some(register(on_back, escape)));
        }
      } else if let Some(id) = registered.get_value() {
        registered.set_value(None);
        unregister(id);
      }
    });
    // A layer can also close by ceasing to exist — the tab it lived on was
    // switched away from, the page signed out. Its entry is spent all the same.
    on_cleanup(move || {
      if let Some(id) = registered.try_get_value().flatten() {
        unregister(id);
      }
    });
  }
}

#[cfg(feature = "ssr")]
mod imp {
  use leptos::prelude::*;

  /// See the `csr`/`hydrate` `use_back_guard` — there is no history to push on
  /// the server, so this registers nothing.
  pub fn use_back_guard(_when: Signal<bool>, _on_back: Callback<()>, _escape: bool) {}
}

use leptos::prelude::*;

pub use imp::use_back_guard;

/// Declares what "back" means while a layer is open.
///
/// Renders nothing — put it beside the layer it guards, anywhere in the view:
///
/// ```rust,ignore
/// // A panel, a sheet, an editor: anything with an open state.
/// <BackGuard when=open on_back=Callback::new(move |_| open.set(false)) />
///
/// // A drill-down page keyed by what it is showing.
/// <BackGuard
///   when=Signal::derive(move || editing.get().is_some())
///   on_back=Callback::new(move |_| editing.set(None))
/// />
/// ```
///
/// * `when` — whether the layer is open. Guards stack in the order they open,
///   and a back gesture closes only the innermost.
/// * `on_back` — what closing means. Called for a back gesture only; a layer
///   closed by its own controls doesn't need to hear about itself.
/// * `escape` — also close on the Escape key (default). Turn it off where
///   Escape already means something else inside the layer.
#[component]
pub fn BackGuard(
  #[prop(into)] when: Signal<bool>,
  #[prop(into)] on_back: Callback<()>,
  #[prop(default = true)] escape: bool,
) -> impl IntoView {
  use_back_guard(when, on_back, escape);
}
