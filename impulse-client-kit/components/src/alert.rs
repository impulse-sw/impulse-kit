#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum AlertVariant {
  #[default]
  Default,
  Destructive,
}

impl AlertVariant {
  fn class(&self) -> &'static str {
    match self {
      AlertVariant::Default => "bg-card text-card-foreground",
      AlertVariant::Destructive => {
        "text-destructive bg-card [&>svg]:text-current *:data-[slot=alert-description]:text-destructive/90"
      }
    }
  }
}

const BASE_CLASSES_CONTAINER: &str = "relative w-full rounded-lg border px-4 py-3 text-sm grid has-[>svg]:grid-cols-[calc(var(--spacing)*4)_1fr] grid-cols-[0_1fr] has-[>svg]:gap-x-3 gap-y-0.5 items-start [&>svg]:size-4 [&>svg]:translate-y-0.5 [&>svg]:text-current";

#[component]
pub fn Alert(
  #[prop(optional)] variant: AlertVariant,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      data-slot="alert"
      role="alert"
      class=cn(&[BASE_CLASSES_CONTAINER, variant.class(), class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn AlertTitle(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="alert-title"
      class=cn(&["col-start-2 line-clamp-1 min-h-4 font-medium tracking-tight", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn AlertDescription(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="alert-description"
      class=cn(
        &[
          "text-muted-foreground col-start-2 grid justify-items-start gap-1 text-sm [&_p]:leading-relaxed",
          class.as_str(),
        ],
      )
    >
      {children()}
    </div>
  }
}
