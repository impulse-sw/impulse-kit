#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

/// Maximum width of a [`Container`].
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerSize {
  Sm,
  Md,
  Lg,
  #[default]
  Xl,
  Full,
}

impl ContainerSize {
  pub fn class(&self) -> &'static str {
    match self {
      ContainerSize::Sm => "max-w-3xl",
      ContainerSize::Md => "max-w-5xl",
      ContainerSize::Lg => "max-w-6xl",
      ContainerSize::Xl => "max-w-7xl",
      ContainerSize::Full => "max-w-full",
    }
  }
}

/// A horizontally centered, width-constrained content container with
/// responsive horizontal padding.
#[component]
pub fn Container(
  #[prop(optional)] size: ContainerSize,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      data-slot="container"
      class=cn(&["mx-auto w-full px-4 sm:px-6 lg:px-8", size.class(), class.as_str()])
    >
      {children()}
    </div>
  }
}
