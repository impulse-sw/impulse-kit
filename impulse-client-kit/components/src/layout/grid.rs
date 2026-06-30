#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use super::Gap;

/// Number of equally sized columns in a [`Grid`].
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum GridCols {
  One,
  #[default]
  Two,
  Three,
  Four,
  Five,
  Six,
  Seven,
  Eight,
  Nine,
  Ten,
  Eleven,
  Twelve,
}

impl GridCols {
  pub fn class(&self) -> &'static str {
    match self {
      GridCols::One => "grid-cols-1",
      GridCols::Two => "grid-cols-2",
      GridCols::Three => "grid-cols-3",
      GridCols::Four => "grid-cols-4",
      GridCols::Five => "grid-cols-5",
      GridCols::Six => "grid-cols-6",
      GridCols::Seven => "grid-cols-7",
      GridCols::Eight => "grid-cols-8",
      GridCols::Nine => "grid-cols-9",
      GridCols::Ten => "grid-cols-10",
      GridCols::Eleven => "grid-cols-11",
      GridCols::Twelve => "grid-cols-12",
    }
  }
}

/// How many columns a [`GridItem`] spans.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum GridSpan {
  #[default]
  One,
  Two,
  Three,
  Four,
  Five,
  Six,
  Seven,
  Eight,
  Nine,
  Ten,
  Eleven,
  Twelve,
  Full,
}

impl GridSpan {
  pub fn class(&self) -> &'static str {
    match self {
      GridSpan::One => "col-span-1",
      GridSpan::Two => "col-span-2",
      GridSpan::Three => "col-span-3",
      GridSpan::Four => "col-span-4",
      GridSpan::Five => "col-span-5",
      GridSpan::Six => "col-span-6",
      GridSpan::Seven => "col-span-7",
      GridSpan::Eight => "col-span-8",
      GridSpan::Nine => "col-span-9",
      GridSpan::Ten => "col-span-10",
      GridSpan::Eleven => "col-span-11",
      GridSpan::Twelve => "col-span-12",
      GridSpan::Full => "col-span-full",
    }
  }
}

/// A CSS grid container with a fixed number of equal-width columns.
///
/// For responsive column counts, leave [`GridCols`] at its default and pass
/// responsive variants (e.g. `"sm:grid-cols-2 lg:grid-cols-4"`) through
/// `class`.
#[component]
pub fn Grid(
  #[prop(optional)] cols: GridCols,
  #[prop(optional)] gap: Gap,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div data-slot="grid" class=cn(&["grid", cols.class(), gap.class(), class.as_str()])>
      {children()}
    </div>
  }
}

/// A child of [`Grid`] that can span multiple columns.
#[component]
pub fn GridItem(
  #[prop(optional)] span: GridSpan,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div data-slot="grid-item" class=cn(&[span.class(), class.as_str()])>
      {children()}
    </div>
  }
}
