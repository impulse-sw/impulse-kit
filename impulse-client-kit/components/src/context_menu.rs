#![allow(missing_docs, dead_code)]

use crate::viewport::viewport_size;
use impulse_client_kit::utils::clamp_to_viewport;
use impulse_client_kit::utils::cn;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::Element;

#[component]
pub fn ContextMenu(#[prop(optional)] open: Option<RwSignal<bool>>, children: Children) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(false));
  let position = RwSignal::new((0.0_f64, 0.0_f64));

  provide_context(ContextMenuContext { is_open, position });

  view! { <div data-slot="context-menu">{children()}</div> }
}

#[component]
pub fn ContextMenuTrigger(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<ContextMenuContext>().expect("ContextMenuTrigger must be used within ContextMenu");

  let trigger_ref = NodeRef::<leptos::html::Div>::new();

  provide_context(ContextMenuTriggerRef { trigger_ref });

  let handle_contextmenu = move |ev: leptos::ev::MouseEvent| {
    ev.prevent_default();
    context.position.set((ev.client_x() as f64, ev.client_y() as f64));
    context.is_open.set(true);
  };

  view! {
    <div
      node_ref=trigger_ref
      data-slot="context-menu-trigger"
      class=cn(&["inline-block", class.as_str()])
      on:contextmenu=handle_contextmenu
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ContextMenuGroup(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="context-menu-group" class=class>
      {children()}
    </div>
  }
}

#[component]
pub fn ContextMenuSub(children: Children) -> impl IntoView {
  let is_open = RwSignal::new(false);
  let is_hovering = RwSignal::new(false);

  provide_context(ContextMenuSubContext { is_open, is_hovering });

  view! { <div data-slot="context-menu-sub">{children()}</div> }
}

#[component]
pub fn ContextMenuRadioGroup(
  #[prop(into)] value: RwSignal<String>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  provide_context(ContextMenuRadioContext { value });

  view! {
    <div data-slot="context-menu-radio-group" class=class>
      {children()}
    </div>
  }
}

#[component]
pub fn ContextMenuSubTrigger(
  #[prop(optional)] inset: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let sub_context =
    use_context::<ContextMenuSubContext>().expect("ContextMenuSubTrigger must be used within ContextMenuSub");

  let trigger_ref = NodeRef::<leptos::html::Div>::new();

  provide_context(ContextMenuSubTriggerRef { trigger_ref });

  let handle_mouse_enter = move |_| {
    sub_context.is_hovering.set(true);
    sub_context.is_open.set(true);
  };

  let handle_mouse_leave = move |_| {
    sub_context.is_hovering.set(false);
    set_timeout(
      move || {
        if !sub_context.is_hovering.get() {
          sub_context.is_open.set(false);
        }
      },
      std::time::Duration::from_millis(150),
    );
  };

  let data_state = move || {
    if sub_context.is_open.get() { "open" } else { "closed" }
  };

  view! {
    <div
      node_ref=trigger_ref
      data-slot="context-menu-sub-trigger"
      data-inset=inset
      data-state=data_state
      class=cn(
        &[
          "focus:bg-accent focus:text-accent-foreground data-[state=open]:bg-accent data-[state=open]:text-accent-foreground [&_svg:not([class*='text-'])]:text-muted-foreground flex cursor-default items-center rounded-sm px-2 py-1.5 text-sm outline-hidden select-none data-[inset]:pl-8 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
          class.as_str(),
        ],
      )
      on:mouseenter=handle_mouse_enter
      on:mouseleave=handle_mouse_leave
    >
      {children()}
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="24"
        height="24"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="ml-auto size-4"
      >
        <path d="m9 18 6-6-6-6" />
      </svg>
    </div>
  }
}

#[component]
pub fn ContextMenuSubContent(#[prop(optional, into)] class: String, children: ChildrenFn) -> impl IntoView {
  let sub_context =
    use_context::<ContextMenuSubContext>().expect("ContextMenuSubContent must be used within ContextMenuSub");

  let sub_trigger_context = use_context::<ContextMenuSubTriggerRef>();

  let content_ref = NodeRef::<leptos::html::Div>::new();
  let position_style = RwSignal::new(String::new());

  let children_stored = StoredValue::new(children);

  // Position calculation
  Effect::new(move |_| {
    if sub_context.is_open.get() {
      // Use requestAnimationFrame to ensure content is laid out
      request_animation_frame(move || {
        if let Some(trigger_ref) = sub_trigger_context
          && let Some(trigger) = trigger_ref.trigger_ref.get()
          && let Some(content) = content_ref.get()
        {
          let trigger_rect = trigger.get_bounding_client_rect();
          let (viewport_width, viewport_height) = viewport_size();

          let top = trigger_rect.top();
          let left = trigger_rect.right() + 4.0;
          let (top, left) = clamp_to_viewport(
            top,
            left,
            content.offset_width() as f64,
            content.offset_height() as f64,
            viewport_width,
            viewport_height,
          );

          position_style.set(format!("position: fixed; top: {}px; left: {}px;", top, left));
        }
      });
    }
  });

  let handle_mouse_enter = move |_| {
    sub_context.is_hovering.set(true);
  };

  let handle_mouse_leave = move |_| {
    sub_context.is_hovering.set(false);
    if !sub_context.is_hovering.get() {
      sub_context.is_open.set(false);
    }
  };

  let data_state = move || {
    if sub_context.is_open.get() { "open" } else { "closed" }
  };

  let class = StoredValue::new(class);

  view! {
    <div
      node_ref=content_ref
      data-slot="context-menu-sub-content"
      data-state=data_state
      data-side="right"
      class=cn(
        &[
          "bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=bottom]:slide-out-to-top-2 data-[side=left]:slide-in-from-right-2 data-[side=left]:slide-out-to-right-2 data-[side=right]:slide-in-from-left-2 data-[side=right]:slide-out-to-left-2 data-[side=top]:slide-in-from-bottom-2 data-[side=top]:slide-out-to-bottom-2 fixed z-50 min-w-[8rem] overflow-hidden rounded-md border p-1 shadow-lg data-[state=closed]:opacity-0 data-[state=closed]:pointer-events-none data-[state=closed]:invisible",
          class.read_value().as_str(),
        ],
      )
      style=move || position_style.get()
      on:mouseenter=handle_mouse_enter
      on:mouseleave=handle_mouse_leave
    >
      {children_stored.get_value()()}
    </div>
  }
}

#[component]
pub fn ContextMenuContent(#[prop(optional, into)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<ContextMenuContext>().expect("ContextMenuContent must be used within ContextMenu");

  let content_ref = NodeRef::<leptos::html::Div>::new();

  let position_style = RwSignal::new(String::new());

  let children_stored = StoredValue::new(children);

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

  // Position calculation
  Effect::new(move |_| {
    if context.is_open.get() {
      // Use requestAnimationFrame to ensure content is laid out
      request_animation_frame(move || {
        let (x, y) = context.position.get();

        if let Some(content) = content_ref.get() {
          let (viewport_width, viewport_height) = viewport_size();
          let (top, left) = clamp_to_viewport(
            y,
            x,
            content.offset_width() as f64,
            content.offset_height() as f64,
            viewport_width,
            viewport_height,
          );

          position_style.set(format!("position: fixed; top: {}px; left: {}px;", top, left));
        } else {
          position_style.set(format!("position: fixed; top: {}px; left: {}px;", y, x));
        }
      });
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
      if !content_el.contains(Some(&target)) {
        context.is_open.set(false);
      }
    }
  };

  Effect::new(move |_| {
    if context.is_open.get() {
      window_event_listener(leptos::ev::click, handle_click_outside);
    }
  });

  let data_state = move || {
    if context.is_open.get() { "open" } else { "closed" }
  };

  let class = StoredValue::new(class);

  view! {
    <div
      node_ref=content_ref
      data-slot="context-menu-content"
      data-state=data_state
      class=cn(
        &[
          "bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=bottom]:slide-out-to-top-2 data-[side=left]:slide-in-from-right-2 data-[side=left]:slide-out-to-right-2 data-[side=right]:slide-in-from-left-2 data-[side=right]:slide-out-to-left-2 data-[side=top]:slide-in-from-bottom-2 data-[side=top]:slide-out-to-bottom-2 fixed z-50 max-h-96 min-w-[8rem] overflow-x-hidden overflow-y-auto rounded-md border p-1 shadow-md data-[state=closed]:opacity-0 data-[state=closed]:pointer-events-none data-[state=closed]:invisible",
          class.read_value().as_str(),
        ],
      )
      style=move || position_style.get()
    >
      {children_stored.get_value()()}
    </div>
  }
}

#[component]
pub fn ContextMenuItem(
  #[prop(optional)] inset: bool,
  #[prop(optional)] variant: Option<ContextMenuItemVariant>,
  #[prop(optional)] on_select: Option<Callback<()>>,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let context = use_context::<ContextMenuContext>();
  let variant = variant.unwrap_or(ContextMenuItemVariant::Default);

  let handle_click = move |_| {
    if !disabled {
      if let Some(on_select) = on_select {
        on_select.run(());
      }
      if let Some(context) = context {
        context.is_open.set(false);
      }
    }
  };

  let variant_class = match variant {
    ContextMenuItemVariant::Default => "",
    ContextMenuItemVariant::Destructive => {
      "data-[variant=destructive]:text-destructive data-[variant=destructive]:focus:bg-destructive/10 dark:data-[variant=destructive]:focus:bg-destructive/20 data-[variant=destructive]:focus:text-destructive data-[variant=destructive]:*:[svg]:!text-destructive"
    }
  };

  view! {
    <div
      data-slot="context-menu-item"
      data-inset=inset
      data-variant=variant.as_str()
      data-disabled=disabled
      class=cn(
        &[
          "focus:bg-accent focus:text-accent-foreground data-[variant=destructive]:text-destructive data-[variant=destructive]:focus:bg-destructive/10 dark:data-[variant=destructive]:focus:bg-destructive/20 data-[variant=destructive]:focus:text-destructive data-[variant=destructive]:*:[svg]:!text-destructive [&_svg:not([class*='text-'])]:text-muted-foreground relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[inset]:pl-8 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 hover:bg-accent hover:text-accent-foreground",
          variant_class,
          class.as_str(),
        ],
      )
      on:click=handle_click
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ContextMenuCheckboxItem(
  #[prop(into)] checked: Signal<bool>,
  #[prop(optional)] on_checked_change: Option<Callback<bool>>,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let handle_click = move |_| {
    if !disabled && let Some(on_checked_change) = on_checked_change {
      on_checked_change.run(!checked.get());
    }
  };

  view! {
    <div
      data-slot="context-menu-checkbox-item"
      data-disabled=disabled
      class=cn(
        &[
          "focus:bg-accent focus:text-accent-foreground relative flex cursor-default items-center gap-2 rounded-sm py-1.5 pr-2 pl-8 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 hover:bg-accent hover:text-accent-foreground",
          class.as_str(),
        ],
      )
      on:click=handle_click
    >
      <span
        class="pointer-events-none absolute left-2 flex size-3.5 items-center justify-center"
        data-checked=move || checked.get()
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="size-4 data-[checked=false]:opacity-0"
          data-checked=move || checked.get()
        >
          <path d="M20 6 9 17l-5-5" />
        </svg>
      </span>
      {children()}
    </div>
  }
}

#[component]
pub fn ContextMenuRadioItem(
  #[prop(into)] value: String,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let radio_context =
    use_context::<ContextMenuRadioContext>().expect("ContextMenuRadioItem must be used within ContextMenuRadioGroup");

  let _value = value.clone();
  let is_checked = Memo::new(move |_| radio_context.value.get() == _value);

  let handle_click = move |_| {
    if !disabled {
      radio_context.value.set(value.clone());
    }
  };

  view! {
    <div
      data-slot="context-menu-radio-item"
      data-disabled=disabled
      class=cn(
        &[
          "focus:bg-accent focus:text-accent-foreground relative flex cursor-default items-center gap-2 rounded-sm py-1.5 pr-2 pl-8 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 hover:bg-accent hover:text-accent-foreground",
          class.as_str(),
        ],
      )
      on:click=handle_click
    >
      <span
        class="pointer-events-none absolute left-2 flex size-3.5 items-center justify-center"
        data-checked=move || is_checked.get()
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="size-2 fill-current data-[checked=false]:opacity-0"
          data-checked=move || is_checked.get()
        >
          <circle cx="12" cy="12" r="10" />
        </svg>
      </span>
      {children()}
    </div>
  }
}

#[component]
pub fn ContextMenuLabel(
  #[prop(optional)] inset: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      data-slot="context-menu-label"
      data-inset=inset
      class=cn(
        &["text-foreground px-2 py-1.5 text-sm font-medium data-[inset]:pl-8", class.as_str()],
      )
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ContextMenuSeparator(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <div
      data-slot="context-menu-separator"
      class=cn(&["bg-border -mx-1 my-1 h-px", class.as_str()])
    />
  }
}

#[component]
pub fn ContextMenuShortcut(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <span
      data-slot="context-menu-shortcut"
      class=cn(&["text-muted-foreground ml-auto text-xs tracking-widest", class.as_str()])
    >
      {children()}
    </span>
  }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ContextMenuItemVariant {
  Default,
  Destructive,
}

impl ContextMenuItemVariant {
  fn as_str(&self) -> &'static str {
    match self {
      ContextMenuItemVariant::Default => "default",
      ContextMenuItemVariant::Destructive => "destructive",
    }
  }
}

#[derive(Clone, Copy)]
struct ContextMenuContext {
  is_open: RwSignal<bool>,
  position: RwSignal<(f64, f64)>,
}

#[derive(Clone, Copy)]
struct ContextMenuTriggerRef {
  trigger_ref: NodeRef<leptos::html::Div>,
}

#[derive(Clone, Copy)]
struct ContextMenuRadioContext {
  value: RwSignal<String>,
}

#[derive(Clone, Copy)]
struct ContextMenuSubContext {
  is_open: RwSignal<bool>,
  is_hovering: RwSignal<bool>,
}

#[derive(Clone, Copy)]
struct ContextMenuSubTriggerRef {
  trigger_ref: NodeRef<leptos::html::Div>,
}
