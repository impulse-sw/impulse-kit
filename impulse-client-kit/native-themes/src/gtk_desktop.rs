//! GTK provider: reads the desktop theme's colours on Linux.
//!
//! Tauri's Linux webview lives in a GTK window, and the desktop theme (Adwaita,
//! Arc, Breeze, …) publishes its palette as GTK named colours — `theme_bg_color`,
//! `theme_fg_color`, `theme_base_color`, `borders` and friends. Those are the
//! real thing: they follow whatever theme the user picked, which is exactly what
//! the webview's CSS system colours fail to expose.
//!
//! # Why the handful of `extern "C"` declarations
//!
//! Linking the `gtk` crate would drag GTK 3's development headers into *every*
//! build of every consumer — including headless CI lint runs and the Android
//! build, which have no business needing them. Instead we resolve the four GTK
//! entry points we need from the symbols **already loaded in this process**:
//! a Tauri Linux app links GTK itself, so they're right there, and anywhere else
//! the lookup simply comes up empty and the app keeps its own palette. The C
//! signatures used here are stable GTK 3 public API, and `GdkRGBA` is a plain
//! four-`double` struct.
//!
//! # Threading and scheme
//!
//! GTK may only be touched from the thread that initialised it, so [`capture`]
//! is meant to run from the Tauri `setup` hook (see
//! [`crate::install_native_base_theme`]); the default GDK display being present
//! is what tells us GTK is initialised at all.
//!
//! A GTK theme describes **one** scheme at a time (a dark theme is simply a
//! theme whose background is dark), so this provider fills in only the scheme
//! the desktop is currently using, detected from the background's luminance. The
//! other scheme is left to the app's own palette — inventing an inverse of the
//! user's theme would look worse than the app's designed colours.

use std::ffi::{CString, c_char, c_double, c_int, c_void};

use crate::dynsym;
use crate::{BaseNeutrals, NativeBaseTheme};

/// `GdkRGBA`: four doubles in `0.0..=1.0`. Stable GTK 3 ABI.
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct GdkRgba {
  red: c_double,
  green: c_double,
  blue: c_double,
  alpha: c_double,
}

type GdkDisplayPtr = *mut c_void;
type GtkWidgetPtr = *mut c_void;
type GtkStyleContextPtr = *mut c_void;

type FnDisplayGetDefault = unsafe extern "C" fn() -> GdkDisplayPtr;
type FnLabelNew = unsafe extern "C" fn(*const c_char) -> GtkWidgetPtr;
type FnWidgetGetStyleContext = unsafe extern "C" fn(GtkWidgetPtr) -> GtkStyleContextPtr;
type FnLookupColor = unsafe extern "C" fn(GtkStyleContextPtr, *const c_char, *mut GdkRgba) -> c_int;

/// A colour as `0.0..=1.0` components, so we can mix and measure before
/// emitting CSS.
#[derive(Clone, Copy)]
struct Rgb {
  r: f64,
  g: f64,
  b: f64,
}

impl Rgb {
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

/// Reads the current GTK theme's palette. `None` when GTK isn't loaded or
/// initialised, or the theme doesn't define the basic named colours.
pub(crate) fn capture() -> Option<NativeBaseTheme> {
  // SAFETY (all four): each type alias mirrors the function's documented GTK 3
  // signature.
  let display_get_default: FnDisplayGetDefault = unsafe { dynsym::symbol("gdk_display_get_default") }?;
  let label_new: FnLabelNew = unsafe { dynsym::symbol("gtk_label_new") }?;
  let widget_style_context: FnWidgetGetStyleContext = unsafe { dynsym::symbol("gtk_widget_get_style_context") }?;
  let lookup_color: FnLookupColor = unsafe { dynsym::symbol("gtk_style_context_lookup_color") }?;

  // A default display exists only once GTK has been initialised. Calling into
  // widget code before that is undefined behaviour, and with `panic = "abort"`
  // in release it would take the app down.
  // SAFETY: no arguments, and the pointer is only compared against null.
  if unsafe { display_get_default() }.is_null() {
    return None;
  }

  // Any widget's style context resolves the screen-wide theme providers, which
  // is where the named colours live. A `Label` is the cheapest one; it is never
  // realised or shown. Its initial floating reference is deliberately left
  // alone — one tiny object, once per process.
  // SAFETY: a null label text is valid (an empty label), and the returned
  // widget's style context is owned by the widget.
  let style = unsafe {
    let label = label_new(std::ptr::null());
    if label.is_null() {
      return None;
    }
    widget_style_context(label)
  };
  if style.is_null() {
    return None;
  }

  let lookup = |name: &str| -> Option<Rgb> {
    let cname = CString::new(name).ok()?;
    let mut rgba = GdkRgba::default();
    // SAFETY: `style` is a live GtkStyleContext, `cname` is NUL-terminated, and
    // `rgba` is a valid writable GdkRGBA.
    let found = unsafe { lookup_color(style, cname.as_ptr(), &mut rgba) };
    (found != 0).then_some(Rgb {
      r: rgba.red,
      g: rgba.green,
      b: rgba.blue,
    })
  };

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
  let theme = if bg.luminance() < 0.5 {
    NativeBaseTheme {
      light: None,
      dark: Some(neutrals),
    }
  } else {
    NativeBaseTheme {
      light: Some(neutrals),
      dark: None,
    }
  };
  tracing::info!("captured GTK desktop palette:\n{}", theme.to_css());
  Some(theme)
}

#[cfg(test)]
mod tests {
  use super::*;

  unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
  }

  const RTLD_NOW: c_int = 2;
  const RTLD_GLOBAL: c_int = 0x100;

  type FnInitCheck = unsafe extern "C" fn(*mut c_int, *mut *mut *mut c_char) -> c_int;

  /// Exercises both halves of the contract in one process: with GTK absent the
  /// provider must stay quiet, and with GTK loaded and initialised it must come
  /// back with the live theme's colours.
  ///
  /// Needs a display (run under `xvfb-run` in a headless environment); it skips
  /// itself when GTK or a display isn't there, so it never fails a build for
  /// reasons unrelated to the code.
  #[test]
  fn reads_the_live_gtk_theme() {
    assert!(
      capture().is_none(),
      "must report nothing while GTK isn't loaded in this process"
    );

    let soname = CString::new("libgtk-3.so.0").expect("static string");
    // SAFETY: valid NUL-terminated soname; RTLD_GLOBAL publishes the symbols so
    // the provider's `dlsym(RTLD_DEFAULT, …)` can find them.
    let handle = unsafe { dlopen(soname.as_ptr(), RTLD_NOW | RTLD_GLOBAL) };
    if handle.is_null() {
      eprintln!("skipping: libgtk-3.so.0 not available");
      return;
    }

    // SAFETY: matches `gboolean gtk_init_check(int*, char***)`.
    let Some(init_check): Option<FnInitCheck> = (unsafe { dynsym::symbol("gtk_init_check") }) else {
      eprintln!("skipping: gtk_init_check not found");
      return;
    };
    // SAFETY: GTK 3 accepts NULL for both arguments.
    if unsafe { init_check(std::ptr::null_mut(), std::ptr::null_mut()) } == 0 {
      eprintln!("skipping: no display, GTK could not initialise");
      return;
    }

    let theme = capture().expect("a palette once GTK is initialised");
    let neutrals = theme
      .light
      .as_ref()
      .or(theme.dark.as_ref())
      .expect("exactly one scheme is filled in");

    // Every token must be a fully-formed CSS hex colour.
    for (name, value) in neutrals.vars() {
      assert!(
        value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit()),
        "{name} is not a hex colour: {value}"
      );
    }
    // Exactly one scheme, and a palette you could actually read text on.
    assert!(
      theme.light.is_some() != theme.dark.is_some(),
      "a GTK theme is one scheme, not both"
    );
    assert_ne!(
      neutrals.background, neutrals.foreground,
      "background and foreground must differ"
    );
    eprintln!("captured GTK palette: {}", theme.to_css());
  }
}
