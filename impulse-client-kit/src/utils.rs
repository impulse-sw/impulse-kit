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
