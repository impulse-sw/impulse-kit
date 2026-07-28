//! GTK provider: reads the desktop theme's colours on Linux.
//!
//! Tauri's Linux webview lives in a GTK window, and the desktop theme (Adwaita,
//! Arc, Breeze, …) publishes its palette as GTK named colours — `theme_bg_color`,
//! `theme_fg_color`, `theme_base_color`, `borders` and friends. Those are the
//! real thing: they follow whatever theme the user picked, which is exactly what
//! the webview's CSS system colours fail to expose.
//!
//! A GTK theme describes **one** scheme at a time (a dark theme is simply a
//! theme whose background is dark), so this provider fills in only the scheme
//! the desktop is currently using, detected from the background's luminance. The
//! other scheme is left to the app's own palette — inventing an inverse of the
//! user's theme would look worse than the app's designed colours.
//!
//! # Threading
//!
//! GTK may only be touched from the thread that initialised it, so
//! [`capture`] refuses to run anywhere else ([`gtk::is_initialized_main_thread`]).
//! Call it from the Tauri `setup` hook and cache the result — which is what
//! [`crate::install_native_base_theme`] does — so the IPC command can serve it
//! from a worker thread later.

use gtk::prelude::*;

use crate::{BaseNeutrals, NativeBaseTheme};

/// A colour as linear `0.0..=1.0` components, so we can mix and measure before
/// emitting CSS.
#[derive(Clone, Copy)]
struct Rgb {
  r: f64,
  g: f64,
  b: f64,
}

impl Rgb {
  /// `f64::from` accepts both `f32` and `f64` components, so this compiles
  /// against either gdk RGBA accessor signature.
  fn from_rgba(rgba: gtk::gdk::RGBA) -> Self {
    Self {
      r: f64::from(rgba.red()),
      g: f64::from(rgba.green()),
      b: f64::from(rgba.blue()),
    }
  }

  /// Blends `self` toward `other` by `t` (0 keeps `self`, 1 becomes `other`).
  fn mix(self, other: Rgb, t: f64) -> Self {
    Self {
      r: self.r + (other.r - self.r) * t,
      g: self.g + (other.g - self.g) * t,
      b: self.b + (other.b - self.b) * t,
    }
  }

  /// Perceptual luminance, used to tell a dark theme from a light one.
  fn luminance(self) -> f64 {
    0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
  }

  fn to_hex(self) -> String {
    fn channel(v: f64) -> u8 {
      (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    format!("#{:02x}{:02x}{:02x}", channel(self.r), channel(self.g), channel(self.b))
  }
}

/// Reads the current GTK theme's palette. `None` when GTK isn't initialised, we
/// aren't on its thread, or the theme doesn't define the basic named colours.
pub(crate) fn capture() -> Option<NativeBaseTheme> {
  // Touching GTK off its own thread (or before `gtk_init`) is undefined
  // behaviour; with `panic = "abort"` in release that would take the app down.
  if !gtk::is_initialized_main_thread() {
    return None;
  }

  // Any widget's style context resolves the screen-wide theme providers, which
  // is where the named colours live. A `Label` is the cheapest one and is never
  // realised or shown.
  let style = gtk::Label::new(None).style_context();
  let lookup = |name: &str| style.lookup_color(name).map(Rgb::from_rgba);

  // Every GTK theme defines these two; without them we have no palette to speak
  // of and the app keeps its own.
  let bg = lookup("theme_bg_color")?;
  let fg = lookup("theme_fg_color")?;

  // Entry/list background — what a "card" surface should look like natively.
  let base = lookup("theme_base_color").unwrap_or(bg);
  let text = lookup("theme_text_color").unwrap_or(fg);
  let dim = lookup("insensitive_fg_color").unwrap_or_else(|| fg.mix(bg, 0.45));
  let borders = lookup("borders").unwrap_or_else(|| bg.mix(fg, 0.25));

  let neutrals = BaseNeutrals {
    background: bg.to_hex(),
    foreground: fg.to_hex(),
    card: base.to_hex(),
    card_foreground: text.to_hex(),
    popover: base.to_hex(),
    popover_foreground: text.to_hex(),
    // Subtle surfaces the theme has no explicit colour for: nudged off the
    // background toward the foreground so they read the same in either scheme.
    muted: bg.mix(fg, 0.06).to_hex(),
    muted_foreground: dim.to_hex(),
    secondary: bg.mix(fg, 0.10).to_hex(),
    secondary_foreground: fg.to_hex(),
    accent: bg.mix(fg, 0.10).to_hex(),
    accent_foreground: fg.to_hex(),
    border: borders.to_hex(),
    input: borders.to_hex(),
    // A focus ring needs to beat the border's contrast to be visible.
    ring: borders.mix(fg, 0.40).to_hex(),
  };

  // A GTK theme is one scheme; publish it as the one it actually is.
  if bg.luminance() < 0.5 {
    Some(NativeBaseTheme {
      light: None,
      dark: Some(neutrals),
    })
  } else {
    Some(NativeBaseTheme {
      light: Some(neutrals),
      dark: None,
    })
  }
}
