#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Separator(
  #[prop(optional)] orientation: SeparatorOrientation,
  #[prop(optional, default = true)] decorative: bool,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let role = if decorative { None } else { Some("separator") };
  let aria_orientation = if decorative { None } else { Some(orientation.as_str()) };

  view! {
    <div
      data-slot="separator"
      data-orientation=orientation.as_str()
      role=role
      aria-orientation=aria_orientation
      class=cn(
        &[
          "bg-border shrink-0 data-[orientation=horizontal]:h-px data-[orientation=horizontal]:w-full data-[orientation=vertical]:h-full data-[orientation=vertical]:w-px",
          class.as_str(),
        ],
      )
    />
  }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum SeparatorOrientation {
  #[default]
  Horizontal,
  Vertical,
}

impl SeparatorOrientation {
  fn as_str(&self) -> &'static str {
    match self {
      SeparatorOrientation::Horizontal => "horizontal",
      SeparatorOrientation::Vertical => "vertical",
    }
  }
}
