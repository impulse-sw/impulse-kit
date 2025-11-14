//! Some utility for UI kit.

/// Utility function to combine classes
pub fn cn(classes: &[impl AsRef<str>]) -> String {
  classes
    .iter()
    .map(|class| class.as_ref())
    .filter(|class| !class.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
}

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

/// Calculates overlay position based on given positions, side and align.
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

  (top, left)
}
