#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::{HtmlElement, PointerEvent};

#[component]
pub fn Slider(
  #[prop(optional, into)] value: Option<RwSignal<f64>>,
  #[prop(optional)] default_value: Option<f64>,
  #[prop(optional)] min: Option<f64>,
  #[prop(optional)] max: Option<f64>,
  #[prop(optional)] step: Option<f64>,
  #[prop(optional)] disabled: bool,
  #[prop(optional)] on_value_change: Option<Callback<f64>>,
  #[prop(into, optional)] class: String,
) -> impl IntoView {
  let value = value.unwrap_or_else(|| RwSignal::new(default_value.unwrap_or(50.0)));
  let min = min.unwrap_or(0.0);
  let max = max.unwrap_or(100.0);
  let step = step.unwrap_or(1.0);

  let slider_ref = NodeRef::<leptos::html::Span>::new();
  let is_dragging = RwSignal::new(false);

  let percentage = move || ((value.get() - min) / (max - min) * 100.0).clamp(0.0, 100.0);

  let update_value = move |client_x: f64| {
    if disabled {
      return;
    }

    if let Some(slider) = slider_ref.get() {
      let rect = slider.get_bounding_client_rect();
      let percentage = ((client_x - rect.left()) / rect.width()).clamp(0.0, 1.0);
      let raw_value = min + percentage * (max - min);
      let stepped_value = (raw_value / step).round() * step;
      let new_value = stepped_value.clamp(min, max);

      value.set(new_value);
      if let Some(callback) = on_value_change {
        callback.run(new_value);
      }
    }
  };

  let handle_pointer_down = move |ev: PointerEvent| {
    if !disabled {
      is_dragging.set(true);
      update_value(ev.client_x() as f64);

      if let Some(target) = ev.target()
        && let Ok(el) = target.dyn_into::<HtmlElement>()
      {
        let _ = el.set_pointer_capture(ev.pointer_id());
      }
    }
  };

  let handle_pointer_move = move |ev: PointerEvent| {
    if is_dragging.get() && !disabled {
      update_value(ev.client_x() as f64);
    }
  };

  let handle_pointer_up = move |ev: PointerEvent| {
    if is_dragging.get() {
      is_dragging.set(false);

      if let Some(target) = ev.target()
        && let Ok(el) = target.dyn_into::<HtmlElement>()
      {
        let _ = el.release_pointer_capture(ev.pointer_id());
      }
    }
  };

  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    if disabled {
      return;
    }

    let mut new_value = value.get();
    match ev.key().as_str() {
      "ArrowRight" | "ArrowUp" => {
        new_value = (new_value + step).min(max);
        ev.prevent_default();
      }
      "ArrowLeft" | "ArrowDown" => {
        new_value = (new_value - step).max(min);
        ev.prevent_default();
      }
      "Home" => {
        new_value = min;
        ev.prevent_default();
      }
      "End" => {
        new_value = max;
        ev.prevent_default();
      }
      _ => return,
    }

    value.set(new_value);
    if let Some(callback) = on_value_change {
      callback.run(new_value);
    }
  };

  view! {
    <span
      node_ref=slider_ref
      data-slot="slider"
      role="slider"
      aria-valuemin=min
      aria-valuemax=max
      aria-valuenow=move || value.get()
      aria-disabled=disabled
      tabindex=if disabled { "-1" } else { "0" }
      class=cn(&["relative flex w-full touch-none select-none items-center", class.as_str()])
      on:pointerdown=handle_pointer_down
      on:pointermove=handle_pointer_move
      on:pointerup=handle_pointer_up
      on:pointercancel=move |_| is_dragging.set(false)
      on:keydown=handle_keydown
    >
      <span
        data-slot="slider-track"
        class="relative h-2 w-full grow overflow-hidden rounded-full bg-secondary"
      >
        <span
          data-slot="slider-range"
          class="absolute h-full bg-primary"
          style:width=move || format!("{}%", percentage())
        />
      </span>
      <span
        data-slot="slider-thumb"
        class="block h-5 w-5 rounded-full border-2 border-primary bg-background ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
        style:left=move || format!("calc({}% - 10px)", percentage())
      />
    </span>
  }
}
