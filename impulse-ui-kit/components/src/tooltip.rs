#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy)]
struct TooltipContext {
  is_open: RwSignal<bool>,
}

#[derive(Clone, Copy)]
struct TooltipTriggerRef {
  trigger_ref: NodeRef<leptos::html::AnyElement>,
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

  let trigger_ref = NodeRef::<leptos::html::AnyElement>::new();

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

  let children = StoredValue::new(children);

  view! {
    <Show when=move || context.is_open.get()>
      <div
        data-slot="tooltip-content"
        data-state=move || if context.is_open.get() { "open" } else { "closed" }
        role="tooltip"
        class=cn(
          &[
            "z-50 overflow-hidden rounded-md border bg-popover px-3 py-1.5 text-sm text-popover-foreground shadow-md animate-in fade-in-0 zoom-in-95",
            class.as_str(),
          ],
        )
      >
        {children.get_value()()}
      </div>
    </Show>
  }
}
