#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use super::{Align, Gap, Justify};

/// Direction of a [`Flex`] container's main axis.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
  #[default]
  Row,
  RowReverse,
  Column,
  ColumnReverse,
}

impl FlexDirection {
  pub fn class(&self) -> &'static str {
    match self {
      FlexDirection::Row => "flex-row",
      FlexDirection::RowReverse => "flex-row-reverse",
      FlexDirection::Column => "flex-col",
      FlexDirection::ColumnReverse => "flex-col-reverse",
    }
  }
}

/// A general-purpose flexbox container.
///
/// Use [`Row`] and [`Column`] for the common horizontal/vertical cases, or
/// reach for `Flex` directly when you need a reversed direction.
#[component]
pub fn Flex(
  #[prop(optional)] direction: FlexDirection,
  #[prop(optional)] gap: Gap,
  #[prop(optional)] align: Align,
  #[prop(optional)] justify: Justify,
  #[prop(optional, default = false)] wrap: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let wrap_class = if wrap { "flex-wrap" } else { "flex-nowrap" };
  view! {
    <div
      data-slot="flex"
      class=cn(
        &[
          "flex",
          direction.class(),
          gap.class(),
          align.class(),
          justify.class(),
          wrap_class,
          class.as_str(),
        ],
      )
    >
      {children()}
    </div>
  }
}

/// A horizontal flexbox container (`flex-row`).
#[component]
pub fn Row(
  #[prop(optional)] gap: Gap,
  #[prop(optional)] align: Align,
  #[prop(optional)] justify: Justify,
  #[prop(optional, default = false)] wrap: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let wrap_class = if wrap { "flex-wrap" } else { "flex-nowrap" };
  view! {
    <div
      data-slot="row"
      class=cn(
        &["flex flex-row", gap.class(), align.class(), justify.class(), wrap_class, class.as_str()],
      )
    >
      {children()}
    </div>
  }
}

/// A vertical flexbox container (`flex-col`).
#[component]
pub fn Column(
  #[prop(optional)] gap: Gap,
  #[prop(optional)] align: Align,
  #[prop(optional)] justify: Justify,
  #[prop(optional, default = false)] wrap: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let wrap_class = if wrap { "flex-wrap" } else { "flex-nowrap" };
  view! {
    <div
      data-slot="column"
      class=cn(
        &["flex flex-col", gap.class(), align.class(), justify.class(), wrap_class, class.as_str()],
      )
    >
      {children()}
    </div>
  }
}
