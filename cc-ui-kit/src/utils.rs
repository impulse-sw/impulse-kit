//! Some utility for UI kit.

/// Utility function to combine classes
pub fn cn(base: &str, additional: Option<&str>) -> String {
  match additional {
    Some(class) => format!("{base} {class}"),
    None => base.to_string(),
  }
}

/// Dark theme colors provider.
pub fn dark_theme() -> thaw::Theme {
  let mut theme = thaw::Theme::dark();
  theme.color.color_brand_background = "#fafafa".to_string();
  theme.color.color_brand_background_hover = "#fafafae6".to_string();
  theme.color.color_brand_background_pressed = "#fafafae6".to_string();
  theme.color.color_neutral_background_1 = "#08080a".to_string();
  theme.color.color_neutral_foreground_on_brand = "#18181b".to_string();
  theme.color.color_neutral_foreground_2_brand_hover = "#fafafa".to_string();
  theme.color.color_neutral_foreground_2_brand_pressed = "#fafafae6".to_string();
  theme.color.color_neutral_foreground_2_brand_selected = "#fafafa".to_string();
  theme
}

/// Light theme colors provider.
pub fn light_theme() -> thaw::Theme {
  let mut theme = thaw::Theme::light();
  theme.color.color_brand_background = "#17171a".to_string();
  theme.color.color_brand_background_hover = "#17171ae6".to_string();
  theme.color.color_brand_background_pressed = "#17171ae6".to_string();
  theme.color.color_neutral_foreground_on_brand = "#fafafa".to_string();
  theme.color.color_neutral_foreground_2_brand_hover = "#18181b".to_string();
  theme.color.color_neutral_foreground_2_brand_pressed = "#18181be6".to_string();
  theme.color.color_neutral_foreground_2_brand_selected = "#18181b".to_string();
  theme
}
