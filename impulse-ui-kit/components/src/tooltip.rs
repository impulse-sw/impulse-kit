#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use impulse_ui_kit::utils::{OverlayAlign, OverlaySide, calculate_position};
use leptos::portal::Portal;
use leptos::prelude::*;

#[derive(Clone, Copy)]
struct TooltipContext {
  is_open: RwSignal<bool>,
}

#[derive(Clone, Copy)]
struct TooltipTriggerRef {
  trigger_ref: NodeRef<leptos::html::Span>,
}

#[component]
pub fn TooltipProvider(children: Children) -> impl IntoView {
  view! { <div data-slot="tooltip-provider">{children()}</div> }
}

#[component]
pub fn Tooltip(#[prop(optional)] open: Option<RwSignal<bool>>, children: Children) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(false));

  provide_context(TooltipContext { is_open });

  view! { <div data-slot="tooltip">{children()}</div> }
}

#[component]
pub fn TooltipTrigger(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<TooltipContext>().expect("TooltipTrigger must be used within Tooltip");

  let trigger_ref = NodeRef::<leptos::html::Span>::new();

  provide_context(TooltipTriggerRef { trigger_ref });

  let handle_mouseenter = move |_| {
    context.is_open.set(true);
  };

  let handle_mouseleave = move |_| {
    context.is_open.set(false);
  };

  let handle_focus = move |_| {
    context.is_open.set(true);
  };

  let handle_blur = move |_| {
    context.is_open.set(false);
  };

  view! {
    <span
      node_ref=trigger_ref
      data-slot="tooltip-trigger"
      class=cn(&["inline-block", class.as_str()])
      on:mouseenter=handle_mouseenter
      on:mouseleave=handle_mouseleave
      on:focus=handle_focus
      on:blur=handle_blur
    >
      {children()}
    </span>
  }
}

#[component]
pub fn TooltipContent(
  #[prop(optional)] side: Option<OverlaySide>,
  #[prop(optional)] align: Option<OverlayAlign>,
  #[prop(optional)] side_offset: Option<i32>,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<TooltipContext>().expect("TooltipContent must be used within Tooltip");
  let trigger_context = use_context::<TooltipTriggerRef>();

  let content_ref = NodeRef::<leptos::html::Div>::new();
  let side = side.unwrap_or(OverlaySide::Top);
  let align = align.unwrap_or(OverlayAlign::Center);
  let side_offset = side_offset.unwrap_or(4);

  let rendered = RwSignal::new(false);
  let position_style = RwSignal::new(String::new());

  // Delayed unmounting for animations
  Effect::new(move |_| {
    if context.is_open.get() {
      rendered.set(true);
    } else {
      set_timeout(move || rendered.set(false), std::time::Duration::from_millis(150));
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
    {move || {
      if rendered.get() {
        Some(
          view! {
            <Portal>
              <div
                node_ref=content_ref
                data-slot="tooltip-content"
                data-state=move || if context.is_open.get() { "open" } else { "closed" }
                role="tooltip"
                class=cn(
                  &[
                    "fixed z-50 overflow-hidden rounded-md border bg-popover px-3 py-1.5 text-sm text-popover-foreground shadow-md data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
                    slide_class,
                    class.read_value().as_str(),
                  ],
                )
                style=move || position_style.get()
              >
                {children.get_value()()}
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
