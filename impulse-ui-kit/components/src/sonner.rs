#![allow(missing_docs, dead_code)]

// Sonner is a toast notification library wrapper
// This is a simplified implementation that can be expanded with full sonner functionality

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Toaster(
  #[prop(into, optional)] class: String,
  #[prop(optional)] position: Option<ToasterPosition>,
) -> impl IntoView {
  let position = position.unwrap_or(ToasterPosition::BottomRight);

  let position_class = match position {
    ToasterPosition::TopLeft => "top-0 left-0",
    ToasterPosition::TopCenter => "top-0 left-1/2 -translate-x-1/2",
    ToasterPosition::TopRight => "top-0 right-0",
    ToasterPosition::BottomLeft => "bottom-0 left-0",
    ToasterPosition::BottomCenter => "bottom-0 left-1/2 -translate-x-1/2",
    ToasterPosition::BottomRight => "bottom-0 right-0",
  };

  view! {
    <div
      data-slot="toaster"
      class=cn(
        &[
          "fixed z-[100] flex max-h-screen w-full flex-col gap-2 p-4 md:max-w-[420px]",
          position_class,
          class.as_str(),
        ],
      )
    />
  }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ToasterPosition {
  TopLeft,
  TopCenter,
  TopRight,
  BottomLeft,
  BottomCenter,
  BottomRight,
}
