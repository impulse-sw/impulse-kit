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
//! # Why the palette is read natively
//!
//! The obvious trick — pointing the tokens at the CSS system-color keywords
//! (`Canvas`, `CanvasText`, …) — does **not** work: WebKitGTK and Android's
//! WebView both resolve them to a generic grey rather than the real desktop
//! theme or the Material You palette, which silently replaces a nicer app
//! palette with grey. So the colours are read on the native side, where the real
//! system APIs live, and pushed into the webview as explicit CSS variables.
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
//!      impulse_client_native_themes::native_base_theme().cloned()
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
//!    Enable the `gtk-desktop` feature wherever the Linux desktop bundle is
//!    built, so the GTK provider is compiled in.
//!
//! 2. **Webview, at startup** — call [`apply_native_base_theme`] (with the
//!    `tauri` feature enabled so it fetches over IPC).
//!
//! Every step degrades gracefully: a platform with no provider, an app that
//! didn't register the command, or a system too old for dynamic colour all end
//! with the app keeping its own palette from `input.css`.

#![deny(warnings)]

use serde::{Deserialize, Serialize};

#[cfg(target_os = "android")]
mod android;

#[cfg(all(target_os = "linux", feature = "gtk-desktop"))]
mod gtk_desktop;

/// The conventional Tauri command name an app registers to serve the captured
/// palette to its webview. Mirrors the kit's other IPC conventions
/// (`ik_http_request`, `ik_ws_send`).
pub const NATIVE_BASE_THEME_COMMAND: &str = "ik_native_base_theme";

/// The `id` of the `<style>` element carrying the platform-independent hints.
pub const BASE_STYLE_ELEMENT_ID: &str = "impulse-native-base-theme";

/// The `id` of the `<style>` element carrying the captured native palette.
pub const PALETTE_STYLE_ELEMENT_ID: &str = "impulse-native-base-palette";

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
}

impl NativeBaseTheme {
  /// Whether any scheme was captured at all.
  pub fn is_empty(&self) -> bool {
    self.light.is_none() && self.dark.is_none()
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

/// The palette captured by [`install_native_base_theme`], or `None` when it was
/// never installed, the platform has no provider, or the system exposes no
/// dynamic palette. Cheap and callable from any thread.
#[cfg(not(target_arch = "wasm32"))]
pub fn native_base_theme() -> Option<&'static NativeBaseTheme> {
  PALETTE.get().and_then(Option::as_ref)
}

/// Reads the palette from the platform, bypassing the cache. Prefer
/// [`install_native_base_theme`]; the same threading rules apply.
#[cfg(not(target_arch = "wasm32"))]
pub fn capture_native_base_theme() -> Option<NativeBaseTheme> {
  #[cfg(target_os = "android")]
  let captured = android::capture();
  #[cfg(all(target_os = "linux", feature = "gtk-desktop"))]
  let captured = gtk_desktop::capture();
  // macOS and Windows providers land in a later increment; until then those
  // platforms (and a Linux build without `gtk-desktop`) keep the app's palette.
  #[cfg(not(any(target_os = "android", all(target_os = "linux", feature = "gtk-desktop"))))]
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
  ipc::fetch_and_apply();
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
  use wasm_bindgen::JsValue;
  use wasm_bindgen::prelude::wasm_bindgen;

  use super::{NativeBaseTheme, PALETTE_STYLE_ELEMENT_ID};

  #[wasm_bindgen]
  extern "C" {
    // Tauri v2 global binding (requires `withGlobalTauri: true`).
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
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
      if theme.is_empty() {
        return;
      }
      super::inject_style(PALETTE_STYLE_ELEMENT_ID, &theme.to_css());
    });
  }
}
