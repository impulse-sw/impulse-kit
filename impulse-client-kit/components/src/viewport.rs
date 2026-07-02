//! Viewport size helper for overlay positioning.

use leptos::prelude::*;

/// Returns the current browser viewport `(width, height)`, falling back to
/// unbounded dimensions when unavailable so overlay positioning is left
/// unclamped rather than collapsed to a corner.
pub(crate) fn viewport_size() -> (f64, f64) {
  let window = window();

  let width = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(f64::INFINITY);
  let height = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(f64::INFINITY);

  (width, height)
}
