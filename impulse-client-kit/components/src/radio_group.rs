#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy)]
struct RadioGroupContext {
  value: RwSignal<String>,
  on_value_change: Option<Callback<String>>,
  disabled: bool,
}

#[component]
pub fn RadioGroup(
  #[prop(optional, into)] value: Option<RwSignal<String>>,
  #[prop(optional, into)] default_value: String,
  #[prop(optional)] on_value_change: Option<Callback<String>>,
  #[prop(optional)] disabled: bool,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let value = value.unwrap_or_else(|| RwSignal::new(default_value));

  provide_context(RadioGroupContext {
    value,
    on_value_change,
    disabled,
  });

  view! {
    <div data-slot="radio-group" role="radiogroup" class=cn(&["grid gap-2", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn RadioGroupItem(
  #[prop(into)] value: String,
  #[prop(optional)] disabled: bool,
  #[prop(into, optional)] class: String,
  #[prop(optional, into)] id: String,
) -> impl IntoView {
  let context = use_context::<RadioGroupContext>().expect("RadioGroupItem must be used within RadioGroup");

  let is_disabled = disabled || context.disabled;
  let value_clone = value.clone();
  let is_checked_memo = Memo::new(move |_| context.value.get() == value_clone);

  let value_for_click = value.clone();
  let handle_click = move |_| {
    if !is_disabled {
      context.value.set(value_for_click.clone());
      if let Some(callback) = context.on_value_change {
        callback.run(value_for_click.clone());
      }
    }
  };

  let value_for_keydown = value.clone();
  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    if (ev.key() == " " || ev.key() == "Enter") && !is_disabled {
      ev.prevent_default();
      context.value.set(value_for_keydown.clone());
      if let Some(callback) = context.on_value_change {
        callback.run(value_for_keydown.clone());
      }
    }
  };

  view! {
    <button
      type="button"
      role="radio"
      id=id
      aria-checked=move || if is_checked_memo.get() { "true" } else { "false" }
      data-state=move || if is_checked_memo.get() { "checked" } else { "unchecked" }
      data-slot="radio-group-item"
      disabled=is_disabled
      class=cn(
        &[
          "border-input text-primary ring-offset-background focus-visible:ring-ring aspect-square h-4 w-4 rounded-full border shadow-xs focus:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
          class.as_str(),
        ],
      )
      on:click=handle_click
      on:keydown=handle_keydown
      tabindex=if is_disabled { "-1" } else { "0" }
    >
      <span
        data-slot="radio-group-indicator"
        data-state=move || if is_checked_memo.get() { "checked" } else { "unchecked" }
        class="flex items-center justify-center"
      >
        {move || {
          if is_checked_memo.get() {
            view! {
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="currentColor"
                class="h-2.5 w-2.5"
              >
                <circle cx="12" cy="12" r="12" />
              </svg>
            }
              .into_any()
          } else {
            ().into_any()
          }
        }}
      </span>
      <input
        type="radio"
        checked=is_checked_memo
        disabled=is_disabled
        class="sr-only"
        tabindex="-1"
        aria-hidden="true"
      />
    </button>
  }
}
