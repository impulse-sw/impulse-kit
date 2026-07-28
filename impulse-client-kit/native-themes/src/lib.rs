//! Make a Tauri webview's **base neutral** palette follow the host system's
//! native theme, so the app blends into the OS instead of looking like a random
//! website — while the app keeps its own brand colours.
//!
//! # What this does and doesn't set
//!
//! Every app already defines its identity colours (`--primary`, `--destructive`,
//! `--chart-*`) in its Tailwind `input.css`. Those are left completely alone.
//! What an app can't know is the system's base neutrals — the exact background,
//! text and surface shades the OS uses — so only those tokens are overridden:
//! `--background`, `--foreground`, `--card`, `--popover`, `--muted`,
//! `--secondary`, `--accent` (all with their `-foreground` pairs), `--border`,
//! `--input` and `--ring`.
//!
//! # Where the palette comes from
//!
//! The obvious trick — pointing the tokens at the CSS system-color keywords
//! (`Canvas`, `CanvasText`, …) — does **not** work: WebKitGTK and Android's
//! WebView both resolve them to a generic grey rather than the real desktop
//! theme or the Material You palette, which silently replaces a nicer app
//! palette with grey. So the palette is produced natively and pushed into the
//! webview as explicit CSS variables.
//!
//! Platforms fall into two camps, and the providers reflect that:
//!
//! * **The user picks the neutrals** — Linux (the chosen GTK theme) and Android
//!   (Material You, derived from the wallpaper). These are read at runtime
//!   through the platform's own APIs.
//! * **The neutrals are part of the design language** — Windows (Fluent), macOS
//!   (AppKit) and iOS (UIKit). Every app there draws on the same published
//!   surface and text colours, with light/dark the only variable, so those are
//!   tabled rather than queried.
//!
//! A provider fills in whichever schemes its platform can describe: the tabled
//! ones publish both, while GTK can only describe the theme currently in use.
//!
//! # Wiring an app up
//!
//! 1. **Native, in the Tauri shell** — capture the palette in `setup` (on the
//!    main thread, which the GTK provider requires) and expose it under the
//!    conventional command name [`NATIVE_BASE_THEME_COMMAND`]:
//!
//!    ```ignore
//!    #[tauri::command]
//!    fn ik_native_base_theme() -> Option<NativeBaseTheme> {
//!      impulse_client_native_themes::native_base_theme()
//!    }
//!
//!    tauri::Builder::default()
//!      .setup(|_app| {
//!        impulse_client_native_themes::install_native_base_theme();
//!        Ok(())
//!      })
//!      .invoke_handler(tauri::generate_handler![ik_native_base_theme])
//!    ```
//!
//! 2. **Webview, at startup** — call [`apply_native_base_theme`] (with the
//!    `tauri` feature enabled so it fetches over IPC).
//!
//! Every step degrades gracefully: a platform with no provider, an app that
//! didn't register the command, or a system too old for dynamic colour all end
//! with the app keeping its own palette from `input.css`.

#![deny(warnings)]

use serde::{Deserialize, Serialize};

#[cfg(any(target_os = "android", target_os = "linux"))]
mod dynsym;

#[cfg(target_os = "android")]
mod android;

/// Matches the Android system bars to the scheme the app is showing. Call it
/// from the [`SYSTEM_BARS_COMMAND`] handler, inside
/// `PlatformWebview::jni_handle().exec(..)` so it runs on the UI thread with the
/// real Activity.
#[cfg(target_os = "android")]
pub use android::apply_status_bar_appearance;

#[cfg(target_os = "linux")]
mod gtk_desktop;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "ios"))]
mod fixed_palettes;

/// The conventional Tauri command name an app registers to serve the captured
/// palette to its webview. Mirrors the kit's other IPC conventions
/// (`ik_http_request`, `ik_ws_send`).
pub const NATIVE_BASE_THEME_COMMAND: &str = "ik_native_base_theme";

/// The conventional Tauri command name for matching the system bars to the
/// app's current scheme. Called with `{ dark: bool }` whenever the app's `dark`
/// class changes. An app that doesn't register it simply keeps the system's
/// default bar styling.
pub const SYSTEM_BARS_COMMAND: &str = "ik_system_bars";

/// The `id` of the `<style>` element carrying the platform-independent hints.
pub const BASE_STYLE_ELEMENT_ID: &str = "impulse-native-base-theme";

/// The `id` of the `<style>` element carrying the captured native palette.
pub const PALETTE_STYLE_ELEMENT_ID: &str = "impulse-native-base-palette";

/// The attribute this crate writes on `<html>` with the system's current scheme
/// (`"dark"` / `"light"`), on platforms where the webview's own
/// `prefers-color-scheme` can't be trusted. Absent means "use the media query".
///
/// It is the contract between this crate and a theme provider that has to
/// resolve a "follow the system" setting — deliberately a DOM attribute, so
/// neither crate has to depend on the other. Whoever reads it should also listen
/// for [`SYSTEM_SCHEME_EVENT`], which fires whenever the value changes.
pub const SYSTEM_SCHEME_ATTRIBUTE: &str = "data-impulse-system-scheme";

/// The event dispatched on `window` whenever [`SYSTEM_SCHEME_ATTRIBUTE`] changes.
pub const SYSTEM_SCHEME_EVENT: &str = "impulse:system-scheme";

/// The base-neutral design tokens for one colour scheme. Values are any valid
/// CSS colour (the providers emit `#rrggbb`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseNeutrals {
  pub background: String,
  pub foreground: String,
  pub card: String,
  pub card_foreground: String,
  pub popover: String,
  pub popover_foreground: String,
  pub muted: String,
  pub muted_foreground: String,
  pub secondary: String,
  pub secondary_foreground: String,
  pub accent: String,
  pub accent_foreground: String,
  pub border: String,
  pub input: String,
  pub ring: String,
}

impl BaseNeutrals {
  /// The token/value pairs, in stylesheet order.
  fn vars(&self) -> [(&'static str, &str); 15] {
    [
      ("--background", &self.background),
      ("--foreground", &self.foreground),
      ("--card", &self.card),
      ("--card-foreground", &self.card_foreground),
      ("--popover", &self.popover),
      ("--popover-foreground", &self.popover_foreground),
      ("--muted", &self.muted),
      ("--muted-foreground", &self.muted_foreground),
      ("--secondary", &self.secondary),
      ("--secondary-foreground", &self.secondary_foreground),
      ("--accent", &self.accent),
      ("--accent-foreground", &self.accent_foreground),
      ("--border", &self.border),
      ("--input", &self.input),
      ("--ring", &self.ring),
    ]
  }

  fn write_block(&self, selector: &str, out: &mut String) {
    out.push_str(selector);
    out.push_str(" {\n");
    for (name, value) in self.vars() {
      out.push_str("  ");
      out.push_str(name);
      out.push_str(": ");
      out.push_str(value);
      out.push_str(";\n");
    }
    out.push_str("}\n");
  }
}

/// A system palette, per colour scheme.
///
/// A provider fills in whichever schemes the platform can actually describe:
/// Android derives **both** from the Material You tonal palettes, while a
/// desktop that only exposes its current theme fills in just that one. A scheme
/// left as `None` simply keeps the app's own colours from `input.css`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NativeBaseTheme {
  pub light: Option<BaseNeutrals>,
  pub dark: Option<BaseNeutrals>,
  /// Whether the system itself is currently showing its dark scheme, on the
  /// platforms where the webview can't be asked.
  ///
  /// Everywhere else this stays `None` and `prefers-color-scheme` is the better
  /// answer, because it updates on its own. Android is the exception: its
  /// Activity declares `configChanges="uiMode"`, so the system theme changing
  /// doesn't recreate it, the WebView never learns, and the media query stays
  /// frozen at whatever it was when the app started.
  pub system_dark: Option<bool>,
}

impl NativeBaseTheme {
  /// Whether the report carries nothing at all — no palette and no scheme.
  pub fn is_empty(&self) -> bool {
    self.light.is_none() && self.dark.is_none() && self.system_dark.is_none()
  }

  /// Whether a palette was captured (as opposed to only a scheme report).
  pub fn has_palette(&self) -> bool {
    self.light.is_some() || self.dark.is_some()
  }

  /// Renders the palette as a stylesheet. The dark block is keyed on the app's
  /// `.dark` class, so the palette follows the scheme the app is *showing*
  /// rather than the OS default — the app's own theme toggle stays authoritative.
  pub fn to_css(&self) -> String {
    let mut css = String::new();
    if let Some(light) = &self.light {
      light.write_block(":root", &mut css);
    }
    if let Some(dark) = &self.dark {
      dark.write_block(":root.dark", &mut css);
    }
    css
  }
}

// ---------------------------------------------------------------------------
// Native side
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
static PALETTE: std::sync::OnceLock<Option<NativeBaseTheme>> = std::sync::OnceLock::new();

/// Captures the system palette and caches it for [`native_base_theme`].
///
/// **Call this from the Tauri `setup` hook**, i.e. on the main thread: the GTK
/// provider may only touch GTK from the thread that initialised it, so capturing
/// lazily from an IPC worker would silently come up empty. Caching here lets the
/// IPC command serve the palette from any thread afterwards.
///
/// Capture happens once; later calls return the same result.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_native_base_theme() -> Option<&'static NativeBaseTheme> {
  PALETTE.get_or_init(capture_native_base_theme).as_ref()
}

/// What to report to the webview: the palette captured by
/// [`install_native_base_theme`], plus the system's *current* scheme.
///
/// The colours are cached — they don't change while the app runs — but the
/// scheme is re-read on every call, so a webview that asks again after the user
/// has been away in the system settings gets the new answer. Callable from any
/// thread.
#[cfg(not(target_arch = "wasm32"))]
pub fn native_base_theme() -> Option<NativeBaseTheme> {
  let mut report = PALETTE.get().and_then(Option::as_ref).cloned().unwrap_or_default();
  report.system_dark = system_prefers_dark();
  (!report.is_empty()).then_some(report)
}

/// Whether the system is currently in its dark scheme, where the platform can
/// say and the webview's own media query cannot be trusted (Android).
#[cfg(not(target_arch = "wasm32"))]
pub fn system_prefers_dark() -> Option<bool> {
  #[cfg(target_os = "android")]
  {
    android::system_prefers_dark()
  }
  #[cfg(not(target_os = "android"))]
  {
    None
  }
}

/// Reads the palette from the platform, bypassing the cache. Prefer
/// [`install_native_base_theme`]; the same threading rules apply.
#[cfg(not(target_arch = "wasm32"))]
pub fn capture_native_base_theme() -> Option<NativeBaseTheme> {
  #[cfg(target_os = "android")]
  let captured = android::capture();
  #[cfg(target_os = "linux")]
  let captured = gtk_desktop::capture();
  #[cfg(any(target_os = "windows", target_os = "macos", target_os = "ios"))]
  let captured = fixed_palettes::capture();
  #[cfg(not(any(
    target_os = "android",
    target_os = "linux",
    target_os = "windows",
    target_os = "macos",
    target_os = "ios"
  )))]
  let captured: Option<NativeBaseTheme> = None;

  captured.filter(|theme| !theme.is_empty())
}

// ---------------------------------------------------------------------------
// Webview side
// ---------------------------------------------------------------------------

/// Platform-independent hints, applied synchronously before the palette arrives.
///
/// Tying `color-scheme` to the app's `.dark` class gives native-looking
/// scrollbars, form controls and (on mobile) overscroll glow for the scheme the
/// app is actually showing. It sets no colour tokens, so the app's palette is
/// untouched if no native palette turns up.
#[cfg(target_arch = "wasm32")]
const BASE_HINTS_CSS: &str = ":root { color-scheme: light; }\n:root.dark { color-scheme: dark; }\n";

/// Applies the system-native base theme to the current document: the
/// platform-independent hints immediately, then — with the `tauri` feature — the
/// captured native palette once it arrives over IPC.
///
/// Idempotent and safe to call before hydration. A no-op off wasm.
#[cfg(target_arch = "wasm32")]
pub fn apply_native_base_theme() {
  inject_style(BASE_STYLE_ELEMENT_ID, BASE_HINTS_CSS);
  #[cfg(feature = "tauri")]
  {
    ipc::fetch_and_apply();
    ipc::track_scheme_for_system_bars();
    ipc::refetch_when_visible();
  }
}

/// Publishes the system's scheme on `<html>` for a theme provider to resolve
/// "follow the system" against, and announces the change.
///
/// `None` removes the attribute, which means "the media query is trustworthy
/// here" — the case on every platform but Android.
#[cfg(all(target_arch = "wasm32", feature = "tauri"))]
fn publish_system_scheme(system_dark: Option<bool>) {
  let Some(window) = web_sys::window() else { return };
  let Some(root) = window.document().and_then(|d| d.document_element()) else {
    return;
  };
  let previous = root.get_attribute(SYSTEM_SCHEME_ATTRIBUTE);
  let current = system_dark.map(|dark| if dark { "dark" } else { "light" }.to_string());
  if previous == current {
    return;
  }
  match &current {
    Some(scheme) => {
      let _ = root.set_attribute(SYSTEM_SCHEME_ATTRIBUTE, scheme);
    }
    None => {
      let _ = root.remove_attribute(SYSTEM_SCHEME_ATTRIBUTE);
    }
  }
  if let Ok(event) = web_sys::Event::new(SYSTEM_SCHEME_EVENT) {
    let _ = window.dispatch_event(&event);
  }
}

/// Whether the document is currently showing the dark scheme, i.e. carries the
/// `dark` class the kit's `ThemeProvider` manages.
#[cfg(all(target_arch = "wasm32", feature = "tauri"))]
fn is_dark_scheme() -> bool {
  web_sys::window()
    .and_then(|w| w.document())
    .and_then(|d| d.document_element())
    .is_some_and(|root| root.class_list().contains("dark"))
}

/// No-op stand-in off wasm, so call sites need no target gating of their own.
#[cfg(not(target_arch = "wasm32"))]
pub fn apply_native_base_theme() {}

/// Appends (or replaces the contents of) a `<style>` element with `id`.
///
/// Appending to the end of `<head>` puts these rules after the app's stylesheet,
/// so they win at equal specificity.
#[cfg(target_arch = "wasm32")]
fn inject_style(id: &str, css: &str) {
  let Some(document) = web_sys::window().and_then(|w| w.document()) else {
    return;
  };
  if let Some(existing) = document.get_element_by_id(id) {
    existing.set_text_content(Some(css));
    return;
  }
  let Ok(style) = document.create_element("style") else {
    return;
  };
  let _ = style.set_attribute("id", id);
  style.set_text_content(Some(css));
  if let Some(head) = document.head() {
    let _ = head.append_child(&style);
  } else if let Some(root) = document.document_element() {
    let _ = root.append_child(&style);
  }
}

/// Pulls the captured palette from the native side over Tauri IPC.
#[cfg(all(target_arch = "wasm32", feature = "tauri"))]
mod ipc {
  use wasm_bindgen::JsCast;
  use wasm_bindgen::JsValue;
  use wasm_bindgen::prelude::wasm_bindgen;

  use super::{NativeBaseTheme, PALETTE_STYLE_ELEMENT_ID};

  #[wasm_bindgen]
  extern "C" {
    // Tauri v2 global binding (requires `withGlobalTauri: true`).
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
  }

  /// Tells the native side which scheme the app is showing, so it can match the
  /// system bars to it.
  fn report_scheme() {
    #[derive(serde::Serialize)]
    struct Args {
      dark: bool,
    }
    let Ok(args) = serde_wasm_bindgen::to_value(&Args {
      dark: super::is_dark_scheme(),
    }) else {
      return;
    };
    wasm_bindgen_futures::spawn_local(async move {
      let _ = invoke(super::SYSTEM_BARS_COMMAND, args).await;
    });
  }

  /// Reports the current scheme, then keeps reporting it whenever the app's
  /// `dark` class changes.
  ///
  /// The app owns its light/dark choice (the kit's `ThemeProvider` toggles that
  /// class), and the system bars are drawn over the app's own background because
  /// a Tauri app goes edge-to-edge — so the native side has to be told, or the
  /// bars keep the icon colour Android picked from the app's *theme* at startup
  /// and, after a switch, become invisible against the new background.
  pub(super) fn track_scheme_for_system_bars() {
    report_scheme();

    let Some(root) = web_sys::window()
      .and_then(|w| w.document())
      .and_then(|d| d.document_element())
    else {
      return;
    };
    let callback = wasm_bindgen::prelude::Closure::<dyn FnMut()>::new(report_scheme);
    let Ok(observer) = web_sys::MutationObserver::new(callback.as_ref().unchecked_ref()) else {
      return;
    };
    let options = web_sys::MutationObserverInit::new();
    options.set_attributes(true);
    options.set_attribute_filter(&js_sys::Array::of1(&"class".into()));
    if observer.observe_with_options(&root, &options).is_err() {
      return;
    }
    // The observer and its callback must outlive this call; they live as long as
    // the document does.
    callback.forget();
    std::mem::forget(observer);
  }

  /// Re-asks the native side whenever the app comes back to the foreground.
  ///
  /// Changing the system theme means leaving the app for the settings (or the
  /// notification shade), so returning is precisely when the answer may have
  /// changed. This is what makes "follow the system" live on Android, where the
  /// WebView's own media query never updates.
  pub(super) fn refetch_when_visible() {
    let Some(window) = web_sys::window() else { return };
    let on_visible = wasm_bindgen::prelude::Closure::<dyn FnMut()>::new(|| {
      let visible = web_sys::window()
        .and_then(|w| w.document())
        .is_none_or(|d| d.visibility_state() == web_sys::VisibilityState::Visible);
      if visible {
        fetch_and_apply();
      }
    });
    let handler = on_visible.as_ref().unchecked_ref();
    let _ = window.add_event_listener_with_callback("focus", handler);
    if let Some(document) = window.document() {
      let _ = document.add_event_listener_with_callback("visibilitychange", handler);
    }
    // Lives as long as the document.
    on_visible.forget();
  }

  /// Asks the native side for the palette and injects it. Any failure — no
  /// command registered, no provider for the platform, a decode error — leaves
  /// the app's own palette in place.
  pub(super) fn fetch_and_apply() {
    wasm_bindgen_futures::spawn_local(async {
      // `undefined` lets Tauri's `invoke(cmd, args = {})` default kick in — the
      // command takes no arguments.
      let Ok(value) = invoke(super::NATIVE_BASE_THEME_COMMAND, JsValue::UNDEFINED).await else {
        return;
      };
      let Ok(Some(theme)) = serde_wasm_bindgen::from_value::<Option<NativeBaseTheme>>(value) else {
        return;
      };
      if theme.has_palette() {
        super::inject_style(PALETTE_STYLE_ELEMENT_ID, &theme.to_css());
      }
      super::publish_system_scheme(theme.system_dark);
    });
  }
}
