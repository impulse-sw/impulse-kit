#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Empty(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="empty"
      class=cn(&["flex flex-col items-center justify-center gap-4 py-8", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn EmptyHeader(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="empty-header"
      class=cn(&["flex flex-col items-center gap-2 text-center", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum EmptyMediaVariant {
  #[default]
  Icon,
  Avatar,
}

impl EmptyMediaVariant {
  pub fn class(&self) -> &'static str {
    match self {
      Self::Icon => "text-muted-foreground",
      Self::Avatar => "",
    }
  }
}

#[component]
pub fn EmptyMedia(
  #[prop(optional, into)] class: String,
  #[prop(optional)] variant: EmptyMediaVariant,
  children: Children,
) -> impl IntoView {
  let base_class = match variant {
    EmptyMediaVariant::Icon => "flex size-16 items-center justify-center rounded-full border",
    EmptyMediaVariant::Avatar => "flex size-16 items-center justify-center",
  };

  view! {
    <div
      data-slot="empty-media"
      data-variant=move || match variant {
        EmptyMediaVariant::Icon => "icon",
        EmptyMediaVariant::Avatar => "avatar",
      }

      class=cn(&[base_class, variant.class(), class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn EmptyTitle(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="empty-title" class=cn(&["text-lg font-semibold leading-none", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn EmptyDescription(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="empty-description"
      class=cn(&["text-muted-foreground text-sm text-center", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn EmptyContent(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="empty-content"
      class=cn(&["flex flex-col items-center gap-2 mt-2", class.as_str()])
    >
      {children()}
    </div>
  }
}
