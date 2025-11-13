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
