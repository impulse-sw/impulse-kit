//! Viewport size helper for overlay positioning.

use leptos::prelude::*;

/// Returns the current browser viewport `(width, height)`, falling back to
/// unbounded dimensions when unavailable so overlay positioning is left
/// unclamped rather than collapsed to a corner.
///
/// Uses the document element's `clientWidth`/`clientHeight` rather than
/// `window.innerWidth`/`innerHeight`: the latter includes the scrollbar
/// gutter, which overstates the space actually available to a
/// `position: fixed` overlay and lets it get clamped a scrollbar's-width too
/// far toward the edge.
pub(crate) fn viewport_size() -> (f64, f64) {
  let Some(root) = document().document_element() else {
    return (f64::INFINITY, f64::INFINITY);
  };

  (root.client_width() as f64, root.client_height() as f64)
}
