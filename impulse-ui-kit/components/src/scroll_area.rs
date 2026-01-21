#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn ScrollArea(
  #[prop(into, optional)] class: String,
  #[prop(optional)] orientation: Option<ScrollAreaOrientation>,
  children: Children,
) -> impl IntoView {
  let orientation = orientation.unwrap_or(ScrollAreaOrientation::Vertical);

  let overflow_class = match orientation {
    ScrollAreaOrientation::Vertical => "overflow-y-auto",
    ScrollAreaOrientation::Horizontal => "overflow-x-auto",
    ScrollAreaOrientation::Both => "overflow-auto",
  };

  view! {
    <div data-slot="scroll-area" class=cn(&["relative overflow-hidden", class.as_str()])>
      <div class=cn(&["h-full w-full rounded-[inherit]", overflow_class])>{children()}</div>
    </div>
  }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ScrollAreaOrientation {
  Vertical,
  Horizontal,
  Both,
}

#[component]
pub fn ScrollBar(
  #[prop(into, optional)] class: String,
  #[prop(optional)] orientation: Option<ScrollAreaOrientation>,
) -> impl IntoView {
  let orientation = orientation.unwrap_or(ScrollAreaOrientation::Vertical);

  let orientation_class = match orientation {
    ScrollAreaOrientation::Vertical => "h-full w-2.5 border-l border-l-transparent p-[1px]",
    ScrollAreaOrientation::Horizontal => "h-2.5 w-full border-t border-t-transparent p-[1px]",
    ScrollAreaOrientation::Both => "h-full w-full",
  };

  view! {
    <div
      data-slot="scroll-bar"
      data-orientation=match orientation {
        ScrollAreaOrientation::Vertical => "vertical",
        ScrollAreaOrientation::Horizontal => "horizontal",
        ScrollAreaOrientation::Both => "both",
      }
      class=cn(
        &["flex touch-none select-none transition-colors", orientation_class, class.as_str()],
      )
    >
      <div class="bg-border relative flex-1 rounded-full" />
    </div>
  }
}
