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

/// The icon a mode falls back to when the caller named none: a sun, a moon and
/// a monitor, drawn in the same 24×24 stroked style as the rest of the kit's
/// built-in glyphs.
///
/// A theme control is an icon button in almost every app that has one, so
/// making every caller supply the same three SVGs (or take on an icon set to
/// name them) is a toll on the common case. Defaults mean `<ThemeToggle/>` is a
/// finished control; a caller who wants their own passes it and nothing here
/// applies. Note that these say which mode the toggle is *in*, not which one it
/// would switch to — the sun is the light theme, not "go light".
fn default_icon(mode: ThemeMode) -> AnyView {
  match mode {
    ThemeMode::Light => view! {
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
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
      </svg>
    }
    .into_any(),
    ThemeMode::Dark => view! {
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
        <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" />
      </svg>
    }
    .into_any(),
    ThemeMode::System => view! {
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
        <rect width="20" height="14" x="2" y="3" rx="2" />
        <path d="M8 21h8" />
        <path d="M12 17v4" />
      </svg>
    }
    .into_any(),
  }
}

/// What a [`ThemeToggle`] shows for each mode: an optional icon and an optional
/// label per mode, any subset of which may be left out.
///
/// A control that cycles through three modes has to say which one it is *in*,
/// so an unset icon falls back to [`default_icon`]. With nothing set at all the
/// toggle renders its children unchanged instead, which is what a caller who
/// labels the button themselves wants.
struct ThemeFaces {
  light_icon: Option<ViewFn>,
  dark_icon: Option<ViewFn>,
  system_icon: Option<ViewFn>,
  light_text: Option<TextProp>,
  dark_text: Option<TextProp>,
  system_text: Option<TextProp>,
}

impl ThemeFaces {
  /// Nothing per-mode was passed, so the children stand on their own.
  fn is_empty(&self) -> bool {
    self.light_icon.is_none()
      && self.dark_icon.is_none()
      && self.system_icon.is_none()
      && self.light_text.is_none()
      && self.dark_text.is_none()
      && self.system_text.is_none()
  }

  fn icon(&self, mode: ThemeMode) -> Option<&ViewFn> {
    match mode {
      ThemeMode::Light => self.light_icon.as_ref(),
      ThemeMode::Dark => self.dark_icon.as_ref(),
      ThemeMode::System => self.system_icon.as_ref(),
    }
  }

  fn text(&self, mode: ThemeMode) -> Option<&TextProp> {
    match mode {
      ThemeMode::Light => self.light_text.as_ref(),
      ThemeMode::Dark => self.dark_text.as_ref(),
      ThemeMode::System => self.system_text.as_ref(),
    }
  }
}

/// The toggle's label: the current mode's icon and text, the children instead
/// when the caller gave no face at all and did give children.
///
/// Children are the "I label this button myself" escape hatch, so the built-in
/// icons stay out of their way; naming even one face opts back into the
/// per-mode rendering, and the modes left unnamed get the default icon.
fn theme_faces_view(faces: ThemeFaces, mode: Signal<ThemeMode>, children: Option<Children>) -> AnyView {
  if faces.is_empty()
    && let Some(children) = children
  {
    return children().into_any();
  }

  let faces = StoredValue::new(faces);
  let icon = move || {
    let mode = mode.get();
    faces.with_value(|faces| match faces.icon(mode) {
      Some(icon) => icon.run(),
      None => default_icon(mode),
    })
  };
  let text = move || {
    faces.with_value(|faces| {
      faces
        .text(mode.get())
        .map(|text| view! { <span data-slot="theme-toggle-text">{text.get().to_string()}</span> })
    })
  };

  view! {
    {icon}
    {text}
  }
  .into_any()
}

/// A button that cycles the theme: system → light → dark → system.
///
/// Left bare it is already a working icon button: it shows the face of the mode
/// it is *currently* in, using the built-in sun / moon / monitor icons.
///
/// ```rust,ignore
/// view! { <ThemeToggle size=ButtonSize::IconSm variant=ButtonVariant::Ghost /> }
/// ```
///
/// Any of the six per-mode props replaces a face — the icon first, then the
/// text, both optional and both settable per mode. An icon left unset keeps the
/// built-in one, so overriding a single mode is one prop, and a mode that should
/// show no icon at all takes `light_icon=|| ()`:
///
/// ```rust,ignore
/// view! {
///   <ThemeToggle
///     system_icon=|| view! { <Icon icon=icondata::LuMonitor /> }
///     light_icon=|| view! { <Icon icon=icondata::LuSun /> }
///     dark_icon=|| view! { <Icon icon=icondata::LuMoon /> }
///     system_text="System"
///     light_text="Light"
///     dark_text="Dark"
///   />
/// }
/// ```
///
/// Children replace the whole label — with them and no per-mode prop the button
/// renders exactly what it was given and no built-in icon.
///
/// Under `ssr` the mode is not known to the component (the SSR handler resolves
/// it from the theme cookie), so the `System` face is rendered and hydration
/// corrects it.
#[cfg(any(feature = "csr", feature = "hydrate"))]
#[component]
pub fn ThemeToggle(
  #[prop(optional)] variant: ButtonVariant,
  #[prop(optional)] size: ButtonSize,
  #[prop(into, optional)] class: String,
  /// Shown while the theme follows the operating system. Defaults to a monitor.
  #[prop(into, optional)]
  system_icon: Option<ViewFn>,
  /// Shown while the theme is pinned to light. Defaults to a sun.
  #[prop(into, optional)]
  light_icon: Option<ViewFn>,
  /// Shown while the theme is pinned to dark. Defaults to a moon.
  #[prop(into, optional)]
  dark_icon: Option<ViewFn>,
  /// Label for the "follow the operating system" mode.
  #[prop(into, optional)]
  system_text: Option<TextProp>,
  /// Label for the light mode.
  #[prop(into, optional)]
  light_text: Option<TextProp>,
  /// Label for the dark mode.
  #[prop(into, optional)]
  dark_text: Option<TextProp>,
  /// Rendered as-is, in place of the built-in icons, when no per-mode icon or
  /// text was given.
  #[prop(optional)]
  children: Option<Children>,
) -> impl IntoView {
  let (mode, set_mode) = use_theme_mode();
  let cycle = move |_| set_mode(mode.get_untracked().next());

  let content = theme_faces_view(
    ThemeFaces {
      light_icon,
      dark_icon,
      system_icon,
      light_text,
      dark_text,
      system_text,
    },
    mode,
    children,
  );

  view! {
    <Button variant=variant size=size class=class on:click=cycle>
      {content}
    </Button>
  }
}

/// See the `csr`/`hydrate` [`ThemeToggle`] for the prop reference; on the server
/// the button is inert and the `System` face is rendered.
#[cfg(feature = "ssr")]
#[component]
pub fn ThemeToggle(
  #[prop(optional)] variant: ButtonVariant,
  #[prop(optional)] size: ButtonSize,
  #[prop(into, optional)] class: String,
  #[prop(into, optional)] system_icon: Option<ViewFn>,
  #[prop(into, optional)] light_icon: Option<ViewFn>,
  #[prop(into, optional)] dark_icon: Option<ViewFn>,
  #[prop(into, optional)] system_text: Option<TextProp>,
  #[prop(into, optional)] light_text: Option<TextProp>,
  #[prop(into, optional)] dark_text: Option<TextProp>,
  #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
  let content = theme_faces_view(
    ThemeFaces {
      light_icon,
      dark_icon,
      system_icon,
      light_text,
      dark_text,
      system_text,
    },
    Signal::stored(ThemeMode::default()),
    children,
  );

  view! {
    <Button variant=variant size=size class=class>
      {content}
    </Button>
  }
}
