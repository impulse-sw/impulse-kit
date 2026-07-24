#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum SheetSide {
  Top,
  #[default]
  Right,
  Bottom,
  Left,
}

impl SheetSide {
  fn as_str(&self) -> &'static str {
    match self {
      SheetSide::Top => "top",
      SheetSide::Right => "right",
      SheetSide::Bottom => "bottom",
      SheetSide::Left => "left",
    }
  }
}

#[derive(Clone, Copy)]
struct SheetContext {
  is_open: RwSignal<bool>,
  on_open_change: Option<Callback<bool>>,
  side: SheetSide,
}

#[component]
pub fn Sheet(
  #[prop(optional)] open: Option<RwSignal<bool>>,
  #[prop(optional)] default_open: Option<bool>,
  #[prop(optional)] on_open_change: Option<Callback<bool>>,
  #[prop(optional)] side: Option<SheetSide>,
  children: Children,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(default_open.unwrap_or(false)));
  let side = side.unwrap_or(SheetSide::Right);

  provide_context(SheetContext {
    is_open,
    on_open_change,
    side,
  });

  view! { <div data-slot="sheet">{children()}</div> }
}

#[component]
pub fn SheetTrigger(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<SheetContext>().expect("SheetTrigger must be used within Sheet");

  let handle_click = move |_| {
    context.is_open.set(true);
    if let Some(callback) = context.on_open_change {
      callback.run(true);
    }
  };

  view! {
    <button
      type="button"
      data-slot="sheet-trigger"
      class=cn(&["inline-block", class.as_str()])
      on:click=handle_click
    >
      {children()}
    </button>
  }
}

#[component]
pub fn SheetClose(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<SheetContext>().expect("SheetClose must be used within Sheet");

  let handle_click = move |_| {
    context.is_open.set(false);
    if let Some(callback) = context.on_open_change {
      callback.run(false);
    }
  };

  view! {
    <button type="button" data-slot="sheet-close" class=class on:click=handle_click>
      {children()}
    </button>
  }
}

#[component]
pub fn SheetOverlay(#[prop(into, optional)] class: String) -> impl IntoView {
  let context = use_context::<SheetContext>().expect("SheetOverlay must be used within Sheet");

  let handle_click = move |_| {
    context.is_open.set(false);
    if let Some(callback) = context.on_open_change {
      callback.run(false);
    }
  };

  view! {
    <div
      data-slot="sheet-overlay"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      class=cn(
        &[
          "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/50 data-[state=closed]:pointer-events-none data-[state=closed]:opacity-0",
          class.as_str(),
        ],
      )
      on:click=handle_click
    />
  }
}

#[component]
pub fn SheetContent(#[prop(optional, into)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<SheetContext>().expect("SheetContent must be used within Sheet");

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

  // The effect above only restores scrolling when `is_open` flips to false. If
  // the overlay unmounts while still open, that branch never runs and the body
  // stays scroll-locked; restore it on disposal too.
  on_cleanup(|| {
    if let Some(body) = document().body() {
      let _ = body.style().remove_property("overflow");
    }
  });

  let side_classes = match context.side {
    SheetSide::Top => {
      "inset-x-0 top-0 border-b data-[state=closed]:slide-out-to-top data-[state=open]:slide-in-from-top"
    }
    SheetSide::Bottom => {
      "inset-x-0 bottom-0 border-t data-[state=closed]:slide-out-to-bottom data-[state=open]:slide-in-from-bottom"
    }
    SheetSide::Left => {
      "inset-y-0 left-0 h-full w-3/4 border-r sm:max-w-sm data-[state=closed]:slide-out-to-left data-[state=open]:slide-in-from-left"
    }
    SheetSide::Right => {
      "inset-y-0 right-0 h-full w-3/4 border-l sm:max-w-sm data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right"
    }
  };

  let children = StoredValue::new(children);
  let class = StoredValue::new(class);

  view! {
    <SheetOverlay />
    <div
      data-slot="sheet-content"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      data-side=context.side.as_str()
      class=cn(
        &[
          "bg-background data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:duration-300 data-[state=open]:duration-300 fixed z-50 gap-4 p-6 shadow-lg data-[state=closed]:pointer-events-none data-[state=closed]:invisible",
          side_classes,
          class.read_value().as_str(),
        ],
      )
    >
      {children.get_value()()}
    </div>
  }
}

#[component]
pub fn SheetHeader(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="sheet-header"
      class=cn(&["flex flex-col gap-2 text-center sm:text-left", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn SheetFooter(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="sheet-footer"
      class=cn(&["flex flex-col-reverse gap-2 sm:flex-row sm:justify-end", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn SheetTitle(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <h2
      data-slot="sheet-title"
      class=cn(&["text-lg font-semibold text-foreground", class.as_str()])
    >
      {children()}
    </h2>
  }
}

#[component]
pub fn SheetDescription(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <p data-slot="sheet-description" class=cn(&["text-sm text-muted-foreground", class.as_str()])>
      {children()}
    </p>
  }
}
