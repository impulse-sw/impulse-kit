//! Make a Tauri webview's **base neutral** palette follow the host system's
//! native theme, so the app blends into the OS chrome instead of looking like a
//! random website — while the app keeps its own brand colours.
//!
//! # Why a runtime, client-side approach
//!
//! Each app already defines its identity colours (`--primary`, `--destructive`,
//! `--chart-*`) in its Tailwind `input.css`. What it *doesn't* know is the
//! system's base neutrals — the exact background, text and surface shades the OS
//! uses. In a browser a bespoke neutral palette is fine; inside a Tauri window
//! on macOS, Windows, a Linux desktop, or Android (Material Design 3) it reads
//! as foreign.
//!
//! Rather than baking a per-OS palette at build time, we let the webview read
//! the *live* system colours through the CSS system-color keywords
//! ([`Canvas`], [`CanvasText`], [`GrayText`]). Every Tauri webview engine —
//! WebKitGTK (Linux), WKWebView (macOS), WebView2 (Windows) and Android's
//! Chromium WebView — resolves these from the real OS theme, and they update
//! live when the user changes it. [`color-mix()`] derives the intermediate
//! surfaces from those anchors.
//!
//! [`apply_native_base_theme`] injects a small stylesheet that maps the
//! **base-neutral** design tokens onto those system colours and ties
//! `color-scheme` to the app's `.dark` class, so the system colours resolve for
//! whichever scheme the app is actually showing (not just the OS default). It
//! is appended after the app's stylesheet, so at equal specificity these win;
//! and because it only sets the neutral tokens, `--primary` & friends are
//! untouched.
//!
//! Degradation is graceful: an engine that doesn't understand a system color or
//! `color-mix()` simply drops that declaration, and the token falls back to the
//! app's own value from `input.css`.
//!
//! # Usage
//!
//! Call once at webview startup, only in the Tauri build (a plain website should
//! keep its own neutrals):
//!
//! ```ignore
//! #[cfg(feature = "tauri")]
//! impulse_client_native_themes::apply_native_base_theme();
//! ```
//!
//! [`Canvas`]: https://developer.mozilla.org/en-US/docs/Web/CSS/system-color
//! [`CanvasText`]: https://developer.mozilla.org/en-US/docs/Web/CSS/system-color
//! [`GrayText`]: https://developer.mozilla.org/en-US/docs/Web/CSS/system-color
//! [`color-mix()`]: https://developer.mozilla.org/en-US/docs/Web/CSS/color_value/color-mix

#![deny(warnings)]

/// The `id` of the injected `<style>` element, used to keep injection idempotent.
pub const STYLE_ELEMENT_ID: &str = "impulse-native-base-theme";

/// The stylesheet injected by [`apply_native_base_theme`].
///
/// * `color-scheme` is tied to the app's `.dark` class so the system colours
///   resolve for the scheme the app is *showing*, even if it differs from the OS
///   default. This also gives native dark form controls and scrollbars.
/// * Only base-neutral tokens are set. Brand tokens (`--primary`,
///   `--destructive`, `--chart-*`, …) are deliberately omitted.
/// * `--background`/`--foreground`/surfaces map to system colours directly;
///   intermediate surfaces are mixed from the same anchors, so everything tracks
///   one coherent system palette.
pub const NATIVE_BASE_THEME_CSS: &str = "\
:root { color-scheme: light; }\n\
:root.dark { color-scheme: dark; }\n\
:root {\n\
  --background: Canvas;\n\
  --foreground: CanvasText;\n\
  --card: Canvas;\n\
  --card-foreground: CanvasText;\n\
  --popover: Canvas;\n\
  --popover-foreground: CanvasText;\n\
  --muted: color-mix(in srgb, Canvas 92%, CanvasText);\n\
  --muted-foreground: GrayText;\n\
  --secondary: color-mix(in srgb, Canvas 90%, CanvasText);\n\
  --secondary-foreground: CanvasText;\n\
  --accent: color-mix(in srgb, Canvas 90%, CanvasText);\n\
  --accent-foreground: CanvasText;\n\
  --border: color-mix(in srgb, Canvas 86%, CanvasText);\n\
  --input: color-mix(in srgb, Canvas 86%, CanvasText);\n\
  --ring: color-mix(in srgb, Canvas 55%, CanvasText);\n\
}\n";

/// Injects [`NATIVE_BASE_THEME_CSS`] into `<head>` so the app's base-neutral
/// tokens follow the system theme. Idempotent (a second call is a no-op) and
/// safe to call before hydration. No-op on non-webview targets.
#[cfg(target_arch = "wasm32")]
pub fn apply_native_base_theme() {
  let Some(document) = web_sys::window().and_then(|w| w.document()) else {
    return;
  };
  // Already injected — nothing to do.
  if document.get_element_by_id(STYLE_ELEMENT_ID).is_some() {
    return;
  }
  let Ok(style) = document.create_element("style") else {
    return;
  };
  let _ = style.set_attribute("id", STYLE_ELEMENT_ID);
  style.set_text_content(Some(NATIVE_BASE_THEME_CSS));
  // Append after the app's stylesheet so these rules win at equal specificity.
  if let Some(head) = document.head() {
    let _ = head.append_child(&style);
  } else if let Some(root) = document.document_element() {
    let _ = root.append_child(&style);
  }
}

/// No-op stand-in on non-webview targets, so the crate builds everywhere and
/// call sites need no target gating of their own.
#[cfg(not(target_arch = "wasm32"))]
pub fn apply_native_base_theme() {}
