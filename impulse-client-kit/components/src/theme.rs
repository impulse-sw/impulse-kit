#![allow(missing_docs, dead_code)]

//! Theme management.
//!
//! On `csr`/`hydrate` the theme lives in `localStorage` and is synced to the
//! `<html class="dark">` attribute via an `Effect`. The same `Effect` mirrors
//! the resolved choice into the `impulse_theme` cookie so the server-side SSR
//! handler (`impulse-server-kit::leptos_ssr`) can render the correct class on
//! the next request.
//!
//! On `ssr` the theme is resolved from that cookie by the SSR handler, which
//! also injects a blocking inline script into the `<head>` that reconciles the
//! class from `localStorage` + `prefers-color-scheme` before first paint —
//! together this eliminates the dark-mode flicker. This component therefore
//! renders only its children without touching any DOM.

use leptos::prelude::*;

use super::button::{Button, ButtonSize, ButtonVariant};

pub const LIGHT_THEME: &str = "light";
pub const DARK_THEME: &str = "dark";

pub const THEME_LOCAL_STORAGE_KEY: &str = "theme";
pub const THEME_COOKIE_KEY: &str = "impulse_theme";

/// The attribute a native shell may write on `<html>` to state the system's
/// current scheme, and the event announcing that it changed.
///
/// `prefers-color-scheme` is the right answer nearly everywhere, but not inside
/// an Android WebView: its Activity declares `configChanges="uiMode"`, so a
/// system theme change neither recreates the Activity nor reaches the media
/// query, which stays frozen at whatever it was when the app started.
/// `impulse-client-native-themes` fills this attribute in on such platforms, and
/// [`ThemeMode::System`] resolves against it in preference to the media query.
/// Absent — the normal case — means the media query is trustworthy.
pub const SYSTEM_SCHEME_ATTRIBUTE: &str = "data-impulse-system-scheme";
pub const SYSTEM_SCHEME_EVENT: &str = "impulse:system-scheme";

/// Which theme the user asked for.
///
/// [`System`](ThemeMode::System) is a real, reachable state rather than merely
/// the starting one: an app that only toggles light↔dark leaves anyone who
/// touches the control unable to hand the choice back to the OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
  /// Follow the operating system.
  #[default]
  System,
  Light,
  Dark,
}

impl ThemeMode {
  /// The `localStorage` value. `System` is the empty string, which is also what
  /// an unset key reads as — so the default needs no migration.
  pub fn as_storage(self) -> &'static str {
    match self {
      ThemeMode::System => "",
      ThemeMode::Light => LIGHT_THEME,
      ThemeMode::Dark => DARK_THEME,
    }
  }

  pub fn from_storage(value: &str) -> Self {
    match value {
      LIGHT_THEME => ThemeMode::Light,
      DARK_THEME => ThemeMode::Dark,
      _ => ThemeMode::System,
    }
  }

  /// The next mode in the control's cycle: system → light → dark → system.
  pub fn next(self) -> Self {
    match self {
      ThemeMode::System => ThemeMode::Light,
      ThemeMode::Light => ThemeMode::Dark,
      ThemeMode::Dark => ThemeMode::System,
    }
  }

  /// The explicit choice to persist, if any. `System` is stored as "no choice".
  fn explicit(self) -> Option<&'static str> {
    match self {
      ThemeMode::System => None,
      other => Some(other.as_storage()),
    }
  }
}

/// Persist the resolved theme into the `impulse_theme` cookie so SSR can paint
/// the matching `<html>` class on subsequent requests. An explicit choice
/// (`dark`/`light`) is stored for a year; `None` (the "follow OS" mode) clears
/// the cookie so the server falls back to its default and the blocking head
/// script resolves `prefers-color-scheme` instead.
#[cfg(any(feature = "csr", feature = "hydrate"))]
fn sync_theme_cookie(explicit: Option<&str>) {
  use web_sys::wasm_bindgen::JsCast;

  let Ok(doc) = document().dyn_into::<web_sys::HtmlDocument>() else {
    return;
  };
  let cookie = match explicit {
    Some(value) => format!("{THEME_COOKIE_KEY}={value}; path=/; max-age=31536000; SameSite=Lax"),
    None => format!("{THEME_COOKIE_KEY}=; path=/; max-age=0; SameSite=Lax"),
  };
  let _ = doc.set_cookie(&cookie);
}

/// The system's current scheme, preferring a native shell's report over the
/// media query. Reactive: it re-reads on [`SYSTEM_SCHEME_EVENT`].
#[cfg(any(feature = "csr", feature = "hydrate"))]
fn use_system_prefers_dark() -> Signal<bool> {
  use leptos_use::use_preferred_dark;
  use web_sys::wasm_bindgen::JsCast;
  use web_sys::wasm_bindgen::prelude::Closure;

  let preferred_dark = use_preferred_dark();
  let reported = RwSignal::new(reported_system_scheme());

  // The native shell announces changes; the media query is watched by
  // `use_preferred_dark` already.
  Effect::new(move |previous: Option<()>| {
    if previous.is_some() {
      return;
    }
    let Some(window) = window().dyn_into::<web_sys::Window>().ok() else {
      return;
    };
    let listener = Closure::<dyn FnMut()>::new(move || reported.set(reported_system_scheme()));
    let _ = window.add_event_listener_with_callback(SYSTEM_SCHEME_EVENT, listener.as_ref().unchecked_ref());
    listener.forget();
  });

  Signal::derive(move || reported.get().unwrap_or_else(move || preferred_dark.get()))
}

/// Reads [`SYSTEM_SCHEME_ATTRIBUTE`] off `<html>`, if a native shell set it.
#[cfg(any(feature = "csr", feature = "hydrate"))]
fn reported_system_scheme() -> Option<bool> {
  let value = document().document_element()?.get_attribute(SYSTEM_SCHEME_ATTRIBUTE)?;
  match value.as_str() {
    DARK_THEME => Some(true),
    LIGHT_THEME => Some(false),
    _ => None,
  }
}

/// The current mode and a setter for it, for apps that render their own control
/// (to label it with the active mode, say). [`ThemeToggle`] is built on this.
#[cfg(any(feature = "csr", feature = "hydrate"))]
pub fn use_theme_mode() -> (Signal<ThemeMode>, impl Fn(ThemeMode) + Copy + 'static) {
  use codee::string::FromToStringCodec;
  use leptos_use::storage::use_local_storage;

  let (stored, set_stored, ..) = use_local_storage::<String, FromToStringCodec>(THEME_LOCAL_STORAGE_KEY);
  let mode = Signal::derive(move || ThemeMode::from_storage(&stored.get()));
  let set_mode = move |next: ThemeMode| set_stored.set(next.as_storage().to_string());
  (mode, set_mode)
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
#[component]
pub fn ThemeProvider(children: Children) -> impl IntoView {
  let (mode, _) = use_theme_mode();
  let system_dark = use_system_prefers_dark();

  Effect::new(move |_| {
    let Some(document) = document().document_element() else {
      return;
    };
    let mode = mode.get();
    let dark = match mode {
      ThemeMode::Dark => true,
      ThemeMode::Light => false,
      ThemeMode::System => system_dark.get(),
    };
    if dark {
      let _ = document.class_list().add_1(DARK_THEME);
    } else {
      let _ = document.class_list().remove_1(DARK_THEME);
    }
    // An explicit choice is mirrored for SSR; "follow the system" clears the
    // cookie so the server defers to the blocking head script's own resolution.
    sync_theme_cookie(mode.explicit());
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
  let (mode, set_mode) = use_theme_mode();
  let cycle = move |_| set_mode(mode.get_untracked().next());

  view! {
    <Button variant=variant size=size class=class on:click=cycle>
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
