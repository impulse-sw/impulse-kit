//! A two-column "why us" section: heading on one side, a checklist on the other.

use leptos::prelude::*;

use super::icons::Check;
use super::{HeadingAlign, SectionHeading};

/// One entry in a [`ChecklistSection`]: a bold lead and a muted explanation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChecklistItem {
  /// Short, bold lead-in.
  pub title: String,
  /// One or two sentences of detail.
  pub body: String,
}

impl ChecklistItem {
  /// Build an item from any string-likes.
  pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
    Self { title: title.into(), body: body.into() }
  }
}

/// A two-column section: a left-aligned [`SectionHeading`] beside a vertical
/// checklist of ticked points.
///
/// On large screens the heading sits in the left column and the checklist in
/// the right; they stack on smaller screens. Each point gets a primary-tinted
/// check badge. This is the "Why us" / "How we collaborate" pattern from both
/// source landings.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{ChecklistSection, ChecklistItem};
/// use leptos::prelude::*;
///
/// view! {
///   <ChecklistSection
///     eyebrow="Why"
///     title="Why this, not another script"
///     items=vec![
///       ChecklistItem::new("Interactive mode", "Guided setup, step by step."),
///       ChecklistItem::new("Global registries", "Reuse actions across projects."),
///     ]
///   />
/// }
/// ```
#[component]
pub fn ChecklistSection(
  /// Eyebrow label for the heading.
  #[prop(optional, into)]
  eyebrow: Option<String>,
  /// Section title.
  #[prop(into)]
  title: String,
  /// Optional section subtitle.
  #[prop(optional, into)]
  subtitle: Option<String>,
  /// The checklist points.
  items: Vec<ChecklistItem>,
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
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-20 md:py-24 grid gap-12 lg:grid-cols-2 lg:items-start">
        <div>
          <SectionHeading eyebrow=eyebrow title=title subtitle=subtitle align=HeadingAlign::Start />
        </div>
        <ul class="grid gap-4">
          {items
            .into_iter()
            .map(|item| {
              view! {
                <li class="flex gap-4 rounded-lg border border-border/60 bg-card/40 p-5">
                  <span class="flex-shrink-0 mt-0.5 inline-flex h-7 w-7 items-center justify-center rounded-full bg-primary/15 text-primary">
                    <Check class="h-4 w-4" />
                  </span>
                  <div class="space-y-1">
                    <div class="font-semibold">{item.title}</div>
                    <div class="text-sm text-muted-foreground leading-relaxed">{item.body}</div>
                  </div>
                </li>
              }
            })
            .collect_view()}
        </ul>
      </div>
    </section>
  }
}
