#![allow(missing_docs, dead_code)]

use crate::viewport::viewport_size;
use impulse_client_kit::utils::cn;
use impulse_client_kit::utils::{OverlayAlign, OverlaySide, calculate_position};
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::Element;

#[component]
pub fn Menubar(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="menubar"
      class=cn(
        &["bg-background flex h-10 items-center gap-1 rounded-md border p-1", class.as_str()],
      )
    >

      {children()}
    </div>
  }
}

#[component]
pub fn MenubarMenu(#[prop(optional)] open: Option<RwSignal<bool>>, children: Children) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(false));

  provide_context(MenubarMenuContext { is_open });

  view! {
    <div data-slot="menubar-menu" class="relative">
      {children()}
    </div>
  }
}

#[component]
pub fn MenubarTrigger(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<MenubarMenuContext>().expect("MenubarTrigger must be used within MenubarMenu");

  let trigger_ref = NodeRef::<leptos::html::Button>::new();

  provide_context(MenubarTriggerRef { trigger_ref });

  let handle_click = move |_| {
    context.is_open.update(|open| *open = !*open);
  };

  view! {
    <button
      node_ref=trigger_ref
      data-slot="menubar-trigger"
      class=cn(
        &[
          "hover:bg-accent hover:text-accent-foreground focus-visible:ring-ring/50 data-[state=open]:bg-accent data-[state=open]:text-accent-foreground flex cursor-default items-center rounded-sm px-3 py-1.5 text-sm font-medium outline-none focus-visible:ring-[3px]",
          class.as_str(),
        ],
      )

      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      on:click=handle_click
    >
      {children()}
    </button>
  }
}

#[component]
pub fn MenubarContent(
  #[prop(optional)] side: Option<OverlaySide>,
  #[prop(optional)] align: Option<OverlayAlign>,
  #[prop(optional)] side_offset: Option<i32>,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<MenubarMenuContext>().expect("MenubarContent must be used within MenubarMenu");

  let trigger_context = use_context::<MenubarTriggerRef>();

  let content_ref = NodeRef::<leptos::html::Div>::new();
  let side = side.unwrap_or(OverlaySide::Bottom);
  let align = align.unwrap_or(OverlayAlign::Start);
  let side_offset = side_offset.unwrap_or(4);

  let position_style = RwSignal::new(String::new());

  Effect::new(move |_| {
    if context.is_open.get()
      && let Some(trigger_ref) = trigger_context
      && let Some(trigger) = trigger_ref.trigger_ref.get()
      && let Some(content) = content_ref.get()
    {
      let trigger_rect = trigger.get_bounding_client_rect();
      let (viewport_width, viewport_height) = viewport_size();

      let (top, left) = calculate_position(
        trigger_rect.top(),
        trigger_rect.left(),
        trigger_rect.width(),
        trigger_rect.height(),
        content.offset_width() as f64,
        content.offset_height() as f64,
        side,
        align,
        side_offset,
        viewport_width,
        viewport_height,
      );

      position_style.set(format!("position: fixed; top: {}px; left: {}px;", top, left));
    } else {
      position_style.set("position: fixed; top: 0px; left: 0px;".to_string());
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
    OverlaySide::Top => "data-[state=open]:slide-in-from-bottom-2",
    OverlaySide::Right => "data-[state=open]:slide-in-from-left-2",
    OverlaySide::Bottom => "data-[state=open]:slide-in-from-top-2",
    OverlaySide::Left => "data-[state=open]:slide-in-from-right-2",
  };

  let children = StoredValue::new(children);

  view! {
    <div
      node_ref=content_ref
      data-slot="menubar-content"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      class=cn(
        &[
          "bg-popover text-popover-foreground z-50 min-w-[12rem] overflow-hidden rounded-md border p-1 shadow-md data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:invisible data-[state=closed]:pointer-events-none data-[state=closed]:h-0",
          slide_class,
          class.as_str(),
        ],
      )

      style=move || position_style.get()
    >
      {children.read_value()()}
    </div>
  }
}

#[component]
pub fn MenubarItem(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="menubar-item"
      class=cn(
        &[
          "focus:bg-accent focus:text-accent-foreground relative flex cursor-default select-none items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none transition-colors data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&>svg]:size-4 [&>svg]:shrink-0",
          class.as_str(),
        ],
      )
    >

      {children()}
    </div>
  }
}

#[component]
pub fn MenubarCheckboxItem(
  #[prop(optional, into)] class: String,
  #[prop(optional)] checked: RwSignal<bool>,
  children: Children,
) -> impl IntoView {
  let handle_click = move |_| {
    checked.update(|c| *c = !*c);
  };

  view! {
    <div
      data-slot="menubar-checkbox-item"
      data-state=move || if checked.get() { "checked" } else { "unchecked" }
      class=cn(
        &[
          "focus:bg-accent focus:text-accent-foreground relative flex cursor-default select-none items-center gap-2 rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none transition-colors data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
          class.as_str(),
        ],
      )

      on:click=handle_click
    >
      <span class="absolute left-2 flex size-3.5 items-center justify-center">
        <Show when=move || checked.get()>
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="size-4"
          >
            <path d="M20 6 9 17l-5-5" />
          </svg>
        </Show>
      </span>
      {children()}
    </div>
  }
}

#[component]
pub fn MenubarSeparator(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <div data-slot="menubar-separator" class=cn(&["bg-muted -mx-1 my-1 h-px", class.as_str()]) />
  }
}

#[component]
pub fn MenubarLabel(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="menubar-label" class=cn(&["px-2 py-1.5 text-sm font-semibold", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn MenubarShortcut(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <span
      data-slot="menubar-shortcut"
      class=cn(&["text-muted-foreground ml-auto text-xs tracking-widest", class.as_str()])
    >
      {children()}
    </span>
  }
}

#[derive(Clone, Copy)]
struct MenubarMenuContext {
  is_open: RwSignal<bool>,
}

#[derive(Clone, Copy)]
struct MenubarTriggerRef {
  trigger_ref: NodeRef<leptos::html::Button>,
}
