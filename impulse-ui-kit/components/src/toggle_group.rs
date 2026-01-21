#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

use super::toggle::{ToggleSize, ToggleVariant};

#[derive(Clone, Copy)]
struct ToggleGroupContext {
  value: RwSignal<Vec<String>>,
  on_value_change: Option<Callback<Vec<String>>>,
  r#type: ToggleGroupType,
  disabled: bool,
  variant: ToggleVariant,
  size: ToggleSize,
}

#[derive(Copy, Clone, PartialEq)]
pub enum ToggleGroupType {
  Single,
  Multiple,
}

#[component]
pub fn ToggleGroup(
  #[prop(optional, into)] value: Option<RwSignal<Vec<String>>>,
  #[prop(optional)] default_value: Option<Vec<String>>,
  #[prop(optional)] on_value_change: Option<Callback<Vec<String>>>,
  #[prop(optional)] r#type: Option<ToggleGroupType>,
  #[prop(optional)] variant: Option<ToggleVariant>,
  #[prop(optional)] size: Option<ToggleSize>,
  #[prop(optional)] disabled: bool,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let value = value.unwrap_or_else(|| RwSignal::new(default_value.unwrap_or_default()));
  let r#type = r#type.unwrap_or(ToggleGroupType::Single);
  let variant = variant.unwrap_or(ToggleVariant::Default);
  let size = size.unwrap_or(ToggleSize::Default);

  provide_context(ToggleGroupContext {
    value,
    on_value_change,
    r#type,
    disabled,
    variant,
    size,
  });

  view! {
    <div
      data-slot="toggle-group"
      role="group"
      class=cn(&["inline-flex items-center justify-center gap-1 rounded-md", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ToggleGroupItem(
  #[prop(into)] value: String,
  #[prop(optional)] disabled: bool,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let context = use_context::<ToggleGroupContext>().expect("ToggleGroupItem must be used within ToggleGroup");

  let is_disabled = disabled || context.disabled;
  let value_clone = value.clone();
  let is_pressed = move || context.value.get().contains(&value_clone);

  let handle_click = move |_| {
    if !is_disabled {
      let mut current = context.value.get();
      match context.r#type {
        ToggleGroupType::Single => {
          if current.contains(&value) {
            current.clear();
          } else {
            current.clear();
            current.push(value.clone());
          }
        }
        ToggleGroupType::Multiple => {
          if let Some(pos) = current.iter().position(|v| v == &value) {
            current.remove(pos);
          } else {
            current.push(value.clone());
          }
        }
      }
      context.value.set(current.clone());
      if let Some(callback) = context.on_value_change {
        callback.run(current);
      }
    }
  };

  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    if (ev.key() == " " || ev.key() == "Enter") && !is_disabled {
      ev.prevent_default();
      let mut current = context.value.get();
      match context.r#type {
        ToggleGroupType::Single => {
          if current.contains(&value) {
            current.clear();
          } else {
            current.clear();
            current.push(value.clone());
          }
        }
        ToggleGroupType::Multiple => {
          if let Some(pos) = current.iter().position(|v| v == &value) {
            current.remove(pos);
          } else {
            current.push(value.clone());
          }
        }
      }
      context.value.set(current.clone());
      if let Some(callback) = context.on_value_change {
        callback.run(current);
      }
    }
  };

  let variant_class = context.variant.class();
  let size_class = context.size.class();

  view! {
    <button
      type="button"
      role="button"
      aria-pressed=move || if is_pressed() { "true" } else { "false" }
      data-state=move || if is_pressed() { "on" } else { "off" }
      data-slot="toggle-group-item"
      disabled=is_disabled
      class=cn(
        &[
          "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium ring-offset-background transition-colors hover:bg-muted hover:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 data-[state=on]:bg-accent data-[state=on]:text-accent-foreground [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0",
          variant_class,
          size_class,
          class.as_str(),
        ],
      )
      on:click=handle_click
      on:keydown=handle_keydown
      tabindex=if is_disabled { "-1" } else { "0" }
    >
      {children()}
    </button>
  }
}
