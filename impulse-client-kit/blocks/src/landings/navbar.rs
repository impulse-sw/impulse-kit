//! Sticky landing-page navigation bar.

use leptos::prelude::*;

use super::LinkItem;

/// A sticky, translucent top navigation bar with a brand, in-page links and a
/// free-form actions slot.
///
/// The bar sticks to the top with a blurred, semi-transparent background — the
/// same treatment used by both source landings. Put whatever you like in the
/// right-hand actions slot (a theme toggle, a "Sign in" button, a primary CTA)
/// by passing it as children.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{Navbar, LinkItem};
/// use leptos::prelude::*;
///
/// view! {
///   <Navbar
///     brand="TaskBoard"
///     logo_src="/favicon.svg"
///     links=vec![
///       LinkItem::new("Features", "#features"),
///       LinkItem::new("Pricing", "#pricing"),
///     ]
///   >
///     <a href="/app">"Open app"</a>
///   </Navbar>
/// }
/// ```
#[component]
pub fn Navbar(
  /// Brand / product name shown next to the logo.
  #[prop(into)]
  brand: String,
  /// Optional logo image `src`. When omitted, only the brand text is shown.
  #[prop(optional, into)]
  logo_src: Option<String>,
  /// Where the brand links to. Defaults to `#top`.
  #[prop(optional, into)]
  brand_href: Option<String>,
  /// Optional small version badge next to the brand (e.g. `v4.1.0`).
  #[prop(optional, into)]
  version: Option<String>,
  /// In-page navigation links, hidden on small screens.
  #[prop(optional)]
  links: Vec<LinkItem>,
  /// Right-hand actions slot (theme toggle, CTA button, …).
  #[prop(optional)]
  children: Option<Children>,
) -> impl IntoView {
  let brand_href = brand_href.unwrap_or_else(|| "#top".to_string());
  let logo = logo_src.map(|src| view! { <img src=src alt=brand.clone() class="h-8 w-8" /> });
  let version = version.filter(|v| !v.is_empty()).map(|v| {
    view! {
      <span class="ml-1 hidden sm:inline-flex items-center rounded-md bg-secondary px-1.5 py-0.5 font-mono text-[10px] text-secondary-foreground">
        {v}
      </span>
    }
  });
  let nav = (!links.is_empty()).then(|| {
    view! {
      <nav class="hidden md:flex items-center gap-6 text-sm text-muted-foreground">
        {links
          .into_iter()
          .map(|l| {
            view! {
              <a href=l.href class="hover:text-foreground transition-colors">
                {l.label}
              </a>
            }
          })
          .collect_view()}
      </nav>
    }
  });
  view! {
    <header class="sticky top-0 z-50 w-full border-b border-border/60 bg-background/80 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div class="mx-auto max-w-6xl px-4 lg:px-6 flex h-14 items-center justify-between">
        <a href=brand_href class="flex items-center gap-2 group">
          {logo}
          <span class="font-bold text-lg tracking-tight">{brand}</span>
          {version}
        </a>
        {nav}
        <div class="flex items-center gap-2">{children.map(|c| c())}</div>
      </div>
    </header>
  }
}
