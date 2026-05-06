#![allow(missing_docs, dead_code)]

//! Theme management.
//!
//! On `csr`/`hydrate` the theme lives in `localStorage` and is synced to the
//! `<html class="dark">` attribute via an `Effect`. On `ssr` the theme is
//! resolved from the `impulse_theme` cookie by the server-side SSR handler
//! (`impulse-server-kit::leptos_ssr`) which writes the corresponding class
//! into the rendered HTML prefix; this component therefore renders only its
//! children without touching any DOM.

use leptos::prelude::*;

use super::button::{Button, ButtonSize, ButtonVariant};

pub const LIGHT_THEME: &str = "light";
pub const DARK_THEME: &str = "dark";

pub const THEME_LOCAL_STORAGE_KEY: &str = "theme";
pub const THEME_COOKIE_KEY: &str = "impulse_theme";

#[cfg(any(feature = "csr", feature = "hydrate"))]
#[component]
pub fn ThemeProvider(children: Children) -> impl IntoView {
  use codee::string::FromToStringCodec;
  use leptos_use::{storage::use_local_storage, use_preferred_dark};

  let preferred_dark = use_preferred_dark();
  let (stored_theme, ..) = use_local_storage::<String, FromToStringCodec>(THEME_LOCAL_STORAGE_KEY);

  Effect::new(move |_| {
    if let Some(document) = document().document_element() {
      match stored_theme.get().as_str() {
        DARK_THEME => {
          let _ = document.class_list().add_1(DARK_THEME);
        }
        LIGHT_THEME => {
          let _ = document.class_list().remove_1(DARK_THEME);
        }
        _ => {
          if preferred_dark.get() {
            let _ = document.class_list().add_1(DARK_THEME);
          } else {
            let _ = document.class_list().remove_1(DARK_THEME);
          }
        }
      }
    }
  });

  view! { {children()} }
}

#[cfg(feature = "ssr")]
#[component]
pub fn ThemeProvider(children: Children) -> impl IntoView {
  view! { {children()} }
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
#[component]
pub fn ThemeToggle(
  #[prop(optional)] variant: ButtonVariant,
  #[prop(optional)] size: ButtonSize,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  use codee::string::FromToStringCodec;
  use leptos_use::{storage::use_local_storage, use_preferred_dark};

  let preferred_dark = use_preferred_dark();
  let (stored_theme, set_stored_theme, ..) =
    use_local_storage::<String, FromToStringCodec>(THEME_LOCAL_STORAGE_KEY);

  let toggle_theme = move |_| match stored_theme.get().as_str() {
    LIGHT_THEME => set_stored_theme.set(DARK_THEME.to_string()),
    DARK_THEME => set_stored_theme.set(LIGHT_THEME.to_string()),
    _ => {
      if preferred_dark.get() {
        set_stored_theme.set(LIGHT_THEME.to_string())
      } else {
        set_stored_theme.set(DARK_THEME.to_string())
      }
    }
  };

  view! {
    <Button variant=variant size=size class=class on:click=toggle_theme>
      {children()}
    </Button>
  }
}

#[cfg(feature = "ssr")]
#[component]
pub fn ThemeToggle(
  #[prop(optional)] variant: ButtonVariant,
  #[prop(optional)] size: ButtonSize,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <Button variant=variant size=size class=class>
      {children()}
    </Button>
  }
}
