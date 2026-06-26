//! A numbered "how it works" sequence.

use leptos::prelude::*;

use super::{HeadingAlign, SectionHeading, rich};

/// One step in a [`StepList`]: a title and a short body. Steps are numbered
/// automatically by their position.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Step {
  /// Step title.
  pub title: String,
  /// One or two sentences explaining the step.
  pub body: String,
}

impl Step {
  /// Build a step from any string-likes.
  pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
    Self {
      title: title.into(),
      body: body.into(),
    }
  }
}

/// An auto-numbered sequence of steps — a "how it works" section.
///
/// Steps are laid out in an even grid (one column on mobile, then as many
/// columns as there are steps from `md` up), each in a bordered card with a
/// primary-filled number badge. Good for explaining a pipeline or an algorithm
/// in a handful of stages.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{StepList, Step};
/// use leptos::prelude::*;
///
/// view! {
///   <StepList
///     eyebrow="Scheduler"
///     title="How tasks become a schedule"
///     steps=vec![
///       Step::new("Free slots", "Built from your working hours in your timezone."),
///       Step::new("Sort ready tasks", "By priority, deadline and preferred time."),
///       Step::new("Pack the earliest slot", "Each task lands after its blockers finish."),
///     ]
///   />
/// }
/// ```
#[component]
pub fn StepList(
  /// Eyebrow label for the heading.
  #[prop(optional, into)]
  eyebrow: Option<String>,
  /// Section title.
  #[prop(into)]
  title: String,
  /// Optional section subtitle.
  #[prop(optional, into)]
  subtitle: Option<String>,
  /// The ordered steps.
  steps: Vec<Step>,
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
  // Literal classes only — Tailwind's scanner can't see interpolated ones.
  let grid_class = match steps.len() {
    0 | 1 => "mt-12 grid gap-4 md:grid-cols-1",
    2 => "mt-12 grid gap-4 md:grid-cols-2",
    3 => "mt-12 grid gap-4 md:grid-cols-3",
    4 => "mt-12 grid gap-4 sm:grid-cols-2 md:grid-cols-4",
    5 => "mt-12 grid gap-4 sm:grid-cols-2 md:grid-cols-5",
    _ => "mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-3",
  };
  view! {
    <section id=id class=section_class>
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-20 md:py-24">
        <SectionHeading eyebrow=eyebrow title=title subtitle=subtitle align=HeadingAlign::Center />
        <ol class=grid_class>
          {steps
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
              view! {
                <li class="relative rounded-lg border border-border/60 bg-card p-5">
                  <span class="inline-flex h-8 w-8 items-center justify-center rounded-full bg-primary text-primary-foreground text-sm font-bold">
                    {(i + 1).to_string()}
                  </span>
                  <h3 class="mt-3 font-medium text-sm">{rich(&s.title)}</h3>
                  <p class="mt-1 text-sm text-muted-foreground leading-relaxed">{rich(&s.body)}</p>
                </li>
              }
            })
            .collect_view()}
        </ol>
      </div>
    </section>
  }
}
