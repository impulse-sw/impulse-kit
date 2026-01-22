#![allow(missing_docs, dead_code)]

//! Usage:
//!
//! web-sys = { version = "0.3.82", features = ["DomRect", "Element", "HtmlDivElement"] }

use impulse_ui_kit::utils::cn;
use impulse_ui_kit::utils::{OverlayAlign, OverlaySide, calculate_position};
use leptos::portal::Portal;
use leptos::prelude::*;

const BASE_CONTENT_CLASSES: &str = "bg-popover text-popover-foreground fixed z-50 w-64 rounded-md border p-4 shadow-md outline-hidden data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95";

#[component]
pub fn HoverCard(
  #[prop(optional)] open: Option<RwSignal<bool>>,
  #[prop(optional)] open_delay: Option<u32>,
  #[prop(optional)] close_delay: Option<u32>,
  children: Children,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(false));
  let open_delay = open_delay.unwrap_or(700);
  let close_delay = close_delay.unwrap_or(300);

  provide_context(HoverCardContext {
    is_open,
    open_delay,
    close_delay,
  });

  view! { <div data-slot="hover-card">{children()}</div> }
}

#[component]
pub fn HoverCardTrigger(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<HoverCardContext>().expect("HoverCardTrigger must be used within HoverCard");

  let trigger_ref = NodeRef::<leptos::html::Div>::new();
  let open_timeout = StoredValue::new(None::<leptos::prelude::TimeoutHandle>);
  let close_timeout = StoredValue::new(None::<leptos::prelude::TimeoutHandle>);

  provide_context(HoverCardTriggerRef { trigger_ref });

  let handle_mouse_enter = move |_| {
    if let Some(handle) = close_timeout.get_value() {
      handle.clear();
      close_timeout.set_value(None);
    }

    let timeout = set_timeout_with_handle(
      move || {
        context.is_open.set(true);
      },
      std::time::Duration::from_millis(context.open_delay as u64),
    )
    .ok();

    open_timeout.set_value(timeout);
  };

  let handle_mouse_leave = move |_| {
    if let Some(handle) = open_timeout.get_value() {
      handle.clear();
      open_timeout.set_value(None);
    }

    let timeout = set_timeout_with_handle(
      move || {
        context.is_open.set(false);
      },
      std::time::Duration::from_millis(context.close_delay as u64),
    )
    .ok();

    close_timeout.set_value(timeout);
  };

  view! {
    <div
      node_ref=trigger_ref
      data-slot="hover-card-trigger"
      class=cn(&["inline-block", class.as_str()])
      on:mouseenter=handle_mouse_enter
      on:mouseleave=handle_mouse_leave
    >
      {children()}
    </div>
  }
}

#[component]
pub fn HoverCardContent(
  #[prop(optional)] align: Option<OverlayAlign>,
  #[prop(optional)] side: Option<OverlaySide>,
  #[prop(optional)] side_offset: Option<i32>,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<HoverCardContext>().expect("HoverCardContent must be used within HoverCard");

  let trigger_context = use_context::<HoverCardTriggerRef>();

  let content_ref = NodeRef::<leptos::html::Div>::new();
  let align = align.unwrap_or(OverlayAlign::Center);
  let side = side.unwrap_or(OverlaySide::Bottom);
  let side_offset = side_offset.unwrap_or(4);

  let position_style = RwSignal::new(String::new());
  let rendered = RwSignal::new(false);
  let close_timeout = StoredValue::new(None::<leptos::prelude::TimeoutHandle>);

  // Delayed unmounting for animations
  Effect::new(move |_| {
    if context.is_open.get() {
      rendered.set(true);
    } else {
      set_timeout(move || rendered.set(false), std::time::Duration::from_millis(200));
    }
  });

  // Position calculation - wait for content to be rendered
  Effect::new(move |_| {
    if context.is_open.get() && rendered.get() {
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
    } else {
      position_style.set("position: fixed; top: 0px; left: 0px;".to_string());
    }
  });

  let handle_mouse_enter = move |_| {
    if let Some(handle) = close_timeout.get_value() {
      handle.clear();
      close_timeout.set_value(None);
    }
  };

  let handle_mouse_leave = move |_| {
    let timeout = set_timeout_with_handle(
      move || {
        context.is_open.set(false);
      },
      std::time::Duration::from_millis(context.close_delay as u64),
    )
    .ok();

    close_timeout.set_value(timeout);
  };

  let slide_class = match side {
    OverlaySide::Top => "data-[state=open]:slide-in-from-bottom-2 data-[state=closed]:slide-out-to-bottom-2",
    OverlaySide::Right => "data-[state=open]:slide-in-from-left-2 data-[state=closed]:slide-out-to-left-2",
    OverlaySide::Bottom => "data-[state=open]:slide-in-from-top-2 data-[state=closed]:slide-out-to-top-2",
    OverlaySide::Left => "data-[state=open]:slide-in-from-right-2 data-[state=closed]:slide-out-to-right-2",
  };

  let children = StoredValue::new(children);
  let class = StoredValue::new(class);

  view! {
    {move || {
      if rendered.get() {
        Some(
          view! {
            <Portal>
              <div
                node_ref=content_ref
                data-slot="hover-card-content"
                data-state=move || if context.is_open.get() { "open" } else { "closed" }
                class=cn(&[BASE_CONTENT_CLASSES, slide_class, class.read_value().as_str()])
                style=move || position_style.get()
                on:mouseenter=handle_mouse_enter
                on:mouseleave=handle_mouse_leave
              >
                {children.read_value()()}
              </div>
            </Portal>
          },
        )
      } else {
        None
      }
    }}
  }
}

#[derive(Clone, Copy)]
struct HoverCardContext {
  is_open: RwSignal<bool>,
  open_delay: u32,
  close_delay: u32,
}

#[derive(Clone, Copy)]
struct HoverCardTriggerRef {
  trigger_ref: NodeRef<leptos::html::Div>,
}
