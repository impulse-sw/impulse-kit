//! A strip of headline numbers / KPIs.

use leptos::prelude::*;

/// A single statistic: a big `number` and a small `label` under it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stat {
  /// The headline value (e.g. `"9"`, `"YAML"`, `"~12 MB"`).
  pub number: String,
  /// The caption under the number.
  pub label: String,
}

impl Stat {
  /// Build a stat from any string-likes.
  pub fn new(number: impl Into<String>, label: impl Into<String>) -> Self {
    Self {
      number: number.into(),
      label: label.into(),
    }
  }
}

/// A horizontal band of headline numbers, used right under the hero as a
/// trust / at-a-glance bar.
///
/// Lays the stats out two-up on mobile and four-up from `md`, on the muted
/// background both source landings use for this band.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{StatStrip, Stat};
/// use leptos::prelude::*;
///
/// view! {
///   <StatStrip stats=vec![
///     Stat::new("9", "export formats"),
///     Stat::new("2", "execution engines"),
///     Stat::new("YAML", "one config file"),
///     Stat::new("~12 MB", "single static binary"),
///   ] />
/// }
/// ```
#[component]
pub fn StatStrip(
  /// The statistics to display.
  stats: Vec<Stat>,
  /// Tint the big numbers with the primary colour. Defaults to `false`.
  #[prop(optional)]
  accent: bool,
) -> impl IntoView {
  let number_class = if accent {
    "text-2xl md:text-3xl font-bold tracking-tight text-primary"
  } else {
    "text-2xl md:text-3xl font-bold tracking-tight"
  };
  view! {
    <section class="border-b border-border/60 bg-muted/30">
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-8 grid grid-cols-2 md:grid-cols-4 gap-6 text-center">
        {stats
          .into_iter()
          .map(|s| {
            view! {
              <div class="flex flex-col items-center gap-1">
                <span class=number_class>{s.number}</span>
                <span class="text-xs md:text-sm text-muted-foreground max-w-[16rem]">
                  {s.label}
                </span>
              </div>
            }
          })
          .collect_view()}
      </div>
    </section>
  }
}
