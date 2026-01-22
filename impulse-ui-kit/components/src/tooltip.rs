#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::{Portal, cn};
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
      class=class
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
pub fn TooltipContent(#[prop(optional, into)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<TooltipContext>().expect("TooltipContent must be used within Tooltip");

  let rendered = RwSignal::new(false);

  // Delayed unmounting for animations
  Effect::new(move |_| {
    if context.is_open.get() {
      rendered.set(true);
    } else {
      set_timeout(move || rendered.set(false), std::time::Duration::from_millis(150));
    }
  });

  let children = StoredValue::new(children);
  let class = StoredValue::new(class);

  view! {
    <Show when=move || rendered.get()>
      <Portal>
        <div
          data-slot="tooltip-content"
          data-state=move || if context.is_open.get() { "open" } else { "closed" }
          role="tooltip"
          class=cn(
            &[
              "fixed z-50 overflow-hidden rounded-md border bg-popover px-3 py-1.5 text-sm text-popover-foreground shadow-md data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
              class.read_value().as_str(),
            ],
          )
        >
          {children.get_value()()}
        </div>
      </Portal>
    </Show>
  }
}
