#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use impulse_ui_kit::utils::{OverlayAlign, OverlaySide, Portal, calculate_position};
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::Element;

#[component]
pub fn DropdownMenu(#[prop(optional)] open: Option<RwSignal<bool>>, children: Children) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(false));

  provide_context(DropdownContext { is_open });

  view! { <div data-slot="dropdown-menu">{children()}</div> }
}

#[component]
pub fn DropdownMenuTrigger(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<DropdownContext>().expect("DropdownMenuTrigger must be used within DropdownMenu");

  let trigger_ref = NodeRef::<leptos::html::Div>::new();

  provide_context(DropdownTriggerRef { trigger_ref });

  let handle_click = move |_| {
    context.is_open.update(|open| *open = !*open);
  };

  view! {
    <div node_ref=trigger_ref data-slot="dropdown-menu-trigger" class=class on:click=handle_click>
      {children()}
    </div>
  }
}

#[component]
pub fn DropdownMenuContent(
  #[prop(optional)] side: Option<OverlaySide>,
  #[prop(optional)] align: Option<OverlayAlign>,
  #[prop(optional)] side_offset: Option<i32>,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<DropdownContext>().expect("DropdownMenuContent must be used within DropdownMenu");

  let trigger_context = use_context::<DropdownTriggerRef>();

  let content_ref = NodeRef::<leptos::html::Div>::new();
  let side = side.unwrap_or(OverlaySide::Bottom);
  let align = align.unwrap_or(OverlayAlign::Start);
  let side_offset = side_offset.unwrap_or(4);

  let position_style = RwSignal::new(String::new());
  let rendered = RwSignal::new(false);

  let children_stored = StoredValue::new(children);

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

  let data_state = move || {
    if context.is_open.get() { "open" } else { "closed" }
  };

  let slide_class = match side {
    OverlaySide::Top => "data-[state=open]:slide-in-from-bottom-2 data-[state=closed]:slide-out-to-bottom-2",
    OverlaySide::Right => "data-[state=open]:slide-in-from-left-2 data-[state=closed]:slide-out-to-left-2",
    OverlaySide::Bottom => "data-[state=open]:slide-in-from-top-2 data-[state=closed]:slide-out-to-top-2",
    OverlaySide::Left => "data-[state=open]:slide-in-from-right-2 data-[state=closed]:slide-out-to-right-2",
  };

  let class = StoredValue::new(class);

  view! {
    <Show when=move || rendered.get()>
      <Portal>
        <div
          node_ref=content_ref
          data-slot="dropdown-menu-content"
          data-state=data_state
          class=cn(
            &[
              "bg-popover text-popover-foreground fixed z-50 min-w-[8rem] overflow-hidden rounded-md border p-1 shadow-md data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 max-h-[300px] overflow-y-auto",
              slide_class,
              class.read_value().as_str(),
            ],
          )
          style=move || position_style.get()
        >
          {children_stored.get_value()()}
        </div>
      </Portal>
    </Show>
  }
}

#[component]
pub fn DropdownMenuGroup(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="dropdown-menu-group" class=class>
      {children()}
    </div>
  }
}

#[component]
pub fn DropdownMenuItem(
  #[prop(optional)] inset: bool,
  #[prop(optional)] variant: Option<DropdownItemVariant>,
  #[prop(optional)] on_select: Option<Callback<()>>,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let context = use_context::<DropdownContext>();
  let variant = variant.unwrap_or(DropdownItemVariant::Default);

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
    DropdownItemVariant::Default => "",
    DropdownItemVariant::Destructive => {
      "data-[variant=destructive]:text-destructive data-[variant=destructive]:focus:bg-destructive/10 dark:data-[variant=destructive]:focus:bg-destructive/20 data-[variant=destructive]:focus:text-destructive data-[variant=destructive]:*:[svg]:!text-destructive"
    }
  };

  view! {
    <div
      data-slot="dropdown-menu-item"
      data-inset=inset
      data-variant=variant.as_str()
      data-disabled=disabled
      class=cn(
        &[
          "hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground [&_svg:not([class*='text-'])]:text-muted-foreground relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[inset]:pl-8 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
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
pub fn DropdownMenuCheckboxItem(
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
      data-slot="dropdown-menu-checkbox-item"
      data-disabled=disabled
      class=cn(
        &[
          "hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground relative flex cursor-default items-center gap-2 rounded-sm py-1.5 pr-2 pl-8 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
          class.as_str(),
        ],
      )
      on:click=handle_click
    >
      <span class="pointer-events-none absolute left-2 flex size-3.5 items-center justify-center">
        <Show when=move || checked.get()>
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
pub fn DropdownMenuRadioGroup(#[prop(into)] value: RwSignal<String>, children: Children) -> impl IntoView {
  provide_context(DropdownRadioContext { value });

  view! { <div data-slot="dropdown-menu-radio-group">{children()}</div> }
}

#[component]
pub fn DropdownMenuRadioItem(
  #[prop(into)] value: String,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let radio_context =
    use_context::<DropdownRadioContext>().expect("DropdownMenuRadioItem must be used within DropdownMenuRadioGroup");

  let _value = value.clone();
  let is_checked = Memo::new(move |_| radio_context.value.get() == _value);

  let handle_click = move |_| {
    if !disabled {
      radio_context.value.set(value.clone());
    }
  };

  view! {
    <div
      data-slot="dropdown-menu-radio-item"
      data-disabled=disabled
      class=cn(
        &[
          "hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground relative flex cursor-default items-center gap-2 rounded-sm py-1.5 pr-2 pl-8 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
          class.as_str(),
        ],
      )
      on:click=handle_click
    >
      <span class="pointer-events-none absolute left-2 flex size-3.5 items-center justify-center">
        <Show when=move || is_checked.get()>
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
            class="size-2 fill-current"
          >
            <circle cx="12" cy="12" r="10" />
          </svg>
        </Show>
      </span>
      {children()}
    </div>
  }
}

#[component]
pub fn DropdownMenuLabel(
  #[prop(optional)] inset: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      data-slot="dropdown-menu-label"
      data-inset=inset
      class=cn(&["px-2 py-1.5 text-sm font-medium data-[inset]:pl-8", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn DropdownMenuSeparator(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <div
      data-slot="dropdown-menu-separator"
      class=cn(&["bg-border -mx-1 my-1 h-px", class.as_str()])
    />
  }
}

#[component]
pub fn DropdownMenuShortcut(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <span
      data-slot="dropdown-menu-shortcut"
      class=cn(&["text-muted-foreground ml-auto text-xs tracking-widest", class.as_str()])
    >
      {children()}
    </span>
  }
}

#[component]
pub fn DropdownMenuSub(children: Children) -> impl IntoView {
  let is_open = RwSignal::new(false);
  let is_hovering = RwSignal::new(false);

  provide_context(DropdownSubContext { is_open, is_hovering });

  view! { <div data-slot="dropdown-menu-sub">{children()}</div> }
}

#[component]
pub fn DropdownMenuSubTrigger(
  #[prop(optional)] inset: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let sub_context =
    use_context::<DropdownSubContext>().expect("DropdownMenuSubTrigger must be used within DropdownMenuSub");

  let trigger_ref = NodeRef::<leptos::html::Div>::new();

  provide_context(DropdownSubTriggerRef { trigger_ref });

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
      data-slot="dropdown-menu-sub-trigger"
      data-inset=inset
      data-state=data_state
      class=cn(
        &[
          "hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground data-[state=open]:bg-accent data-[state=open]:text-accent-foreground [&_svg:not([class*='text-'])]:text-muted-foreground flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none data-[inset]:pl-8 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
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
pub fn DropdownMenuSubContent(#[prop(optional, into)] class: String, children: ChildrenFn) -> impl IntoView {
  let sub_context =
    use_context::<DropdownSubContext>().expect("DropdownMenuSubContent must be used within DropdownMenuSub");

  let sub_trigger_context = use_context::<DropdownSubTriggerRef>();

  let content_ref = NodeRef::<leptos::html::Div>::new();
  let position_style = RwSignal::new(String::new());

  let children_stored = StoredValue::new(children);

  Effect::new(move |_| {
    if sub_context.is_open.get()
      && let Some(trigger_ref) = sub_trigger_context
      && let Some(trigger) = trigger_ref.trigger_ref.get()
    {
      let trigger_rect = trigger.get_bounding_client_rect();
      let top = trigger_rect.top();
      let left = trigger_rect.right() + 4.0;

      position_style.set(format!("position: fixed; top: {}px; left: {}px;", top, left));
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

  view! {
    <div
      node_ref=content_ref
      data-slot="dropdown-menu-sub-content"
      data-state=data_state
      class=cn(
        &[
          "bg-popover text-popover-foreground fixed z-50 min-w-[8rem] overflow-hidden rounded-md border p-1 shadow-lg data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=right]:slide-in-from-left-2 data-[state=closed]:pointer-events-none data-[state=closed]:h-0 data-[state=closed]:opacity-0",
          class.as_str(),
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

#[derive(Clone, Copy, PartialEq)]
pub enum DropdownItemVariant {
  Default,
  Destructive,
}

impl DropdownItemVariant {
  fn as_str(&self) -> &'static str {
    match self {
      DropdownItemVariant::Default => "default",
      DropdownItemVariant::Destructive => "destructive",
    }
  }
}

#[derive(Clone, Copy)]
struct DropdownContext {
  is_open: RwSignal<bool>,
}

#[derive(Clone, Copy)]
struct DropdownTriggerRef {
  trigger_ref: NodeRef<leptos::html::Div>,
}

#[derive(Clone, Copy)]
struct DropdownRadioContext {
  value: RwSignal<String>,
}

#[derive(Clone, Copy)]
struct DropdownSubContext {
  is_open: RwSignal<bool>,
  is_hovering: RwSignal<bool>,
}

#[derive(Clone, Copy)]
struct DropdownSubTriggerRef {
  trigger_ref: NodeRef<leptos::html::Div>,
}
