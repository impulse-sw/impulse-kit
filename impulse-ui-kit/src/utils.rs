//! Some utility for UI kit.

/// Utility function to combine classes
pub fn cn(base: &str, additional: Option<&str>) -> String {
  match additional {
    Some(class) => format!("{base} {class}"),
    None => base.to_string(),
  }
}
