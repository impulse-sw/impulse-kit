#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

const BASE_CLASSES: &str = "border-input bg-background ring-ring/50 flex h-9 w-full items-center justify-between gap-2 rounded-md border px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm focus-visible:border-ring focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive [&>select]:appearance-none [&>select]:bg-transparent [&>select]:pr-2 [&>select]:outline-none [&>select]:w-full";

#[component]
pub fn NativeSelect(
  #[prop(optional, into)] class: String,
  #[prop(optional)] value: RwSignal<String>,
  #[prop(optional, into)] name: String,
  #[prop(optional)] disabled: bool,
  children: Children,
) -> impl IntoView {
  view! {
    <div data-slot="native-select" class=cn(&[BASE_CLASSES, class.as_str()])>
      <select
        name=name
        disabled=disabled
        prop:value=value
        on:change:target=move |ev| {
          value.set(ev.target().value());
        }
      >

        {children()}
      </select>
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="text-muted-foreground size-4 shrink-0 opacity-50"
      >
        <path d="m6 9 6 6 6-6" />
      </svg>
    </div>
  }
}

#[component]
pub fn NativeSelectOption(
  #[prop(into)] value: String,
  #[prop(optional)] disabled: bool,
  children: Children,
) -> impl IntoView {
  view! {
    <option data-slot="native-select-option" value=value disabled=disabled>
      {children()}
    </option>
  }
}

#[component]
pub fn NativeSelectOptGroup(#[prop(into)] label: String, children: Children) -> impl IntoView {
  view! {
    <optgroup data-slot="native-select-optgroup" label=label>
      {children()}
    </optgroup>
  }
}
