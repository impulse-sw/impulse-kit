//! Inline SVG icons shared by the landing blocks.
//!
//! Kept in a private module (not re-exported) so the `#[component]`-generated
//! symbols don't collide across blocks. Each icon takes a `class` so call sites
//! control sizing and colour.

use leptos::prelude::*;

/// A rightward arrow — used in CTAs and "before → after" rows.
#[component]
pub(crate) fn ArrowRight(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class=class
    >
      <path d="M5 12h14" />
      <path d="m12 5 7 7-7 7" />
    </svg>
  }
}

/// A four-point spark / asterisk — used in hero eyebrows.
#[component]
pub(crate) fn Spark(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class=class
    >
      <path d="M12 3v18" />
      <path d="M3 12h18" />
      <path d="m5.6 5.6 12.8 12.8" />
      <path d="m18.4 5.6-12.8 12.8" />
    </svg>
  }
}

/// A check mark — used in checklists and pricing feature lists.
#[component]
pub(crate) fn Check(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      class=class
    >
      <path d="M20 6 9 17l-5-5" />
    </svg>
  }
}
