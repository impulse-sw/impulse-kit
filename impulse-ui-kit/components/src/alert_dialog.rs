#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

use super::button::{Button, ButtonVariant};

#[derive(Clone, Copy)]
struct AlertDialogContext {
  is_open: RwSignal<bool>,
}

#[component]
pub fn AlertDialog(
  #[prop(optional)] open: Option<RwSignal<bool>>,
  #[prop(optional)] default_open: Option<bool>,
  children: Children,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(default_open.unwrap_or(false)));

  provide_context(AlertDialogContext { is_open });

  view! { <div data-slot="alert-dialog">{children()}</div> }
}

#[component]
pub fn AlertDialogTrigger(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<AlertDialogContext>().expect("AlertDialogTrigger must be used within AlertDialog");

  view! {
    <div
      attr:data-slot="alert-dialog-trigger"
      on:click=move |_| context.is_open.set(true)
      class=cn(&["inline-block", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn AlertDialogOverlay(#[prop(optional)] class: String) -> impl IntoView {
  let context = use_context::<AlertDialogContext>().expect("AlertDialogOverlay must be used within AlertDialog");

  view! {
    <div
      data-slot="alert-dialog-overlay"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      class=cn(
        &[
          "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/50 data-[state=closed]:pointer-events-none data-[state=closed]:opacity-0",
          class.as_str(),
        ],
      )
    />
  }
}

#[component]
pub fn AlertDialogContent(#[prop(optional)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<AlertDialogContext>().expect("AlertDialogContent must be used within AlertDialog");

  let children = StoredValue::new(children);
  let class = StoredValue::new(class);

  view! {
    <AlertDialogOverlay />
    <div
      data-slot="alert-dialog-content"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      class=cn(
        &[
          "bg-background data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed top-[50%] left-[50%] z-50 grid w-full max-w-[calc(100%-2rem)] translate-x-[-50%] translate-y-[-50%] gap-4 rounded-lg border p-6 shadow-lg sm:max-w-lg data-[state=closed]:pointer-events-none data-[state=closed]:invisible",
          class.read_value().as_str(),
        ],
      )
    >
      {children.read_value()()}
    </div>
  }
}

#[component]
pub fn AlertDialogHeader(#[prop(optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="alert-dialog-header"
      class=cn(&["flex flex-col gap-2 text-center sm:text-left", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn AlertDialogFooter(#[prop(optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="alert-dialog-footer"
      class=cn(&["flex flex-col-reverse gap-2 sm:flex-row sm:justify-end", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn AlertDialogTitle(#[prop(optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <h2 data-slot="alert-dialog-title" class=cn(&["text-lg font-semibold", class.as_str()])>
      {children()}
    </h2>
  }
}

#[component]
pub fn AlertDialogDescription(#[prop(optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <p
      data-slot="alert-dialog-description"
      class=cn(&["text-muted-foreground text-sm", class.as_str()])
    >
      {children()}
    </p>
  }
}

#[component]
pub fn AlertDialogCancel(
  #[prop(optional)] class: String,
  #[prop(optional)] on_click: Option<Callback<()>>,
  children: Children,
) -> impl IntoView {
  let context = use_context::<AlertDialogContext>().expect("AlertDialogCancel must be used within AlertDialog");

  view! {
    <Button
      variant=ButtonVariant::Outline
      class=class
      attr:data-slot="alert-dialog-cancel"
      on:click=move |_| {
        context.is_open.set(false);
        if let Some(cb) = on_click {
          cb.run(());
        }
      }
    >
      {children()}
    </Button>
  }
}

#[component]
pub fn AlertDialogAction(
  #[prop(optional)] class: String,
  #[prop(optional)] on_click: Option<Callback<()>>,
  children: Children,
) -> impl IntoView {
  let context = use_context::<AlertDialogContext>().expect("AlertDialogAction must be used within AlertDialog");

  view! {
    <Button
      class=class
      attr:data-slot="alert-dialog-action"
      on:click=move |_| {
        if let Some(cb) = on_click {
          cb.run(());
        }
        context.is_open.set(false);
      }
    >
      {children()}
    </Button>
  }
}
