//! Some utility for Client kit.

mod position;

pub use position::{OverlayAlign, OverlaySide, calculate_position, clamp_to_viewport};

/// Utility function to combine classes
pub fn cn(classes: &[impl AsRef<str>]) -> String {
  classes
    .iter()
    .map(|class| class.as_ref())
    .filter(|class| !class.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
}

/// Tailwind classes for the display cutouts and system bars a mobile OS carves
/// out of the viewport.
///
/// Tauri draws the webview edge-to-edge on Android and iOS, so anything pinned
/// to an edge renders *under* the status bar or the home indicator — visible,
/// but with its taps swallowed by the system. `env(safe-area-inset-*)` is what
/// the platform reports for that, and it resolves to 0 in a desktop browser, so
/// padding by it costs nothing there.
///
/// Two things are easy to get wrong, which is why these are constants rather
/// than something each app retypes:
///
/// The page must opt in. Without `viewport-fit=cover` in the viewport meta tag
/// the insets are all 0 and these classes silently do nothing — an app can look
/// correct in a browser and be broken on a phone.
///
/// [`TOP`] belongs on whatever is actually pinned to the top. A `position:
/// sticky` header pins to the viewport no matter what its container is padded
/// by, so padding the container leaves the header sliding back under the status
/// bar on scroll; pad the header itself, and its background still fills the bar
/// while its contents clear it.
pub mod safe_area {
  /// Clears the status bar / notch.
  pub const TOP: &str = "pt-[env(safe-area-inset-top)]";
  /// Clears the home indicator / gesture bar.
  pub const BOTTOM: &str = "pb-[env(safe-area-inset-bottom)]";
  /// Clears rounded corners and landscape cutouts on both sides.
  pub const X: &str = "pl-[env(safe-area-inset-left)] pr-[env(safe-area-inset-right)]";
  /// All four edges, for a container that owns the whole viewport.
  // Written out on one line rather than composed from the three above, or split
  // with a line continuation: Tailwind finds classes by scanning source text, so
  // a class only exists if it appears literally somewhere it looks.
  pub const ALL: &str = "pt-[env(safe-area-inset-top)] pr-[env(safe-area-inset-right)] pb-[env(safe-area-inset-bottom)] pl-[env(safe-area-inset-left)]";
}
