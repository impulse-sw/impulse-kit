#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Switch(
  #[prop(optional, into)] checked: Option<RwSignal<bool>>,
  #[prop(optional, into)] default_checked: bool,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] class: String,
  #[prop(optional)] on_change: Option<Callback<bool>>,
) -> impl IntoView {
  let checked = checked.unwrap_or_else(|| RwSignal::new(default_checked));

  let handle_change = move || {
    if !disabled {
      let new_value = !checked.get();
      checked.set(new_value);
      if let Some(callback) = &on_change {
        callback.run(new_value);
      }
    }
  };

  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    if ev.key() == " " || ev.key() == "Enter" {
      ev.prevent_default();
      handle_change();
    }
  };

  view! {
    <button
      type="button"
      role="switch"
      aria-checked=move || if checked.get() { "true" } else { "false" }
      data-state=move || if checked.get() { "checked" } else { "unchecked" }
      data-slot="switch"
      disabled=disabled
      class=cn(
        &[
          "peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-primary data-[state=unchecked]:bg-input",
          class.as_str(),
        ],
      )
      on:click=move |ev| {
        ev.prevent_default();
        handle_change();
      }
      on:keydown=handle_keydown
      tabindex=if disabled { "-1" } else { "0" }
    >
      <span
        data-slot="switch-thumb"
        data-state=move || if checked.get() { "checked" } else { "unchecked" }
        class=cn(&[
          "pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg ring-0 transition-transform data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0",
        ])
      />
      <input
        type="checkbox"
        checked=move || checked.get()
        disabled=disabled
        class="sr-only"
        tabindex="-1"
        aria-hidden="true"
      />
    </button>
  }
}
