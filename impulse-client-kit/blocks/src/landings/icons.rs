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
