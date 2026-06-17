#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

#[derive(Copy, Clone, PartialEq, Default)]
pub enum ToggleVariant {
  #[default]
  Default,
  Outline,
}

#[derive(Copy, Clone, PartialEq, Default)]
pub enum ToggleSize {
  #[default]
  Default,
  Sm,
  Lg,
}

impl ToggleVariant {
  pub fn class(&self) -> &'static str {
    match self {
      Self::Default => "bg-transparent",
      Self::Outline => "border border-input bg-transparent hover:bg-accent hover:text-accent-foreground",
    }
  }
}

impl ToggleSize {
  pub fn class(&self) -> &'static str {
    match self {
      Self::Default => "h-10 px-3",
      Self::Sm => "h-9 px-2.5",
      Self::Lg => "h-11 px-5",
    }
  }
}

#[component]
pub fn Toggle(
  #[prop(optional, into)] pressed: Option<RwSignal<bool>>,
  #[prop(optional, into)] default_pressed: bool,
  #[prop(optional)] on_pressed_change: Option<Callback<bool>>,
  #[prop(optional)] variant: ToggleVariant,
  #[prop(optional)] size: ToggleSize,
  #[prop(optional)] disabled: bool,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let pressed = pressed.unwrap_or_else(|| RwSignal::new(default_pressed));

  let handle_click = move |_| {
    if !disabled {
      let new_pressed = !pressed.get();
      pressed.set(new_pressed);
      if let Some(callback) = on_pressed_change {
        callback.run(new_pressed);
      }
    }
  };

  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    if (ev.key() == " " || ev.key() == "Enter") && !disabled {
      ev.prevent_default();
      let new_pressed = !pressed.get();
      pressed.set(new_pressed);
      if let Some(callback) = on_pressed_change {
        callback.run(new_pressed);
      }
    }
  };

  view! {
    <button
      type="button"
      role="button"
      aria-pressed=move || if pressed.get() { "true" } else { "false" }
      data-state=move || if pressed.get() { "on" } else { "off" }
      data-slot="toggle"
      disabled=disabled
      class=cn(
        &[
          "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium ring-offset-background transition-colors hover:bg-muted hover:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 data-[state=on]:bg-accent data-[state=on]:text-accent-foreground [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0",
          variant.class(),
          size.class(),
          class.as_str(),
        ],
      )
      on:click=handle_click
      on:keydown=handle_keydown
      tabindex=if disabled { "-1" } else { "0" }
    >
      {children()}
    </button>
  }
}
