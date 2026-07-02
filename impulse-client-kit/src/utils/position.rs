//! Calculate position utility.

/// Minimum gap kept between an overlay and the edge of the viewport.
const VIEWPORT_PADDING: f64 = 8.0;

/// Overlay side to show at.
#[derive(Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub enum OverlaySide {
  Top,
  Right,
  Bottom,
  Left,
}

/// Overlay align to show with.
#[derive(Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub enum OverlayAlign {
  Start,
  Center,
  End,
}

/// Calculates overlay position based on given positions, side and align, then
/// clamps the result so the overlay stays fully within the viewport.
#[allow(clippy::too_many_arguments)]
pub fn calculate_position(
  trigger_top: f64,
  trigger_left: f64,
  trigger_width: f64,
  trigger_height: f64,
  content_width: f64,
  content_height: f64,
  side: OverlaySide,
  align: OverlayAlign,
  side_offset: i32,
  viewport_width: f64,
  viewport_height: f64,
) -> (f64, f64) {
  let offset = side_offset as f64;

  let (mut top, mut left) = match side {
    OverlaySide::Top => (trigger_top - content_height - offset, trigger_left),
    OverlaySide::Bottom => (trigger_top + trigger_height + offset, trigger_left),
    OverlaySide::Left => (trigger_top, trigger_left - content_width - offset),
    OverlaySide::Right => (trigger_top, trigger_left + trigger_width + offset),
  };

  match side {
    OverlaySide::Top | OverlaySide::Bottom => {
      left += match align {
        OverlayAlign::Start => 0.0,
        OverlayAlign::Center => (trigger_width - content_width) / 2.0,
        OverlayAlign::End => trigger_width - content_width,
      };
    }
    OverlaySide::Left | OverlaySide::Right => {
      top += match align {
        OverlayAlign::Start => 0.0,
        OverlayAlign::Center => (trigger_height - content_height) / 2.0,
        OverlayAlign::End => trigger_height - content_height,
      };
    }
  }

  clamp_to_viewport(top, left, content_width, content_height, viewport_width, viewport_height)
}

/// Clamps a `top`/`left` position so a `content_width` x `content_height` box
/// stays fully inside the viewport, leaving a small edge padding.
///
/// If the content itself is larger than the viewport (minus padding), it is
/// pinned to the padded edge rather than centered off-screen.
pub fn clamp_to_viewport(
  top: f64,
  left: f64,
  content_width: f64,
  content_height: f64,
  viewport_width: f64,
  viewport_height: f64,
) -> (f64, f64) {
  let max_left = (viewport_width - content_width - VIEWPORT_PADDING).max(VIEWPORT_PADDING);
  let max_top = (viewport_height - content_height - VIEWPORT_PADDING).max(VIEWPORT_PADDING);

  (top.clamp(VIEWPORT_PADDING, max_top), left.clamp(VIEWPORT_PADDING, max_left))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clamps_left_edge() {
    let (top, left) = calculate_position(100.0, -50.0, 30.0, 20.0, 200.0, 100.0, OverlaySide::Bottom, OverlayAlign::Start, 4, 1024.0, 768.0);
    assert_eq!(left, VIEWPORT_PADDING);
    assert_eq!(top, 124.0);
  }

  #[test]
  fn clamps_right_edge() {
    let (_, left) = calculate_position(
      100.0,
      1000.0,
      30.0,
      20.0,
      200.0,
      100.0,
      OverlaySide::Bottom,
      OverlayAlign::Start,
      4,
      1024.0,
      768.0,
    );
    assert_eq!(left, 1024.0 - 200.0 - VIEWPORT_PADDING);
  }

  #[test]
  fn clamps_bottom_edge() {
    let (top, _) = calculate_position(700.0, 100.0, 30.0, 20.0, 100.0, 300.0, OverlaySide::Bottom, OverlayAlign::Start, 4, 1024.0, 768.0);
    assert_eq!(top, 768.0 - 300.0 - VIEWPORT_PADDING);
  }

  #[test]
  fn oversized_content_pins_to_padded_edge() {
    let (top, left) = clamp_to_viewport(-500.0, -500.0, 2000.0, 2000.0, 1024.0, 768.0);
    assert_eq!(top, VIEWPORT_PADDING);
    assert_eq!(left, VIEWPORT_PADDING);
  }

  #[test]
  fn keeps_position_when_within_viewport() {
    let (top, left) = calculate_position(100.0, 100.0, 30.0, 20.0, 50.0, 40.0, OverlaySide::Bottom, OverlayAlign::Start, 4, 1024.0, 768.0);
    assert_eq!(top, 124.0);
    assert_eq!(left, 100.0);
  }
}
