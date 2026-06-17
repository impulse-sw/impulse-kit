#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

const BASE_CLASSES: &str = "placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input flex min-h-[80px] w-full rounded-md border bg-transparent px-3 py-2 text-base shadow-xs transition-[color,box-shadow] outline-none disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive";

#[component]
pub fn Textarea(
  #[prop(into, optional)] class: String,
  #[prop(optional)] value: RwSignal<String>,
  #[prop(optional, into)] placeholder: String,
  #[prop(optional)] disabled: bool,
  #[prop(optional)] rows: Option<i32>,
) -> impl IntoView {
  view! {
    <textarea
      data-slot="textarea"
      class=cn(&[BASE_CLASSES, class.as_str()])
      prop:value=value
      placeholder=placeholder
      disabled=disabled
      rows=rows.unwrap_or(4)
      on:input:target=move |ev| {
        value.set(ev.target().value());
      }
    />
  }
}
