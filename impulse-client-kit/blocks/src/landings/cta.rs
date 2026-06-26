//! A closing call-to-action band.

use leptos::prelude::*;

use impulse_client_kit_components::button::{Button, ButtonSize, ButtonVariant};

use super::icons::ArrowRight;
use super::{CtaAction, GridBackdrop};

/// A centered, full-width call-to-action band — the closing "ready to start?"
/// section.
///
/// Renders a bold prompt, an optional supporting line and a row of buttons over
/// the soft primary glow. Set `grid=true` to add the blueprint grid as well, to
/// echo the hero.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{CallToAction, CtaAction};
/// use leptos::prelude::*;
///
/// view! {
///   <CallToAction
///     title="Ready to stop sorting tasks by hand?"
///     subtitle="Sign in and let the scheduler build your plan."
///     actions=vec![CtaAction::primary("Open app", "/app")]
///   />
/// }
/// ```
#[component]
pub fn CallToAction(
  /// The headline prompt.
  #[prop(into)]
  title: String,
  /// Optional supporting line under the title.
  #[prop(optional, into)]
  subtitle: Option<String>,
  /// Call-to-action buttons.
  #[prop(optional)]
  actions: Vec<CtaAction>,
  /// Anchor `id` for in-page navigation.
  #[prop(optional, into)]
  id: Option<String>,
  /// Also draw the blueprint grid behind the glow. Defaults to `false`.
  #[prop(optional)]
  grid: bool,
) -> impl IntoView {
  let subtitle = subtitle
    .filter(|s| !s.is_empty())
    .map(|s| view! { <p class="mt-4 text-muted-foreground text-pretty">{s}</p> });
  let actions = (!actions.is_empty()).then(|| {
    view! {
      <div class="mt-8 flex flex-col sm:flex-row justify-center gap-3">
        {actions
          .into_iter()
          .map(|a| {
            let variant = if a.primary { ButtonVariant::Default } else { ButtonVariant::Outline };
            let arrow = a.primary.then(|| view! { <ArrowRight class="h-4 w-4" /> });
            view! {
              <a href=a.href>
                <Button size=ButtonSize::Lg variant=variant class="gap-2">
                  {a.label}
                  {arrow}
                </Button>
              </a>
            }
          })
          .collect_view()}
      </div>
    }
  });
  view! {
    <section id=id class="relative isolate overflow-hidden">
      <GridBackdrop grid=grid glow=true />
      <div class="relative mx-auto max-w-3xl px-4 lg:px-6 py-20 md:py-24 text-center">
        <h2 class="text-3xl md:text-4xl font-bold tracking-tight text-balance">{title}</h2>
        {subtitle}
        {actions}
      </div>
    </section>
  }
}
