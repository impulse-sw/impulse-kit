//! Base neutrals for the platforms whose palette is a fixed part of the design
//! language: Windows (Fluent), macOS (AppKit) and iOS (UIKit).
//!
//! # Why these are constants rather than something read from the system
//!
//! On Linux the user picks a GTK theme, and on Android the palette is derived
//! from the wallpaper — so on those platforms the neutrals genuinely have to be
//! read at runtime. Windows, macOS and iOS work the other way round: every app
//! draws on the same system neutrals, and the only thing the user changes is
//! light vs dark (plus an accent colour, which this crate deliberately leaves to
//! the app). So "matching the system" here means using the platform's published
//! surface and text colours, which is exactly what these tables hold.
//!
//! Because both schemes are known, each platform publishes **both** — unlike the
//! GTK provider, which can only describe the theme currently in use. The app's
//! own light/dark toggle then works fully, and the webview picks the scheme via
//! the `dark` class.
//!
//! Known gap: the platforms' high-contrast / increased-contrast accessibility
//! modes do change these colours, and that is not reflected here.

use crate::{BaseNeutrals, NativeBaseTheme};

/// One scheme's distinct colours. The design tokens repeat several of these
/// (`muted`/`secondary`/`accent` are one surface; `border`/`input` one line), so
/// naming the *roles* keeps the tables readable and impossible to misalign.
struct Scheme {
  /// Window/page background.
  background: &'static str,
  /// Primary text on `background`.
  foreground: &'static str,
  /// Raised surfaces: cards and popovers.
  card: &'static str,
  /// Menus and flyouts, where the platform separates them from cards.
  popover: &'static str,
  /// Subtle filled surfaces (muted / secondary / accent).
  surface: &'static str,
  /// De-emphasised text.
  dim: &'static str,
  /// Separators and control outlines.
  line: &'static str,
  /// Focus ring — a line strong enough to be seen against `background`.
  ring: &'static str,
}

impl Scheme {
  fn into_neutrals(self) -> BaseNeutrals {
    BaseNeutrals {
      background: self.background.into(),
      foreground: self.foreground.into(),
      card: self.card.into(),
      card_foreground: self.foreground.into(),
      popover: self.popover.into(),
      popover_foreground: self.foreground.into(),
      muted: self.surface.into(),
      muted_foreground: self.dim.into(),
      secondary: self.surface.into(),
      secondary_foreground: self.foreground.into(),
      accent: self.surface.into(),
      accent_foreground: self.foreground.into(),
      border: self.line.into(),
      input: self.line.into(),
      ring: self.ring.into(),
    }
  }
}

fn theme(light: Scheme, dark: Scheme) -> Option<NativeBaseTheme> {
  Some(NativeBaseTheme {
    light: Some(light.into_neutrals()),
    dark: Some(dark.into_neutrals()),
  })
}

/// Fluent / WinUI neutrals — the "solid background" layers, card and flyout
/// fills, and the text colours Windows 11 apps draw with.
#[cfg(target_os = "windows")]
pub(crate) fn capture() -> Option<NativeBaseTheme> {
  theme(
    Scheme {
      background: "#f3f3f3",
      foreground: "#1a1a1a",
      card: "#fbfbfb",
      popover: "#f9f9f9",
      surface: "#ededed",
      dim: "#5d5d5d",
      line: "#e5e5e5",
      ring: "#868686",
    },
    Scheme {
      background: "#202020",
      foreground: "#ffffff",
      card: "#2b2b2b",
      popover: "#2c2c2c",
      surface: "#323232",
      dim: "#c5c5c5",
      line: "#383838",
      ring: "#9a9a9a",
    },
  )
}

/// AppKit's window, control and label colours.
///
/// Note the dark scheme's `card`: on macOS a text/list background is *darker*
/// than the window behind it, the opposite of the Material convention. That is
/// how a Mac window is meant to look, so it is kept.
#[cfg(target_os = "macos")]
pub(crate) fn capture() -> Option<NativeBaseTheme> {
  theme(
    Scheme {
      background: "#ececec",
      foreground: "#000000",
      card: "#ffffff",
      popover: "#f6f6f6",
      surface: "#e0e0e0",
      dim: "#737373",
      line: "#d8d8d8",
      ring: "#a0a0a0",
    },
    Scheme {
      background: "#323232",
      foreground: "#ffffff",
      card: "#1e1e1e",
      popover: "#2d2d2d",
      surface: "#3f3f3f",
      dim: "#a0a0a0",
      line: "#48484a",
      ring: "#7c7c7c",
    },
  )
}

/// UIKit's grouped-content colours — the arrangement iOS uses for exactly this
/// shape of screen: cards on a recessed page. In dark mode the page is true
/// black, which is the platform's own convention on OLED hardware.
#[cfg(target_os = "ios")]
pub(crate) fn capture() -> Option<NativeBaseTheme> {
  theme(
    Scheme {
      background: "#f2f2f7",
      foreground: "#000000",
      card: "#ffffff",
      popover: "#ffffff",
      surface: "#e5e5ea",
      dim: "#8e8e93",
      line: "#c6c6c8",
      ring: "#8e8e93",
    },
    Scheme {
      background: "#000000",
      foreground: "#ffffff",
      card: "#1c1c1e",
      popover: "#1c1c1e",
      surface: "#2c2c2e",
      dim: "#8e8e93",
      line: "#38383a",
      ring: "#8e8e93",
    },
  )
}
