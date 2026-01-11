#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::{HtmlElement, PointerEvent};

const CLOSE_THRESHOLD: f64 = 0.25;
const VELOCITY_THRESHOLD: f64 = 0.4;
const TRANSITION_DURATION: f64 = 0.5;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum DrawerDirection {
  Top,
  #[default]
  Bottom,
  Left,
  Right,
}

impl DrawerDirection {
  fn as_str(&self) -> &'static str {
    match self {
      DrawerDirection::Top => "top",
      DrawerDirection::Bottom => "bottom",
      DrawerDirection::Left => "left",
      DrawerDirection::Right => "right",
    }
  }

  fn is_vertical(&self) -> bool {
    matches!(self, DrawerDirection::Top | DrawerDirection::Bottom)
  }
}

#[component]
pub fn Drawer(
  #[prop(optional)] open: Option<RwSignal<bool>>,
  #[prop(optional)] default_open: Option<bool>,
  #[prop(optional)] on_open_change: Option<Callback<bool>>,
  #[prop(optional)] direction: Option<DrawerDirection>,
  #[prop(optional)] modal: Option<bool>,
  #[prop(optional)] dismissible: Option<bool>,
  #[prop(optional)] should_scale_background: Option<bool>,
  children: Children,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(default_open.unwrap_or(false)));
  let direction = direction.unwrap_or(DrawerDirection::Bottom);
  let modal = modal.unwrap_or(true);
  let dismissible = dismissible.unwrap_or(true);
  let should_scale_background = should_scale_background.unwrap_or(false);

  let is_dragging = RwSignal::new(false);
  let drag_start_pos = RwSignal::new(0.0_f64);
  let drag_offset = RwSignal::new(0.0_f64);
  let drawer_size = RwSignal::new(0.0_f64);

  provide_context(DrawerContext {
    is_open,
    on_open_change,
    direction,
    modal,
    dismissible,
    should_scale_background,
    is_dragging,
    drag_start_pos,
    drag_offset,
    drawer_size,
  });

  view! { <div data-slot="drawer">{children()}</div> }
}

#[component]
pub fn DrawerTrigger(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<DrawerContext>().expect("DrawerTrigger must be used within Drawer");

  let handle_click = move |_| {
    context.is_open.set(true);
    if let Some(callback) = context.on_open_change {
      callback.run(true);
    }
  };

  view! {
    <button type="button" data-slot="drawer-trigger" class=class on:click=handle_click>
      {children()}
    </button>
  }
}

#[component]
pub fn DrawerClose(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<DrawerContext>().expect("DrawerClose must be used within Drawer");

  let handle_click = move |_| {
    context.is_open.set(false);
    if let Some(callback) = context.on_open_change {
      callback.run(false);
    }
  };

  view! {
    <button type="button" data-slot="drawer-close" class=class on:click=handle_click>
      {children()}
    </button>
  }
}

#[component]
pub fn DrawerOverlay(#[prop(optional, into)] class: String) -> impl IntoView {
  let context = use_context::<DrawerContext>().expect("DrawerOverlay must be used within Drawer");

  let handle_click = move |_| {
    if context.dismissible {
      context.is_open.set(false);
      if let Some(callback) = context.on_open_change {
        callback.run(false);
      }
    }
  };

  let data_state = move || {
    if context.is_open.get() { "open" } else { "closed" }
  };

  let opacity_style = move || {
    if context.is_dragging.get() {
      let offset = context.drag_offset.get().abs();
      let size = context.drawer_size.get();
      if size > 0.0 {
        let opacity = 1.0 - (offset / size).min(1.0);
        return format!("opacity: {};", opacity);
      }
    }
    String::new()
  };

  view! {
    <div
      data-slot="drawer-overlay"
      data-state=data_state
      class=cn(
        &[
          "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/50",
          class.as_str(),
        ],
      )
      style=opacity_style
      on:click=handle_click
    />
  }
}

#[component]
pub fn DrawerContent(#[prop(optional, into)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<DrawerContext>().expect("DrawerContent must be used within Drawer");

  let content_ref = NodeRef::<leptos::html::Div>::new();
  let should_render = RwSignal::new(false);
  let transform_style = RwSignal::new(String::new());

  let children_stored = StoredValue::new(children);

  Effect::new(move |_| {
    if context.is_open.get() {
      should_render.set(true);
      context.drag_offset.set(0.0);
      transform_style.set(String::new());
    } else if should_render.get() {
      set_timeout(move || should_render.set(false), std::time::Duration::from_millis(500));
    }
  });

  Effect::new(move |_| {
    if context.is_open.get() {
      if let Some(body) = document().body() {
        let _ = body.style().set_property("overflow", "hidden");
      }
    } else if let Some(body) = document().body() {
      let _ = body.style().remove_property("overflow");
    }
  });

  Effect::new(move |_| {
    if context.is_open.get()
      && should_render.get()
      && let Some(content) = content_ref.get()
    {
      let el: &HtmlElement = content.as_ref();
      let size = if context.direction.is_vertical() {
        el.offset_height() as f64
      } else {
        el.offset_width() as f64
      };
      context.drawer_size.set(size);
    }
  });

  let handle_pointer_down = move |ev: PointerEvent| {
    if !context.dismissible {
      return;
    }

    let pos = if context.direction.is_vertical() {
      ev.client_y() as f64
    } else {
      ev.client_x() as f64
    };

    context.is_dragging.set(true);
    context.drag_start_pos.set(pos);
    context.drag_offset.set(0.0);

    if let Some(target) = ev.target()
      && let Ok(el) = target.dyn_into::<HtmlElement>()
    {
      let _ = el.set_pointer_capture(ev.pointer_id());
    }
  };

  let handle_pointer_move = move |ev: PointerEvent| {
    if !context.is_dragging.get() {
      return;
    }

    let current_pos = if context.direction.is_vertical() {
      ev.client_y() as f64
    } else {
      ev.client_x() as f64
    };

    let diff = current_pos - context.drag_start_pos.get();

    let valid_drag = match context.direction {
      DrawerDirection::Bottom => diff > 0.0,
      DrawerDirection::Top => diff < 0.0,
      DrawerDirection::Right => diff > 0.0,
      DrawerDirection::Left => diff < 0.0,
    };

    if valid_drag {
      context.drag_offset.set(diff);

      let transform = match context.direction {
        DrawerDirection::Bottom => format!("translate3d(0, {}px, 0)", diff),
        DrawerDirection::Top => format!("translate3d(0, {}px, 0)", diff),
        DrawerDirection::Right => format!("translate3d({}px, 0, 0)", diff),
        DrawerDirection::Left => format!("translate3d({}px, 0, 0)", diff),
      };

      transform_style.set(format!("transform: {}; transition: none;", transform));
    }
  };

  let handle_pointer_up = move |ev: PointerEvent| {
    if !context.is_dragging.get() {
      return;
    }

    context.is_dragging.set(false);

    let offset = context.drag_offset.get().abs();
    let size = context.drawer_size.get();

    let start_pos = context.drag_start_pos.get();
    let end_pos = if context.direction.is_vertical() {
      ev.client_y() as f64
    } else {
      ev.client_x() as f64
    };

    let time_diff = 0.3;
    let velocity = (end_pos - start_pos).abs() / time_diff / 1000.0;

    let should_close = (size > 0.0 && offset / size >= CLOSE_THRESHOLD) || velocity > VELOCITY_THRESHOLD;

    if should_close {
      let close_transform = match context.direction {
        DrawerDirection::Bottom => "translate3d(0, 100%, 0)",
        DrawerDirection::Top => "translate3d(0, -100%, 0)",
        DrawerDirection::Right => "translate3d(100%, 0, 0)",
        DrawerDirection::Left => "translate3d(-100%, 0, 0)",
      };

      transform_style.set(format!(
        "transform: {}; transition: transform {}s cubic-bezier(0.32, 0.72, 0, 1);",
        close_transform, TRANSITION_DURATION
      ));

      set_timeout(
        move || {
          context.is_open.set(false);
          if let Some(callback) = context.on_open_change {
            callback.run(false);
          }
        },
        std::time::Duration::from_millis((TRANSITION_DURATION * 1000.0) as u64),
      );
    } else {
      transform_style.set(format!(
        "transform: translate3d(0, 0, 0); transition: transform {}s cubic-bezier(0.32, 0.72, 0, 1);",
        TRANSITION_DURATION
      ));
      context.drag_offset.set(0.0);
    }
  };

  Effect::new(move |_| {
    if context.is_open.get() {
      window_event_listener(leptos::ev::keydown, move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" && context.dismissible {
          context.is_open.set(false);
          if let Some(callback) = context.on_open_change {
            callback.run(false);
          }
        }
      });
    }
  });

  let data_state = move || {
    if context.is_open.get() { "open" } else { "closed" }
  };

  let direction_classes = match context.direction {
    DrawerDirection::Top => {
      "inset-x-0 top-0 mb-24 max-h-[80vh] rounded-b-lg border-b data-[state=closed]:slide-out-to-top data-[state=open]:slide-in-from-top"
    }
    DrawerDirection::Bottom => {
      "inset-x-0 bottom-0 mt-24 max-h-[80vh] rounded-t-lg border-t data-[state=closed]:slide-out-to-bottom data-[state=open]:slide-in-from-bottom"
    }
    DrawerDirection::Left => {
      "inset-y-0 left-0 w-3/4 border-r sm:max-w-sm data-[state=closed]:slide-out-to-left data-[state=open]:slide-in-from-left"
    }
    DrawerDirection::Right => {
      "inset-y-0 right-0 w-3/4 border-l sm:max-w-sm data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right"
    }
  };

  let show_handle = context.direction == DrawerDirection::Bottom;

  view! {
    <Show when=move || should_render.get()>
      <DrawerOverlay />
      <div
        node_ref=content_ref
        data-slot="drawer-content"
        data-state=data_state
        data-vaul-drawer-direction=context.direction.as_str()
        class=cn(
          &[
            "group/drawer-content bg-background fixed z-50 flex h-auto flex-col data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:duration-300 data-[state=open]:duration-500 touch-none",
            direction_classes,
            class.as_str(),
          ],
        )
        style=move || transform_style.get()
        on:pointerdown=handle_pointer_down
        on:pointermove=handle_pointer_move
        on:pointerup=handle_pointer_up
        on:pointercancel=move |_| context.is_dragging.set(false)
      >
        <Show when=move || show_handle>
          <div class="bg-muted mx-auto mt-4 h-2 w-[100px] shrink-0 rounded-full" />
        </Show>
        {children_stored.get_value()()}
      </div>
    </Show>
  }
}

#[component]
pub fn DrawerHeader(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="drawer-header"
      class=cn(
        &[
          "flex flex-col gap-0.5 p-4 group-data-[vaul-drawer-direction=bottom]/drawer-content:text-center group-data-[vaul-drawer-direction=top]/drawer-content:text-center md:gap-1.5 md:text-left",
          class.as_str(),
        ],
      )
    >
      {children()}
    </div>
  }
}

#[component]
pub fn DrawerFooter(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="drawer-footer" class=cn(&["mt-auto flex flex-col gap-2 p-4", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn DrawerTitle(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <h2 data-slot="drawer-title" class=cn(&["text-foreground font-semibold", class.as_str()])>
      {children()}
    </h2>
  }
}

#[component]
pub fn DrawerDescription(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <p data-slot="drawer-description" class=cn(&["text-muted-foreground text-sm", class.as_str()])>
      {children()}
    </p>
  }
}

#[component]
pub fn DrawerHandle(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <div
      data-slot="drawer-handle"
      class=cn(
        &[
          "bg-muted mx-auto mt-4 h-2 w-[100px] shrink-0 rounded-full cursor-grab active:cursor-grabbing",
          class.as_str(),
        ],
      )
    />
  }
}

#[derive(Clone, Copy)]
struct DrawerContext {
  is_open: RwSignal<bool>,
  on_open_change: Option<Callback<bool>>,
  direction: DrawerDirection,
  modal: bool,
  dismissible: bool,
  should_scale_background: bool,
  is_dragging: RwSignal<bool>,
  drag_start_pos: RwSignal<f64>,
  drag_offset: RwSignal<f64>,
  drawer_size: RwSignal<f64>,
}
