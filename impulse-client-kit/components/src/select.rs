#![allow(missing_docs, dead_code)]

use crate::viewport::viewport_size;
use impulse_client_kit::utils::cn;
use impulse_client_kit::utils::{OverlayAlign, OverlaySide, calculate_position};
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum SelectTriggerSize {
  Sm,
  #[default]
  Default,
}

impl SelectTriggerSize {
  fn as_str(&self) -> &'static str {
    match self {
      SelectTriggerSize::Sm => "sm",
      SelectTriggerSize::Default => "default",
    }
  }
}

#[component]
pub fn Select(
  #[prop(optional)] value: Option<RwSignal<String>>,
  #[prop(optional)] default_value: Option<String>,
  #[prop(optional)] open: Option<RwSignal<bool>>,
  #[prop(optional)] on_value_change: Option<Callback<String>>,
  children: Children,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(false));
  let selected_value = value.unwrap_or_else(|| RwSignal::new(default_value.unwrap_or_default()));
  let selected_label = RwSignal::new(String::new());

  provide_context(SelectContext {
    is_open,
    selected_value,
    selected_label,
    on_value_change,
  });

  view! { <div data-slot="select">{children()}</div> }
}

#[component]
pub fn SelectGroup(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="select-group" class=class>
      {children()}
    </div>
  }
}

#[component]
pub fn SelectValue(#[prop(optional, into)] placeholder: String) -> impl IntoView {
  let context = use_context::<SelectContext>().expect("SelectValue must be used within Select");

  view! {
    <span data-slot="select-value">
      {move || {
        let label = context.selected_label.get();
        if label.is_empty() { placeholder.clone() } else { label }
      }}
    </span>
  }
}

#[component]
pub fn SelectTrigger(
  #[prop(optional)] size: SelectTriggerSize,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let context = use_context::<SelectContext>().expect("SelectTrigger must be used within Select");

  let trigger_ref = NodeRef::<leptos::html::Button>::new();

  provide_context(SelectTriggerRef { trigger_ref });

  let handle_click = move |_| {
    if !disabled {
      context.is_open.update(|open| *open = !*open);
    }
  };

  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    if disabled {
      return;
    }
    match ev.key().as_str() {
      " " | "Enter" | "ArrowDown" | "ArrowUp" => {
        ev.prevent_default();
        if !context.is_open.get() {
          context.is_open.set(true);
        }
      }
      "Escape" => {
        ev.prevent_default();
        context.is_open.set(false);
      }
      _ => {}
    }
  };

  let data_placeholder = move || {
    if context.selected_value.get().is_empty() {
      Some("true")
    } else {
      None
    }
  };

  view! {
    <button
      node_ref=trigger_ref
      type="button"
      role="combobox"
      aria-expanded=move || context.is_open.get().to_string()
      aria-haspopup="listbox"
      data-slot="select-trigger"
      data-size=size.as_str()
      data-placeholder=data_placeholder
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      disabled=disabled
      class=cn(
        &[
          "border-input data-[placeholder]:text-muted-foreground [&_svg:not([class*='text-'])]:text-muted-foreground focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:bg-input/30 dark:hover:bg-input/50 flex w-fit items-center justify-between gap-2 rounded-md border bg-transparent px-3 py-2 text-sm whitespace-nowrap shadow-xs transition-[color,box-shadow] outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50 data-[size=default]:h-9 data-[size=sm]:h-8 *:data-[slot=select-value]:line-clamp-1 *:data-[slot=select-value]:flex *:data-[slot=select-value]:items-center *:data-[slot=select-value]:gap-2 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
          class.as_str(),
        ],
      )
      on:click=handle_click
      on:keydown=handle_keydown
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
        class="size-4 opacity-50"
        aria-hidden="true"
      >
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>
  }
}

#[component]
pub fn SelectContent(
  #[prop(optional)] side: Option<OverlaySide>,
  #[prop(optional)] align: Option<OverlayAlign>,
  #[prop(optional)] side_offset: Option<i32>,
  #[prop(optional)] position: Option<SelectContentPosition>,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<SelectContext>().expect("SelectContent must be used within Select");

  let trigger_context = use_context::<SelectTriggerRef>();

  let content_ref = NodeRef::<leptos::html::Div>::new();
  let viewport_ref = NodeRef::<leptos::html::Div>::new();
  let side = side.unwrap_or(OverlaySide::Bottom);
  let align = align.unwrap_or(OverlayAlign::Start);
  let side_offset = side_offset.unwrap_or(4);
  let position = position.unwrap_or(SelectContentPosition::ItemAligned);

  let position_style = RwSignal::new(String::new());

  let can_scroll_up = RwSignal::new(false);
  let can_scroll_down = RwSignal::new(false);

  provide_context(SelectScrollContext {
    viewport_ref,
    can_scroll_up,
    can_scroll_down,
  });

  let children_stored = StoredValue::new(children);

  let update_scroll_state = move || {
    if let Some(viewport) = viewport_ref.get() {
      let el: &HtmlElement = viewport.as_ref();
      let scroll_top = el.scroll_top() as f64;
      let scroll_height = el.scroll_height() as f64;
      let client_height = el.client_height() as f64;

      can_scroll_up.set(scroll_top > 0.0);
      can_scroll_down.set(scroll_top < scroll_height - client_height - 1.0);
    }
  };

  Effect::new(move |_| {
    if context.is_open.get() {
      if let Some(body) = document().body() {
        let _ = body.style().set_property("overflow", "hidden");
      }
    } else if let Some(body) = document().body() {
      let _ = body.style().remove_property("overflow");
    }
  });

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
          let (viewport_width, viewport_height) = viewport_size();

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
            viewport_width,
            viewport_height,
          );

          let width_style = if position == SelectContentPosition::Popper {
            format!("min-width: {}px;", trigger_rect.width())
          } else {
            String::new()
          };

          position_style.set(format!(
            "position: fixed; top: {}px; left: {}px; {}",
            top, left, width_style
          ));

          update_scroll_state();
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

  let popper_class = if position == SelectContentPosition::Popper {
    "data-[side=bottom]:translate-y-1 data-[side=left]:-translate-x-1 data-[side=right]:translate-x-1 data-[side=top]:-translate-y-1"
  } else {
    ""
  };

  let viewport_popper_class = if position == SelectContentPosition::Popper {
    "h-[var(--radix-select-trigger-height)] w-full min-w-[var(--radix-select-trigger-width)] scroll-my-1"
  } else {
    ""
  };

  let class = StoredValue::new(class);

  view! {
    <div
      node_ref=content_ref
      role="listbox"
      data-slot="select-content"
      data-state=data_state
      data-side=side_as_str(side)
      class=cn(
        &[
          "bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed z-50 max-h-96 min-w-[8rem] overflow-hidden rounded-md border shadow-md data-[state=closed]:opacity-0 data-[state=closed]:pointer-events-none data-[state=closed]:h-0 data-[state=closed]:w-0 data-[state=closed]:overflow-hidden",
          slide_class,
          popper_class,
          class.read_value().as_str(),
        ],
      )
      style=move || position_style.get()
    >
      <SelectScrollUpButton />
      <div
        node_ref=viewport_ref
        data-slot="select-viewport"
        class=cn(&["p-1 overflow-y-auto max-h-[calc(24rem-2rem)]", viewport_popper_class])
        on:scroll=move |_| update_scroll_state()
      >
        {children_stored.get_value()()}
      </div>
      <SelectScrollDownButton />
    </div>
  }
}

#[component]
pub fn SelectLabel(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="select-label"
      class=cn(&["text-muted-foreground px-2 py-1.5 text-xs", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn SelectItem(
  #[prop(into)] value: String,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<SelectContext>().expect("SelectItem must be used within Select");

  let item_value = value.clone();
  let is_selected = Memo::new(move |_| context.selected_value.get() == item_value);

  let handle_select = {
    let value = value.clone();
    move |_| {
      if !disabled {
        context.selected_value.set(value.clone());
        context.is_open.set(false);
        if let Some(callback) = context.on_value_change {
          callback.run(value.clone());
        }
      }
    }
  };

  let handle_keydown = {
    let value = value.clone();
    move |ev: web_sys::KeyboardEvent| {
      if disabled {
        return;
      }
      if ev.key() == "Enter" || ev.key() == " " {
        ev.prevent_default();
        context.selected_value.set(value.clone());
        context.is_open.set(false);
        if let Some(callback) = context.on_value_change {
          callback.run(value.clone());
        }
      }
    }
  };

  let children_stored = StoredValue::new(children);

  Effect::new({
    let value = value.clone();
    move |_| {
      if context.selected_value.get() == value
        && let Some(label_el) = document()
          .query_selector(&format!(
            "[data-slot='select-item'][data-value='{}'] [data-slot='select-item-text']",
            value
          ))
          .ok()
          .flatten()
      {
        context.selected_label.set(label_el.text_content().unwrap_or_default());
      }
    }
  });

  view! {
    <div
      role="option"
      aria-selected=move || is_selected.get().to_string()
      data-slot="select-item"
      data-value=value
      data-disabled=disabled
      data-selected=move || if is_selected.get() { Some("true") } else { None }
      class=cn(
        &[
          "focus:bg-accent focus:text-accent-foreground [&_svg:not([class*='text-'])]:text-muted-foreground relative flex w-full cursor-default items-center gap-2 rounded-sm py-1.5 pr-8 pl-2 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 *:[span]:last:flex *:[span]:last:items-center *:[span]:last:gap-2 hover:bg-accent hover:text-accent-foreground",
          class.as_str(),
        ],
      )
      tabindex=if disabled { "-1" } else { "0" }
      on:click=handle_select
      on:keydown=handle_keydown
    >
      <span
        data-slot="select-item-indicator"
        class="absolute right-2 flex size-3.5 items-center justify-center"
        data-selected=move || if is_selected.get() { "true" } else { "false" }
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
          class="size-4 data-[selected=false]:opacity-0"
          data-selected=move || if is_selected.get() { "true" } else { "false" }
        >
          <path d="M20 6 9 17l-5-5" />
        </svg>
      </span>
      <span data-slot="select-item-text">{children_stored.get_value()()}</span>
    </div>
  }
}

#[component]
pub fn SelectSeparator(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <div
      data-slot="select-separator"
      class=cn(&["bg-border pointer-events-none -mx-1 my-1 h-px", class.as_str()])
    />
  }
}

#[component]
pub fn SelectScrollUpButton(#[prop(optional, into)] class: String) -> impl IntoView {
  let scroll_context = use_context::<SelectScrollContext>();

  let can_scroll = move || scroll_context.map(|ctx| ctx.can_scroll_up.get()).unwrap_or(false);

  let handle_click = move |_| {
    if let Some(ctx) = scroll_context
      && let Some(viewport) = ctx.viewport_ref.get()
    {
      let el: &HtmlElement = viewport.as_ref();
      // Relative scroll keeps the call signature stable: `set_scroll_top` is `i32`
      // normally but `f64` under `--cfg=web_sys_unstable_apis`, while
      // `scroll_by_with_x_and_y` is always `f64`.
      el.scroll_by_with_x_and_y(0.0, -50.0);
    }
  };

  let class = StoredValue::new(class);

  view! {
    <Show when=can_scroll>
      <div
        data-slot="select-scroll-up-button"
        class=cn(
          &["flex cursor-default items-center justify-center py-1", class.read_value().as_str()],
        )
        on:click=handle_click
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
          class="size-4"
        >
          <path d="m18 15-6-6-6 6" />
        </svg>
      </div>
    </Show>
  }
}

#[component]
pub fn SelectScrollDownButton(#[prop(optional, into)] class: String) -> impl IntoView {
  let scroll_context = use_context::<SelectScrollContext>();

  let can_scroll = move || scroll_context.map(|ctx| ctx.can_scroll_down.get()).unwrap_or(false);

  let handle_click = move |_| {
    if let Some(ctx) = scroll_context
      && let Some(viewport) = ctx.viewport_ref.get()
    {
      let el: &HtmlElement = viewport.as_ref();
      // See `SelectScrollUpButton` for why a relative scroll is used here.
      el.scroll_by_with_x_and_y(0.0, 50.0);
    }
  };

  let class = StoredValue::new(class);

  view! {
    <Show when=can_scroll>
      <div
        data-slot="select-scroll-down-button"
        class=cn(
          &["flex cursor-default items-center justify-center py-1", class.read_value().as_str()],
        )
        on:click=handle_click
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
          class="size-4"
        >
          <path d="m6 9 6 6 6-6" />
        </svg>
      </div>
    </Show>
  }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum SelectContentPosition {
  #[default]
  ItemAligned,
  Popper,
}

fn side_as_str(side: OverlaySide) -> &'static str {
  match side {
    OverlaySide::Top => "top",
    OverlaySide::Right => "right",
    OverlaySide::Bottom => "bottom",
    OverlaySide::Left => "left",
  }
}

#[derive(Clone, Copy)]
struct SelectContext {
  is_open: RwSignal<bool>,
  selected_value: RwSignal<String>,
  selected_label: RwSignal<String>,
  on_value_change: Option<Callback<String>>,
}

#[derive(Clone, Copy)]
struct SelectTriggerRef {
  trigger_ref: NodeRef<leptos::html::Button>,
}

#[derive(Clone, Copy)]
struct SelectScrollContext {
  viewport_ref: NodeRef<leptos::html::Div>,
  can_scroll_up: RwSignal<bool>,
  can_scroll_down: RwSignal<bool>,
}
