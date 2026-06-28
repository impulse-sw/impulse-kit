//! A slim announcement bar for the very top of the page.

use leptos::prelude::*;

use super::LinkItem;
use super::icons::ArrowRight;

/// A slim, full-width announcement strip — for launches, release notes or
/// promos — meant to sit above the [`Navbar`](super::Navbar).
///
/// Renders a centered message with an optional inline call-to-action link, on a
/// subtle primary-tinted band. New in the landings set.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{AnnouncementBanner, LinkItem};
/// use leptos::prelude::*;
///
/// view! {
///   <AnnouncementBanner
///     text="v4.1 is out — signed deploys are here."
///     link=LinkItem::new("Read the notes", "/changelog")
///   />
/// }
/// ```
#[component]
pub fn AnnouncementBanner(
  /// The announcement text.
  #[prop(into)]
  text: String,
  /// Optional inline call-to-action link after the text.
  #[prop(optional)]
  link: Option<LinkItem>,
  /// Optional leading tag (e.g. "New"). Rendered as a small pill.
  #[prop(optional, into)]
  tag: Option<String>,
) -> impl IntoView {
  view! {
    <div class="w-full border-b border-border/60 bg-primary/10 text-foreground">
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-2 flex items-center justify-center gap-2 text-center text-sm">
        {tag
          .filter(|s| !s.is_empty())
          .map(|t| {
            view! {
              <span class="rounded-full bg-primary px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-primary-foreground">
                {t}
              </span>
            }
          })} <span class="text-muted-foreground">{text}</span>
        {link
          .map(|l| {
            view! {
              <a
                href=l.href
                class="inline-flex items-center gap-1 font-medium text-primary hover:underline"
              >
                {l.label}
                <ArrowRight class="h-3.5 w-3.5" />
              </a>
            }
          })}
      </div>
    </div>
  }
}
