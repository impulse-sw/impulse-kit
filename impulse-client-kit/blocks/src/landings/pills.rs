//! A centered row of "pill" chips — a tech-stack / badges strip.

use leptos::prelude::*;

/// One pill: a bold `name` and an optional muted `note` shown beside it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pill {
  /// Bold label (e.g. "Rust").
  pub name: String,
  /// Optional muted detail (e.g. "One language end to end").
  pub note: Option<String>,
}

impl Pill {
  /// A bare pill.
  pub fn new(name: impl Into<String>) -> Self {
    Self { name: name.into(), note: None }
  }

  /// A pill with a trailing note (also used as the `title` tooltip).
  pub fn noted(name: impl Into<String>, note: impl Into<String>) -> Self {
    Self { name: name.into(), note: Some(note.into()) }
  }
}

/// A centered, wrapping row of rounded pills under a small label — the
/// "Built with" / tech-stack strip.
///
/// Each pill shows its name; a note, when present, appears beside it from `sm`
/// up and doubles as the hover tooltip.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{PillRow, Pill};
/// use leptos::prelude::*;
///
/// view! {
///   <PillRow
///     label="Stack"
///     pills=vec![
///       Pill::noted("Rust", "One language end to end"),
///       Pill::noted("Leptos 0.8", "SSR landing + CSR app"),
///       Pill::new("SQLite"),
///     ]
///   />
/// }
/// ```
#[component]
pub fn PillRow(
  /// Optional small uppercase label above the row.
  #[prop(optional, into)]
  label: Option<String>,
  /// The pills.
  pills: Vec<Pill>,
) -> impl IntoView {
  view! {
    <section class="border-b border-border/60">
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-16">
        {label
          .filter(|s| !s.is_empty())
          .map(|l| {
            view! {
              <p class="text-center text-xs uppercase tracking-wider text-muted-foreground">{l}</p>
            }
          })}
        <div class="mt-6 flex flex-wrap items-center justify-center gap-3">
          {pills
            .into_iter()
            .map(|p| {
              let title = p.note.clone().unwrap_or_default();
              view! {
                <span
                  class="inline-flex items-center gap-2 rounded-full border border-border/60 bg-card px-4 py-1.5 text-sm"
                  title=title
                >
                  <span class="font-medium">{p.name}</span>
                  {p
                    .note
                    .map(|n| view! { <span class="hidden text-muted-foreground sm:inline">{n}</span> })}
                </span>
              }
            })
            .collect_view()}
        </div>
      </div>
    </section>
  }
}
