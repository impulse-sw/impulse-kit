#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemVariant {
  #[default]
  Default,
  Outline,
  Muted,
}

impl ItemVariant {
  pub fn class(&self) -> &'static str {
    match self {
      Self::Default => "bg-card border-border/40",
      Self::Outline => "bg-transparent border-border",
      Self::Muted => "bg-muted/50 border-border/40",
    }
  }
}

#[component]
pub fn ItemGroup(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="item-group"
      class=cn(&["flex flex-col divide-y divide-border", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn Item(
  #[prop(optional, into)] class: String,
  #[prop(optional)] variant: ItemVariant,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      data-slot="item"
      data-variant=move || match variant {
        ItemVariant::Default => "default",
        ItemVariant::Outline => "outline",
        ItemVariant::Muted => "muted",
      }

      class=cn(
        &[
          "flex items-start gap-4 rounded-lg border p-4 transition-colors",
          variant.class(),
          class.as_str(),
        ],
      )
    >

      {children()}
    </div>
  }
}

#[component]
pub fn ItemMedia(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="item-media"
      class=cn(&["flex size-10 shrink-0 items-center justify-center", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ItemContent(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="item-content" class=cn(&["flex flex-1 flex-col gap-1", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn ItemHeader(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="item-header"
      class=cn(&["flex items-start justify-between gap-4", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ItemTitle(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="item-title" class=cn(&["text-sm font-semibold leading-none", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn ItemDescription(
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      data-slot="item-description"
      class=cn(&["text-muted-foreground text-sm", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ItemActions(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="item-actions" class=cn(&["flex shrink-0 items-center gap-2", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn ItemFooter(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="item-footer"
      class=cn(&["flex items-center gap-2 border-t pt-3 mt-3", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ItemSeparator(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <hr data-slot="item-separator" class=cn(&["border-border", class.as_str()]) />
  }
}
