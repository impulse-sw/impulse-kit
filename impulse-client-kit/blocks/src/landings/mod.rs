//! Ready-made **landing-page blocks** for the Impulse Client Kit.
//!
//! Where [`crate::charts`] and [`crate::graph`] are data-visualisation widgets,
//! this module ships the marketing-page sections you reach for when building a
//! product landing: a [`Navbar`], a [`Hero`], [`FeatureGrid`], [`Pricing`],
//! [`Faq`], a [`Footer`] and more. They were distilled from two real landing
//! pages built on the kit (TaskBoard and Деплойер) and generalised into
//! data-driven, theme-aware blocks.
//!
//! # Design
//!
//! * **Data-driven.** Every block takes plain data (`Vec<Feature>`,
//!   `Vec<PricingTier>`, …) rather than a wall of markup, so a whole section is
//!   a single `view!` node fed by a `vec!`.
//! * **Theme-aware.** Colours come from the kit's CSS variables
//!   (`--primary`, `--foreground`, `--muted-foreground`, `--border`, …), so the
//!   blocks follow light/dark mode and any palette override automatically.
//! * **Self-contained.** The signature "blueprint grid + glow" backdrop from
//!   both source landings is shipped as [`GridBackdrop`], rendered with inline
//!   styles built from the same CSS variables — no app-level `@layer utilities`
//!   required.
//!
//! ```rust,ignore
//! use impulse_client_kit_blocks::landings::*;
//! use leptos::prelude::*;
//!
//! view! {
//!   <Hero
//!     eyebrow="Local CI/CD"
//!     title="Simple, yet powerful"
//!     highlight="local CI/CD"
//!     subtitle="One YAML replaces five to seven scattered configs."
//!     actions=vec![
//!       CtaAction::primary("Request a demo", "#contact"),
//!       CtaAction::secondary("See features", "#features"),
//!     ]
//!   />
//! }
//! ```

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

mod icons;

pub mod banner;
pub mod checklist;
pub mod cta;
pub mod faq;
pub mod features;
pub mod footer;
pub mod hero;
pub mod logos;
pub mod metrics;
pub mod navbar;
pub mod pills;
pub mod pricing;
pub mod stats;
pub mod steps;
pub mod testimonials;

pub use banner::*;
pub use checklist::*;
pub use cta::*;
pub use faq::*;
pub use features::*;
pub use footer::*;
pub use hero::*;
pub use logos::*;
pub use metrics::*;
pub use navbar::*;
pub use pills::*;
pub use pricing::*;
pub use stats::*;
pub use steps::*;
pub use testimonials::*;

/// A plain navigational link: a label and an `href`.
///
/// Used by [`Navbar`] and [`Footer`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkItem {
  /// Visible text.
  pub label: String,
  /// Destination URL or `#anchor`.
  pub href: String,
}

impl LinkItem {
  /// Build a link from any string-likes.
  pub fn new(label: impl Into<String>, href: impl Into<String>) -> Self {
    Self { label: label.into(), href: href.into() }
  }
}

/// A call-to-action button: a label, an `href` and a visual-emphasis flag.
///
/// `primary` actions render as a filled [`Button`](impulse_client_kit_components::button::Button),
/// non-primary ones as an outline button.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CtaAction {
  /// Visible text.
  pub label: String,
  /// Destination URL or `#anchor`.
  pub href: String,
  /// Render filled (`true`) or outline (`false`).
  pub primary: bool,
}

impl CtaAction {
  /// A filled, primary call-to-action.
  pub fn primary(label: impl Into<String>, href: impl Into<String>) -> Self {
    Self { label: label.into(), href: href.into(), primary: true }
  }

  /// An outline, secondary call-to-action.
  pub fn secondary(label: impl Into<String>, href: impl Into<String>) -> Self {
    Self { label: label.into(), href: href.into(), primary: false }
  }
}

/// Horizontal alignment for a [`SectionHeading`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HeadingAlign {
  /// Centered — the default for full-width sections.
  #[default]
  Center,
  /// Left-aligned — for split / two-column sections.
  Start,
}

/// The signature blueprint-grid background.
const GRID_STYLE: &str = "background-image:linear-gradient(to right, color-mix(in oklch, var(--foreground) 6%, transparent) 1px, transparent 1px),linear-gradient(to bottom, color-mix(in oklch, var(--foreground) 6%, transparent) 1px, transparent 1px);background-size:32px 32px;-webkit-mask-image:radial-gradient(ellipse 80% 60% at 50% 0%, black, transparent 80%);mask-image:radial-gradient(ellipse 80% 60% at 50% 0%, black, transparent 80%);";

/// The soft primary glow that sits over the grid.
const GLOW_STYLE: &str = "background:radial-gradient(60% 60% at 50% 0%, color-mix(in oklch, var(--primary) 25%, transparent), transparent 70%);";

/// The decorative "blueprint grid + glow" backdrop used at the top of both
/// source landings.
///
/// Render it as the first child of a `relative isolate overflow-hidden`
/// container; it positions itself absolutely and is purely decorative
/// (`pointer-events-none`). Both layers are optional so you can keep just the
/// grid, just the glow, or both.
///
/// [`Hero`] and [`CallToAction`] embed this automatically via their `backdrop`
/// prop; use it directly to decorate your own sections.
#[component]
pub fn GridBackdrop(
  /// Draw the faint blueprint grid. Defaults to `true`.
  #[prop(optional, default = true)]
  grid: bool,
  /// Draw the soft primary glow. Defaults to `true`.
  #[prop(optional, default = true)]
  glow: bool,
) -> impl IntoView {
  view! {
    {grid.then(|| view! { <div class="absolute inset-0 pointer-events-none" style=GRID_STYLE /> })}
    {glow.then(|| view! { <div class="absolute inset-0 pointer-events-none" style=GLOW_STYLE /> })}
  }
}

/// A section header: a small coloured eyebrow, a bold title and a muted
/// subtitle, centered or left-aligned.
///
/// Every section block ([`FeatureGrid`], [`Pricing`], [`Faq`], …) takes the
/// same three `eyebrow` / `title` / `subtitle` props and renders one of these
/// internally, so the typography stays consistent across the whole page.
#[component]
pub fn SectionHeading(
  /// Small uppercase-ish label above the title (e.g. `Some("Features")`).
  ///
  /// Required as an `Option` rather than an optional prop so the section blocks
  /// can forward their own optional eyebrow straight through; pass `None` to
  /// omit it.
  eyebrow: Option<String>,
  /// The section title.
  #[prop(into)]
  title: String,
  /// A muted line under the title, or `None` to omit it.
  subtitle: Option<String>,
  /// Center (default) or left-align the block.
  #[prop(optional)]
  align: HeadingAlign,
  /// Extra classes for the wrapper.
  #[prop(optional, into)]
  class: String,
) -> impl IntoView {
  let wrap = match align {
    HeadingAlign::Center => "max-w-2xl mx-auto text-center",
    HeadingAlign::Start => "max-w-2xl",
  };
  let subtitle = subtitle.filter(|s| !s.is_empty());
  let eyebrow = eyebrow.filter(|s| !s.is_empty());
  view! {
    <div class=cn(&[wrap, class.as_str()])>
      {eyebrow.map(|e| view! { <p class="text-sm font-medium text-primary">{e}</p> })}
      <h2 class="mt-2 text-3xl md:text-4xl font-bold tracking-tight text-balance">{title}</h2>
      {subtitle
        .map(|s| view! { <p class="mt-4 text-muted-foreground text-pretty">{s}</p> })}
    </div>
  }
}
