#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ToastVariant {
  #[default]
  Default,
  Destructive,
}

impl ToastVariant {
  fn class(&self) -> &'static str {
    match self {
      Self::Default => "border bg-background text-foreground",
      Self::Destructive => {
        "destructive group border-destructive bg-destructive text-destructive-foreground"
      }
    }
  }
}

#[component]
pub fn ToastProvider(children: Children) -> impl IntoView {
  view! { <div data-slot="toast-provider">{children()}</div> }
}

#[component]
pub fn ToastViewport(#[prop(into, optional)] class: String) -> impl IntoView {
  view! {
    <div
      data-slot="toast-viewport"
      class=cn(
        &[
          "fixed top-0 z-[100] flex max-h-screen w-full flex-col-reverse p-4 sm:bottom-0 sm:right-0 sm:top-auto sm:flex-col md:max-w-[420px]",
          class.as_str(),
        ],
      )
    />
  }
}

#[component]
pub fn Toast(
  #[prop(optional)] variant: ToastVariant,
  #[prop(into, optional)] class: String,
  #[prop(optional)] open: Option<RwSignal<bool>>,
  children: ChildrenFn,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(true));
  let children = StoredValue::new(children);

  view! {
    <Show when=move || is_open.get()>
      <div
        data-slot="toast"
        data-state=move || if is_open.get() { "open" } else { "closed" }
        class=cn(
          &[
            "group pointer-events-auto relative flex w-full items-center justify-between gap-4 overflow-hidden rounded-md border p-6 pr-8 shadow-lg transition-all data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-80 data-[state=open]:slide-in-from-top-full data-[state=open]:sm:slide-in-from-bottom-full",
            variant.class(),
            class.as_str(),
          ],
        )
      >
        {children.get_value()()}
      </div>
    </Show>
  }
}

#[component]
pub fn ToastTitle(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="toast-title" class=cn(&["text-sm font-semibold", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn ToastDescription(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="toast-description" class=cn(&["text-sm opacity-90", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn ToastAction(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <button
      type="button"
      data-slot="toast-action"
      class=cn(
        &[
          "inline-flex h-8 shrink-0 items-center justify-center rounded-md border bg-transparent px-3 text-sm font-medium ring-offset-background transition-colors hover:bg-secondary focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 group-[.destructive]:border-muted/40 group-[.destructive]:hover:border-destructive/30 group-[.destructive]:hover:bg-destructive group-[.destructive]:hover:text-destructive-foreground group-[.destructive]:focus:ring-destructive",
          class.as_str(),
        ],
      )
    >
      {children()}
    </button>
  }
}

#[component]
pub fn ToastClose(
  #[prop(into, optional)] class: String,
  #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
  let handle_click = move |_| {
    if let Some(cb) = on_click {
      cb.run(());
    }
  };

  view! {
    <button
      type="button"
      data-slot="toast-close"
      class=cn(
        &[
          "absolute right-2 top-2 rounded-md p-1 text-foreground/50 opacity-0 transition-opacity hover:text-foreground focus:opacity-100 focus:outline-none focus:ring-2 group-hover:opacity-100 group-[.destructive]:text-red-300 group-[.destructive]:hover:text-red-50 group-[.destructive]:focus:ring-red-400 group-[.destructive]:focus:ring-offset-red-600",
          class.as_str(),
        ],
      )
      on:click=handle_click
    >
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
        class="h-4 w-4"
      >
        <path d="M18 6 6 18" />
        <path d="m6 6 12 12" />
      </svg>
    </button>
  }
}
