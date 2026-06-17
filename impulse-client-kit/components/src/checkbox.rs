#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Checkbox(
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
      role="checkbox"
      aria-checked=move || if checked.get() { "true" } else { "false" }
      data-state=move || if checked.get() { "checked" } else { "unchecked" }
      data-slot="checkbox"
      disabled=disabled
      class=cn(
        &[
          "peer border-input dark:bg-input/30 data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground dark:data-[state=checked]:bg-primary data-[state=checked]:border-primary focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive size-4 shrink-0 rounded-[4px] border shadow-xs transition-shadow outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50",
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
        data-slot="checkbox-indicator"
        class="grid place-content-center text-current transition-none"
      >
        {move || {
          if checked.get() {
            view! {
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="size-3.5"
              >
                <path d="M20 6 9 17l-5-5" />
              </svg>
            }
              .into_any()
          } else {
            ().into_any()
          }
        }}
      </span>
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
