//! A "before → after" metric comparison grid.

use leptos::prelude::*;

use super::icons::ArrowRight;
use super::{HeadingAlign, SectionHeading};

/// One before/after metric: a `label`, the old (`before`) and new (`after`)
/// values and an optional progress fraction (0–100) for the bar.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Metric {
  /// What is being measured (e.g. "Lead time to release").
  pub label: String,
  /// The value before adoption (rendered struck through).
  pub before: String,
  /// The value after adoption (rendered bold).
  pub after: String,
  /// Optional improvement fraction, 0–100, drawn as a primary bar.
  pub progress: Option<f64>,
}

impl Metric {
  /// A metric without a progress bar.
  pub fn new(label: impl Into<String>, before: impl Into<String>, after: impl Into<String>) -> Self {
    Self { label: label.into(), before: before.into(), after: after.into(), progress: None }
  }

  /// Add the progress fraction (clamped to 0–100).
  pub fn progress(mut self, pct: f64) -> Self {
    self.progress = Some(pct.clamp(0.0, 100.0));
    self
  }
}

/// A grid of "before → after" metric cards under a [`SectionHeading`].
///
/// Each card shows the old value struck through, an arrow, then the new value
/// in bold, with an optional progress bar underneath. Distilled from the DORA /
/// outcomes section of the Деплойер landing and generalised — good for any
/// "we moved these numbers" story.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{MetricComparison, Metric};
/// use leptos::prelude::*;
///
/// view! {
///   <MetricComparison
///     eyebrow="Outcomes"
///     title="Move your team's numbers"
///     metrics=vec![
///       Metric::new("Lead time to release", "weeks", "<1 day").progress(92.0),
///       Metric::new("Change failure rate", "20–30%", "<10%").progress(70.0),
///     ]
///   />
/// }
/// ```
#[component]
pub fn MetricComparison(
  /// Eyebrow label for the heading.
  #[prop(optional, into)]
  eyebrow: Option<String>,
  /// Section title.
  #[prop(into)]
  title: String,
  /// Optional section subtitle.
  #[prop(optional, into)]
  subtitle: Option<String>,
  /// The metrics to compare.
  metrics: Vec<Metric>,
  /// Anchor `id` for in-page navigation.
  #[prop(optional, into)]
  id: Option<String>,
  /// Tint the section with the muted background. Defaults to `false`.
  #[prop(optional)]
  muted: bool,
) -> impl IntoView {
  let section_class = if muted {
    "border-b border-border/60 bg-muted/30"
  } else {
    "border-b border-border/60"
  };
  view! {
    <section id=id class=section_class>
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-20 md:py-24">
        <SectionHeading eyebrow=eyebrow title=title subtitle=subtitle align=HeadingAlign::Center />
        <div class="mt-12 grid gap-6 md:grid-cols-2">
          {metrics
            .into_iter()
            .map(|m| {
              view! {
                <div class="rounded-lg border border-border/60 bg-card/40 p-6 space-y-3">
                  <div class="text-sm font-medium text-muted-foreground">{m.label}</div>
                  <div class="flex items-baseline justify-between gap-4">
                    <div class="text-sm text-muted-foreground line-through">{m.before}</div>
                    <ArrowRight class="h-4 w-4 flex-shrink-0 text-muted-foreground" />
                    <div class="text-2xl md:text-3xl font-bold tracking-tight">{m.after}</div>
                  </div>
                  {m
                    .progress
                    .map(|p| {
                      view! {
                        <div class="h-2 w-full overflow-hidden rounded-full bg-primary/15">
                          <div
                            class="h-full rounded-full bg-primary transition-all"
                            style=format!("width:{p}%")
                          />
                        </div>
                      }
                    })}
                </div>
              }
            })
            .collect_view()}
        </div>
      </div>
    </section>
  }
}
