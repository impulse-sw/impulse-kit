//! Layout container primitives.
//!
//! A small set of structural containers built on Tailwind utility classes:
//! flexbox rows and columns ([`Flex`], [`Row`], [`Column`]), CSS grids
//! ([`Grid`], [`GridItem`]), a centered page [`Container`], a full-viewport
//! [`FullScreen`] section, a [`Center`] helper, and a flexible [`Spacer`].
//!
//! Spacing and alignment are expressed through the shared [`Gap`], [`Align`],
//! and [`Justify`] enums so the generated class names stay static and remain
//! discoverable by the Tailwind scanner.

#![allow(missing_docs, dead_code)]

pub mod center;
pub mod container;
pub mod flex;
pub mod fullscreen;
pub mod grid;
pub mod spacer;

pub use center::Center;
pub use container::{Container, ContainerSize};
pub use flex::{Column, Flex, FlexDirection, Row};
pub use fullscreen::FullScreen;
pub use grid::{Grid, GridCols, GridItem, GridSpan};
pub use spacer::Spacer;

/// Spacing between children, mapped onto Tailwind's `gap-*` scale.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Gap {
  None,
  Xs,
  Sm,
  #[default]
  Md,
  Lg,
  Xl,
  Xxl,
}

impl Gap {
  pub fn class(&self) -> &'static str {
    match self {
      Gap::None => "gap-0",
      Gap::Xs => "gap-1",
      Gap::Sm => "gap-2",
      Gap::Md => "gap-4",
      Gap::Lg => "gap-6",
      Gap::Xl => "gap-8",
      Gap::Xxl => "gap-12",
    }
  }
}

/// Cross-axis alignment, mapped onto Tailwind's `items-*` utilities.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
  Start,
  Center,
  End,
  #[default]
  Stretch,
  Baseline,
}

impl Align {
  pub fn class(&self) -> &'static str {
    match self {
      Align::Start => "items-start",
      Align::Center => "items-center",
      Align::End => "items-end",
      Align::Stretch => "items-stretch",
      Align::Baseline => "items-baseline",
    }
  }
}

/// Main-axis distribution, mapped onto Tailwind's `justify-*` utilities.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
  #[default]
  Start,
  Center,
  End,
  Between,
  Around,
  Evenly,
}

impl Justify {
  pub fn class(&self) -> &'static str {
    match self {
      Justify::Start => "justify-start",
      Justify::Center => "justify-center",
      Justify::End => "justify-end",
      Justify::Between => "justify-between",
      Justify::Around => "justify-around",
      Justify::Evenly => "justify-evenly",
    }
  }
}
