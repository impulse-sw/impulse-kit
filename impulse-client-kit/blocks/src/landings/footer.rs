//! The page footer.

use leptos::prelude::*;

use impulse_client_kit_components::separator::{Separator, SeparatorOrientation};

use super::LinkItem;

/// A titled column of links in the [`Footer`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FooterColumn {
  /// Column heading.
  pub title: String,
  /// Links in the column.
  pub links: Vec<LinkItem>,
}

impl FooterColumn {
  /// Build a column from a title and a list of links.
  pub fn new(title: impl Into<String>, links: impl IntoIterator<Item = LinkItem>) -> Self {
    Self {
      title: title.into(),
      links: links.into_iter().collect(),
    }
  }
}

/// A page footer: a brand block with a tagline, any number of link columns and
/// a thin bottom bar for legal / colophon lines.
///
/// The brand block spans the first column and the link columns fill the rest,
/// matching the source landings. When there are no `notes`, the bottom bar and
/// its separator are omitted.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{Footer, FooterColumn, LinkItem};
/// use leptos::prelude::*;
///
/// view! {
///   <Footer
///     brand="Деплойер"
///     logo_src="/depl-logo-2.svg"
///     tagline="Simple, yet powerful local CI/CD."
///     columns=vec![
///       FooterColumn::new("Business", [LinkItem::new("Pricing", "#pricing")]),
///     ]
///     notes=vec!["© Verbal Automation Systems LLC".into()]
///   />
/// }
/// ```
#[component]
pub fn Footer(
  /// Brand / product name.
  #[prop(into)]
  brand: String,
  /// Optional logo image `src`.
  #[prop(optional, into)]
  logo_src: Option<String>,
  /// Optional tagline under the brand.
  #[prop(optional, into)]
  tagline: Option<String>,
  /// Link columns.
  #[prop(optional)]
  columns: Vec<FooterColumn>,
  /// Bottom-bar lines (copyright, colophon, …).
  #[prop(optional)]
  notes: Vec<String>,
) -> impl IntoView {
  let logo = logo_src.map(|src| view! { <img src=src alt=brand.clone() class="h-8 w-8" /> });
  let tagline = tagline
    .filter(|s| !s.is_empty())
    .map(|t| view! { <p class="text-muted-foreground">{t}</p> });
  let columns_view = columns
    .into_iter()
    .map(|c| {
      view! {
        <div class="space-y-2">
          <div class="font-semibold">{c.title}</div>
          <ul class="space-y-1.5 text-muted-foreground">
            {c
              .links
              .into_iter()
              .map(|l| {
                view! {
                  <li>
                    <a href=l.href class="hover:text-foreground transition-colors">
                      {l.label}
                    </a>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </div>
      }
    })
    .collect_view();
  let bottom = (!notes.is_empty()).then(|| {
    view! {
      <Separator orientation=SeparatorOrientation::Horizontal />
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-4 flex flex-col sm:flex-row items-center justify-between gap-2 text-xs text-muted-foreground">
        {notes.into_iter().map(|n| view! { <span>{n}</span> }).collect_view()}
      </div>
    }
  });
  view! {
    <footer class="border-t border-border/60 bg-background">
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-10 grid gap-6 md:grid-cols-3 lg:grid-cols-4 text-sm">
        <div class="space-y-2 md:col-span-1 lg:col-span-1">
          <div class="flex items-center gap-2">{logo} <span class="font-bold">{brand}</span></div>
          {tagline}
        </div>
        {columns_view}
      </div>
      {bottom}
    </footer>
  }
}
