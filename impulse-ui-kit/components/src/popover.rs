#![allow(missing_docs, dead_code)]

//! Usage:
//!
//! web-sys = { version = "0.3.82", features = ["DomRect", "Element", "HtmlButtonElement", "HtmlDivElement"] }

use impulse_ui_kit::utils::cn;
use impulse_ui_kit::utils::{OverlayAlign, OverlaySide, calculate_position};
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::Element;

const BASE_CONTENT_CLASSES: &str = "bg-popover text-popover-foreground fixed z-50 w-72 overflow-hidden rounded-md border p-4 shadow-md outline-hidden data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:opacity-0 data-[state=closed]:pointer-events-none data-[state=closed]:invisible";

#[derive(Clone, Copy, PartialEq)]
pub enum PopoverAlign {
  Start,
  Center,
  End,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PopoverSide {
  Top,
  Right,
  Bottom,
  Left,
}

#[component]
pub fn Popover(#[prop(optional)] open: Option<RwSignal<bool>>, children: Children) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(false));

  provide_context(PopoverContext { is_open });

  view! { <div data-slot="popover">{children()}</div> }
}

#[component]
pub fn PopoverTrigger(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<PopoverContext>().expect("PopoverTrigger must be used within Popover");

  let trigger_ref = NodeRef::<leptos::html::Div>::new();

  provide_context(PopoverTriggerRef { trigger_ref });

  let handle_click = move |_| {
    context.is_open.update(|open| *open = !*open);
  };

  view! {
    <div
      node_ref=trigger_ref
      attr:data-slot="popover-trigger"
      class=cn(&["inline-block", class.as_str()])
      on:click=handle_click
    >
      {children()}
    </div>
  }
}

#[component]
pub fn PopoverContent(
  #[prop(optional)] align: Option<OverlayAlign>,
  #[prop(optional)] side: Option<OverlaySide>,
  #[prop(optional)] side_offset: Option<i32>,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<PopoverContext>().expect("PopoverContent must be used within Popover");

  let trigger_context = use_context::<PopoverTriggerRef>();

  let content_ref = NodeRef::<leptos::html::Div>::new();
  let align = align.unwrap_or(OverlayAlign::Center);
  let side = side.unwrap_or(OverlaySide::Bottom);
  let side_offset = side_offset.unwrap_or(4);

  let position_style = RwSignal::new(String::new());

  // Position calculation
  Effect::new(move |_| {
    if context.is_open.get() {
      // Use requestAnimationFrame to ensure content is laid out
      request_animation_frame(move || {
        if let Some(trigger_ref) = trigger_context
          && let Some(trigger) = trigger_ref.trigger_ref.get()
          && let Some(content) = content_ref.get()
        {
          let trigger_rect = trigger.get_bounding_client_rect();
          let content_rect = content.get_bounding_client_rect();

          let (top, left) = calculate_position(
            trigger_rect.top(),
            trigger_rect.left(),
            trigger_rect.width(),
            trigger_rect.height(),
            content_rect.width(),
            content_rect.height(),
            side,
            align,
            side_offset,
          );

          position_style.set(format!("position: fixed; top: {}px; left: {}px;", top, left));
        }
      });
    }
  });

  // Click outside detection
  let handle_click_outside = move |ev: leptos::ev::MouseEvent| {
    if !context.is_open.get() {
      return;
    }

    let target = ev.target().and_then(|t| t.dyn_into::<Element>().ok());

    if let Some(target) = target
      && let Some(content) = content_ref.get()
    {
      let content_el: &Element = content.as_ref();

      if !content_el.contains(Some(&target))
        && let Some(trigger_ref) = trigger_context
        && let Some(trigger) = trigger_ref.trigger_ref.get()
      {
        let trigger_el: &Element = trigger.as_ref();
        if !trigger_el.contains(Some(&target)) {
          context.is_open.set(false);
        }
      }
    }
  };

  Effect::new(move |_| {
    if context.is_open.get() {
      window_event_listener(leptos::ev::click, handle_click_outside);
    }
  });

  let slide_class = match side {
    OverlaySide::Top => "data-[state=open]:slide-in-from-bottom-2 data-[state=closed]:slide-out-to-bottom-2",
    OverlaySide::Right => "data-[state=open]:slide-in-from-left-2 data-[state=closed]:slide-out-to-left-2",
    OverlaySide::Bottom => "data-[state=open]:slide-in-from-top-2 data-[state=closed]:slide-out-to-top-2",
    OverlaySide::Left => "data-[state=open]:slide-in-from-right-2 data-[state=closed]:slide-out-to-right-2",
  };

  let children = StoredValue::new(children);
  let class = StoredValue::new(class);

  view! {
    <div
      node_ref=content_ref
      data-slot="popover-content"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      class=cn(&[BASE_CONTENT_CLASSES, slide_class, class.read_value().as_str()])
      style=move || position_style.get()
    >
      {children.read_value()()}
    </div>
  }
}

#[component]
pub fn PopoverAnchor(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="popover-anchor" class=class>
      {children()}
    </div>
  }
}

#[derive(Clone, Copy)]
struct PopoverContext {
  is_open: RwSignal<bool>,
}

#[derive(Clone, Copy)]
struct PopoverTriggerRef {
  trigger_ref: NodeRef<leptos::html::Div>,
}
