#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

#[derive(Copy, Clone, PartialEq, Default)]
pub enum SpinnerSize {
  Sm,
  #[default]
  Default,
  Lg,
}

impl SpinnerSize {
  fn class(&self) -> &'static str {
    match self {
      Self::Sm => "h-4 w-4 border-2",
      Self::Default => "h-8 w-8 border-2",
      Self::Lg => "h-12 w-12 border-3",
    }
  }
}

#[component]
pub fn Spinner(#[prop(optional)] size: SpinnerSize, #[prop(into, optional)] class: String) -> impl IntoView {
  view! {
    <div
      data-slot="spinner"
      role="status"
      aria-label="Loading"
      class=cn(
        &[
          "animate-spin rounded-full border-solid border-current border-r-transparent",
          size.class(),
          class.as_str(),
        ],
      )
    >
      <span class="sr-only">"Loading..."</span>
    </div>
  }
}
