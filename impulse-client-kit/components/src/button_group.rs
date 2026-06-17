#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use super::separator::{Separator, SeparatorOrientation};

#[component]
pub fn ButtonGroup(
  #[prop(optional)] orientation: ButtonGroupOrientation,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      role="group"
      data-slot="button-group"
      data-orientation=orientation.as_str()
      class=cn(
        &[
          "flex w-fit items-stretch [&>*]:focus-visible:z-10 [&>*]:focus-visible:relative [&>[data-slot=select-trigger]:not([class*='w-'])]:w-fit [&>input]:flex-1 has-[select[aria-hidden=true]:last-child]:[&>[data-slot=select-trigger]:last-of-type]:rounded-r-md has-[>[data-slot=button-group]]:gap-2",
          orientation.class(),
          class.as_str(),
        ],
      )
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ButtonGroupText(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div class=cn(
      &[
        "bg-muted flex items-center gap-2 rounded-md border px-4 text-sm font-medium shadow-xs [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4",
        class.as_str(),
      ],
    )>{children()}</div>
  }
}

#[component]
pub fn ButtonGroupSeparator(
  #[prop(optional)] orientation: Option<SeparatorOrientation>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let orientation = orientation.unwrap_or(SeparatorOrientation::Vertical);

  view! {
    <Separator
      orientation=orientation
      class=cn(
        &["bg-input relative !m-0 self-stretch data-[orientation=vertical]:h-auto", class.as_str()],
      )
    />
  }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ButtonGroupOrientation {
  #[default]
  Horizontal,
  Vertical,
}

impl ButtonGroupOrientation {
  fn as_str(&self) -> &'static str {
    match self {
      ButtonGroupOrientation::Horizontal => "horizontal",
      ButtonGroupOrientation::Vertical => "vertical",
    }
  }

  fn class(&self) -> &'static str {
    match self {
      Self::Horizontal => {
        "[&>*:not(:first-child)]:rounded-l-none [&>*:not(:first-child)]:border-l-0 [&>*:not(:last-child)]:rounded-r-none"
      }
      Self::Vertical => {
        "flex-col [&>*:not(:first-child)]:rounded-t-none [&>*:not(:first-child)]:border-t-0 [&>*:not(:last-child)]:rounded-b-none"
      }
    }
  }
}
