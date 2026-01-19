#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn InputGroup(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="input-group" class=cn(&["relative flex items-center gap-0", class.as_str()])>
      {children()}
    </div>
  }
}

const INPUT_BASE_CLASSES: &str = "file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input h-9 w-full min-w-0 border bg-transparent px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive has-[+[data-slot=input-group-addon]]:rounded-r-none has-[[data-slot=input-group-addon]+&]:rounded-l-none has-[[data-slot=input-group-addon]+&]:border-l-0 has-[+[data-slot=input-group-addon]]:border-r-0";

#[component]
pub fn InputGroupInput(
  #[prop(into, optional)] class: String,
  #[prop(into, optional)] r#type: String,
  #[prop(optional)] value: RwSignal<String>,
) -> impl IntoView {
  view! {
    <input
      data-slot="input-group-control"
      type=r#type
      class=cn(&[INPUT_BASE_CLASSES, "rounded-md", class.as_str()])
      prop:value=value
      on:input:target=move |ev| {
        value.set(ev.target().value());
      }
    />
  }
}

#[component]
pub fn InputGroupAddon(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="input-group-addon"
      class=cn(
        &[
          "border-input bg-muted text-muted-foreground flex h-9 items-center gap-2 border px-3 text-sm has-[+[data-slot=input-group-control]]:rounded-l-md has-[+[data-slot=input-group-control]]:border-r-0 has-[[data-slot=input-group-control]+&]:rounded-r-md has-[[data-slot=input-group-control]+&]:border-l-0",
          class.as_str(),
        ],
      )
    >

      {children()}
    </div>
  }
}

#[component]
pub fn InputGroupText(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <span data-slot="input-group-text" class=cn(&["text-sm", class.as_str()])>
      {children()}
    </span>
  }
}

#[component]
pub fn InputGroupButton(
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] r#type: String,
  children: Children,
) -> impl IntoView {
  let button_type = if r#type.is_empty() {
    "button".to_string()
  } else {
    r#type
  };

  view! {
    <button
      data-slot="input-group-button"
      type=button_type
      class=cn(
        &[
          "border-input bg-muted text-muted-foreground hover:bg-accent hover:text-accent-foreground focus-visible:ring-ring/50 inline-flex h-9 items-center justify-center gap-2 whitespace-nowrap border px-3 text-sm font-medium transition-colors outline-none focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50 has-[+[data-slot=input-group-control]]:rounded-l-md has-[+[data-slot=input-group-control]]:border-r-0 has-[[data-slot=input-group-control]+&]:rounded-r-md has-[[data-slot=input-group-control]+&]:border-l-0",
          class.as_str(),
        ],
      )
    >

      {children()}
    </button>
  }
}

const TEXTAREA_BASE_CLASSES: &str = "placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input min-h-[80px] w-full min-w-0 rounded-md border bg-transparent px-3 py-2 text-base shadow-xs transition-[color,box-shadow] outline-none disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive has-[+[data-slot=input-group-addon]]:rounded-r-none has-[[data-slot=input-group-addon]+&]:rounded-l-none has-[[data-slot=input-group-addon]+&]:border-l-0 has-[+[data-slot=input-group-addon]]:border-r-0";

#[component]
pub fn InputGroupTextarea(
  #[prop(into, optional)] class: String,
  #[prop(optional)] value: RwSignal<String>,
) -> impl IntoView {
  view! {
    <textarea
      data-slot="input-group-control"
      class=cn(&[TEXTAREA_BASE_CLASSES, class.as_str()])
      prop:value=value
      on:input:target=move |ev| {
        value.set(ev.target().value());
      }
    />
  }
}
