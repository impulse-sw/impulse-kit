//! Closing a panel by clicking away from it.
//!
//! A menu, a popover or an inline editor that only closes through its own
//! controls is a trap on a touch screen: there is no Escape to reach for, the
//! panel covers what you were trying to tap, and the obvious gesture — tapping
//! somewhere else — does nothing. Every overlay in this crate handles that
//! itself; [`use_click_outside`] is the same behaviour for a panel an app builds
//! out of plain markup.

#[cfg(any(feature = "csr", feature = "hydrate"))]
mod imp {
  use leptos::prelude::*;
  use leptos::wasm_bindgen::JsCast;
  use web_sys::Element;

  /// Closes `open` when a click lands outside `container`.
  ///
  /// `container` must wrap **both** the panel and whatever opens it — the usual
  /// `<div class="relative">` around a trigger and its absolutely-positioned
  /// panel. The click that opens the panel reaches the window too, so a trigger
  /// left outside the container would close the panel in the same gesture that
  /// opened it.
  ///
  /// ```rust,ignore
  /// let open = RwSignal::new(false);
  /// let menu = NodeRef::<leptos::html::Div>::new();
  /// use_click_outside(menu, open);
  /// view! {
  ///   <div node_ref=menu class="relative">
  ///     <button on:click=move |_| open.update(|o| *o = !*o)>"Menu"</button>
  ///     <Show when=move || open.get()>
  ///       <div class="absolute right-0">…</div>
  ///     </Show>
  ///   </div>
  /// }
  /// ```
  ///
  /// One listener per call site, for as long as the component lives — rather
  /// than one per opening, which is how this ends up quietly accumulating.
  pub fn use_click_outside(container: NodeRef<leptos::html::Div>, open: RwSignal<bool>) {
    let handle = window_event_listener(leptos::ev::click, move |ev| {
      if !open.get_untracked() {
        return;
      }
      let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
        return;
      };
      let Some(container) = container.get_untracked() else {
        return;
      };
      let container: &Element = container.as_ref();
      if !container.contains(Some(&target)) {
        open.set(false);
      }
    });
    on_cleanup(move || handle.remove());
  }
}

#[cfg(feature = "ssr")]
mod imp {
  use leptos::prelude::*;

  /// See the `csr`/`hydrate` `use_click_outside` — there are no clicks to hear
  /// on the server, so this does nothing.
  pub fn use_click_outside(_container: NodeRef<leptos::html::Div>, _open: RwSignal<bool>) {}
}

pub use imp::use_click_outside;
