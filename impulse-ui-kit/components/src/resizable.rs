#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;
use web_sys::{HtmlElement, PointerEvent};

#[derive(Copy, Clone, PartialEq)]
pub enum ResizableDirection {
  Horizontal,
  Vertical,
}

impl ResizableDirection {
  fn as_str(&self) -> &'static str {
    match self {
      ResizableDirection::Horizontal => "horizontal",
      ResizableDirection::Vertical => "vertical",
    }
  }
}

#[derive(Clone, Copy)]
struct ResizableContext {
  direction: ResizableDirection,
  container_ref: NodeRef<leptos::html::Div>,
  panels: StoredValue<Vec<RwSignal<f64>>>,
}

#[component]
pub fn ResizablePanelGroup(
  #[prop(optional)] direction: Option<ResizableDirection>,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let direction = direction.unwrap_or(ResizableDirection::Horizontal);
  let container_ref = NodeRef::<leptos::html::Div>::new();
  let panels = StoredValue::new(Vec::<RwSignal<f64>>::new());

  provide_context(ResizableContext {
    direction,
    container_ref,
    panels,
  });

  let direction_class = match direction {
    ResizableDirection::Horizontal => "flex-row",
    ResizableDirection::Vertical => "flex-col",
  };

  view! {
    <div
      node_ref=container_ref
      data-slot="resizable-panel-group"
      data-panel-group-direction=direction.as_str()
      class=cn(&["flex h-full w-full", direction_class, class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ResizablePanel(
  #[prop(optional)] default_size: Option<f64>,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let default_size = default_size.unwrap_or(50.0);
  let size = RwSignal::new(default_size);

  if let Some(context) = use_context::<ResizableContext>() {
    context.panels.update_value(|panels| panels.push(size));
  }

  view! {
    <div
      data-slot="resizable-panel"
      class=cn(&["relative overflow-hidden", class.as_str()])
      style:flex=move || format!("{} 1 0%", size.get())
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ResizableHandle(#[prop(into, optional)] class: String) -> impl IntoView {
  let context = use_context::<ResizableContext>().expect("ResizableHandle must be used within ResizablePanelGroup");

  let left_index = context.panels.with_value(|p| p.len().saturating_sub(1));

  let handle_ref = NodeRef::<leptos::html::Div>::new();
  let is_dragging = RwSignal::new(false);
  let drag_start_pos = RwSignal::new(0.0_f64);
  let drag_start_left = RwSignal::new(0.0_f64);
  let drag_start_right = RwSignal::new(0.0_f64);

  let handle_pointer_down = move |ev: PointerEvent| {
    let panels_ok =
      context.panels.with_value(|p| p.len() > left_index + 1) && context.panels.with_value(|p| !p.is_empty());
    if !panels_ok {
      return;
    }

    let (left, right) = context.panels.with_value(|p| (p[left_index], p[left_index + 1]));

    let pos = match context.direction {
      ResizableDirection::Horizontal => ev.client_x() as f64,
      ResizableDirection::Vertical => ev.client_y() as f64,
    };

    drag_start_pos.set(pos);
    drag_start_left.set(left.get());
    drag_start_right.set(right.get());
    is_dragging.set(true);

    ev.prevent_default();

    if let Some(handle_el) = handle_ref.get() {
      let el: &HtmlElement = handle_el.as_ref();
      let _ = el.set_pointer_capture(ev.pointer_id());
    }
  };

  let handle_pointer_move = move |ev: PointerEvent| {
    if !is_dragging.get() {
      return;
    }
    let Some(container) = context.container_ref.get() else {
      return;
    };
    let rect = container.get_bounding_client_rect();
    let total = match context.direction {
      ResizableDirection::Horizontal => rect.width(),
      ResizableDirection::Vertical => rect.height(),
    };
    if total <= 0.0 {
      return;
    }

    let current = match context.direction {
      ResizableDirection::Horizontal => ev.client_x() as f64,
      ResizableDirection::Vertical => ev.client_y() as f64,
    };
    let delta_px = current - drag_start_pos.get();
    let delta_pct = delta_px / total * 100.0;

    let panels_ok = context.panels.with_value(|p| p.len() > left_index + 1);
    if !panels_ok {
      return;
    }
    let (left, right) = context.panels.with_value(|p| (p[left_index], p[left_index + 1]));

    let sum = drag_start_left.get() + drag_start_right.get();
    let min = 10.0_f64;
    if sum <= 2.0 * min {
      return;
    }
    let new_left = (drag_start_left.get() + delta_pct).clamp(min, sum - min);
    let new_right = sum - new_left;

    left.set(new_left);
    right.set(new_right);
  };

  let handle_pointer_up = move |ev: PointerEvent| {
    if is_dragging.get() {
      is_dragging.set(false);
      if let Some(handle_el) = handle_ref.get() {
        let el: &HtmlElement = handle_el.as_ref();
        let _ = el.release_pointer_capture(ev.pointer_id());
      }
    }
  };

  let handle_pointer_cancel = move |ev: PointerEvent| {
    is_dragging.set(false);
    if let Some(handle_el) = handle_ref.get() {
      let el: &HtmlElement = handle_el.as_ref();
      let _ = el.release_pointer_capture(ev.pointer_id());
    }
  };

  let cursor_class = match context.direction {
    ResizableDirection::Horizontal => "cursor-col-resize",
    ResizableDirection::Vertical => "cursor-row-resize",
  };

  let size_class = match context.direction {
    ResizableDirection::Horizontal => "w-px",
    ResizableDirection::Vertical => "h-px w-full",
  };

  let rotate_class = match context.direction {
    ResizableDirection::Horizontal => "",
    ResizableDirection::Vertical => "rotate-90",
  };

  view! {
    <div
      node_ref=handle_ref
      data-slot="resizable-handle"
      data-panel-group-direction=context.direction.as_str()
      class=cn(
        &[
          "relative flex items-center justify-center bg-border touch-none select-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-offset-1",
          size_class,
          cursor_class,
          class.as_str(),
        ],
      )
      on:pointerdown=handle_pointer_down
      on:pointermove=handle_pointer_move
      on:pointerup=handle_pointer_up
      on:pointercancel=handle_pointer_cancel
    >
      <div class=cn(
        &["z-10 flex h-4 w-3 items-center justify-center rounded-sm border bg-border", rotate_class],
      )>
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
          class="h-2.5 w-2.5"
        >
          <circle cx="9" cy="12" r="1" />
          <circle cx="9" cy="5" r="1" />
          <circle cx="9" cy="19" r="1" />
          <circle cx="15" cy="12" r="1" />
          <circle cx="15" cy="5" r="1" />
          <circle cx="15" cy="19" r="1" />
        </svg>
      </div>
    </div>
  }
}
