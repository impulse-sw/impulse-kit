#![allow(missing_docs, dead_code)]

//! Usage:
//!
//! web-sys = { version = "0.3.82", features = ["DomRect", "Element", "HtmlButtonElement", "HtmlDivElement"] }

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::Element;

use super::button::{Button, ButtonSize, ButtonVariant};

const BASE_CONTENT_CLASSES: &str = "bg-popover text-popover-foreground z-50 w-72 rounded-md border p-4 shadow-md outline-hidden data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95";

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
pub fn PopoverTrigger(
  #[prop(optional)] variant: ButtonVariant,
  #[prop(optional)] size: ButtonSize,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let context = use_context::<PopoverContext>().expect("PopoverTrigger must be used within Popover");

  let trigger_ref = NodeRef::<leptos::html::Button>::new();

  provide_context(PopoverTriggerRef { trigger_ref });

  let handle_click = move |_| {
    context.is_open.update(|open| *open = !*open);
  };

  view! {
    <Button
      node_ref=trigger_ref
      attr:data-slot="popover-trigger"
      variant=variant
      size=size
      class=class
      on:click=handle_click
    >
      {children()}
    </Button>
  }
}

#[component]
pub fn PopoverContent(
  #[prop(optional)] align: Option<PopoverAlign>,
  #[prop(optional)] side: Option<PopoverSide>,
  #[prop(optional)] side_offset: Option<i32>,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<PopoverContext>().expect("PopoverContent must be used within Popover");

  let trigger_context = use_context::<PopoverTriggerRef>();

  let content_ref = NodeRef::<leptos::html::Div>::new();
  let align = align.unwrap_or(PopoverAlign::Center);
  let side = side.unwrap_or(PopoverSide::Bottom);
  let side_offset = side_offset.unwrap_or(4);

  let position_style = RwSignal::new(String::new());
  let rendered = RwSignal::new(false);

  Effect::new(move |_| {
    if context.is_open.get() {
      rendered.set(true);
    } else {
      set_timeout(move || rendered.set(false), std::time::Duration::from_millis(150));
    }
  });

  Effect::new(move |_| {
    if context.is_open.get()
      && let Some(trigger_ref) = trigger_context
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
    PopoverSide::Top => "data-[state=open]:slide-in-from-bottom-2",
    PopoverSide::Right => "data-[state=open]:slide-in-from-left-2",
    PopoverSide::Bottom => "data-[state=open]:slide-in-from-top-2",
    PopoverSide::Left => "data-[state=open]:slide-in-from-right-2",
  };

  let children = StoredValue::new(children);

  view! {
    <Show when=move || rendered.get()>
      <div
        node_ref=content_ref
        data-slot="popover-content"
        data-state=move || if context.is_open.get() { "open" } else { "closed" }
        class=cn(&[BASE_CONTENT_CLASSES, slide_class, class.as_str()])
        style=move || position_style.get()
      >
        {children.read_value()()}
      </div>
    </Show>
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
  trigger_ref: NodeRef<leptos::html::Button>,
}

#[allow(clippy::too_many_arguments)]
fn calculate_position(
  trigger_top: f64,
  trigger_left: f64,
  trigger_width: f64,
  trigger_height: f64,
  content_width: f64,
  content_height: f64,
  side: PopoverSide,
  align: PopoverAlign,
  side_offset: i32,
) -> (f64, f64) {
  let offset = side_offset as f64;

  let (mut top, mut left) = match side {
    PopoverSide::Top => (trigger_top - content_height - offset, trigger_left),
    PopoverSide::Bottom => (trigger_top + trigger_height + offset, trigger_left),
    PopoverSide::Left => (trigger_top, trigger_left - content_width - offset),
    PopoverSide::Right => (trigger_top, trigger_left + trigger_width + offset),
  };

  match side {
    PopoverSide::Top | PopoverSide::Bottom => {
      left += match align {
        PopoverAlign::Start => 0.0,
        PopoverAlign::Center => (trigger_width - content_width) / 2.0,
        PopoverAlign::End => trigger_width - content_width,
      };
    }
    PopoverSide::Left | PopoverSide::Right => {
      top += match align {
        PopoverAlign::Start => 0.0,
        PopoverAlign::Center => (trigger_height - content_height) / 2.0,
        PopoverAlign::End => trigger_height - content_height,
      };
    }
  }

  (top, left)
}
