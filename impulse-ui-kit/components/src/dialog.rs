#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy)]
struct DialogContext {
  is_open: RwSignal<bool>,
  on_open_change: Option<Callback<bool>>,
}

#[component]
pub fn Dialog(
  #[prop(optional)] open: Option<RwSignal<bool>>,
  #[prop(optional)] default_open: Option<bool>,
  #[prop(optional)] on_open_change: Option<Callback<bool>>,
  children: Children,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(default_open.unwrap_or(false)));

  provide_context(DialogContext {
    is_open,
    on_open_change,
  });

  view! { <div data-slot="dialog">{children()}</div> }
}

#[component]
pub fn DialogTrigger(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<DialogContext>().expect("DialogTrigger must be used within Dialog");

  let handle_click = move |_| {
    context.is_open.set(true);
    if let Some(callback) = context.on_open_change {
      callback.run(true);
    }
  };

  view! {
    <div
      attr:data-slot="dialog-trigger"
      class=cn(&["inline-block", class.as_str()])
      on:click=handle_click
    >
      {children()}
    </div>
  }
}

#[component]
pub fn DialogClose(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<DialogContext>().expect("DialogClose must be used within Dialog");

  let handle_click = move |_| {
    context.is_open.set(false);
    if let Some(callback) = context.on_open_change {
      callback.run(false);
    }
  };

  view! {
    <div attr:data-slot="dialog-close" class=class on:click=handle_click>
      {children()}
    </div>
  }
}

#[component]
pub fn DialogOverlay(#[prop(optional)] class: String) -> impl IntoView {
  let context = use_context::<DialogContext>().expect("DialogOverlay must be used within Dialog");

  let handle_click = move |_| {
    context.is_open.set(false);
    if let Some(callback) = context.on_open_change {
      callback.run(false);
    }
  };

  view! {
    <div
      data-slot="dialog-overlay"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      class=cn(
        &[
          "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/50 data-[state=closed]:pointer-events-none",
          class.as_str(),
        ],
      )
      on:click=handle_click
    />
  }
}

#[component]
pub fn DialogContent(#[prop(optional)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<DialogContext>().expect("DialogContent must be used within Dialog");

  Effect::new(move |_| {
    if context.is_open.get() {
      window_event_listener(leptos::ev::keydown, move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" {
          context.is_open.set(false);
          if let Some(callback) = context.on_open_change {
            callback.run(false);
          }
        }
      });
    }
  });

  Effect::new(move |_| {
    if context.is_open.get() {
      if let Some(body) = document().body() {
        let _ = body.style().set_property("overflow", "hidden");
      }
    } else if let Some(body) = document().body() {
      let _ = body.style().remove_property("overflow");
    }
  });

  let children = StoredValue::new(children);
  let class = StoredValue::new(class);

  view! {
    <DialogOverlay />
    <div
      data-slot="dialog-content"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      class=cn(
        &[
          "bg-background data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed top-[50%] left-[50%] z-50 grid w-full max-w-[calc(100%-2rem)] translate-x-[-50%] translate-y-[-50%] gap-4 rounded-lg border p-6 shadow-lg duration-200 sm:max-w-lg data-[state=closed]:pointer-events-none",
          class.read_value().as_str(),
        ],
      )
    >
      {children.read_value()()}
    </div>
  }
}

#[component]
pub fn DialogHeader(#[prop(optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="dialog-header"
      class=cn(&["flex flex-col gap-2 text-center sm:text-left", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn DialogFooter(#[prop(optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="dialog-footer"
      class=cn(&["flex flex-col-reverse gap-2 sm:flex-row sm:justify-end", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn DialogTitle(#[prop(optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <h2
      data-slot="dialog-title"
      class=cn(&["text-lg font-semibold leading-none tracking-tight", class.as_str()])
    >
      {children()}
    </h2>
  }
}

#[component]
pub fn DialogDescription(#[prop(optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <p data-slot="dialog-description" class=cn(&["text-muted-foreground text-sm", class.as_str()])>
      {children()}
    </p>
  }
}
