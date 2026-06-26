//! Above-the-fold hero section.

use leptos::prelude::*;

use impulse_client_kit_components::button::{Button, ButtonSize, ButtonVariant};

use super::{CtaAction, GridBackdrop};

/// The headline section at the top of a landing page.
///
/// Renders an optional eyebrow badge, a large headline (with an optional
/// gradient-highlighted phrase in the middle), a subtitle, a row of
/// call-to-action buttons and an optional fine-print note. The signature
/// blueprint-grid + glow [`GridBackdrop`] is on by default and can be turned
/// off with `backdrop=false`.
///
/// The headline is assembled as `title` + `highlight` + `title_suffix`, so you
/// can reproduce either landing's style:
///
/// * trailing highlight — `title="Simple, yet powerful "`,
///   `highlight="local CI/CD"`;
/// * mid-sentence highlight — `title="A planner that "`,
///   `highlight="lays out the work itself"`,
///   `title_suffix=" on your schedule"`.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{Hero, CtaAction};
/// use leptos::prelude::*;
///
/// view! {
///   <Hero
///     eyebrow="Auto task planner"
///     title="A planner that "
///     highlight="lays out the work itself"
///     subtitle="Estimates, priorities, deadlines and dependencies become a schedule."
///     actions=vec![CtaAction::primary("Open app", "/app")]
///     note="Email · Google · Yandex ID · Built in Rust"
///   />
/// }
/// ```
#[component]
pub fn Hero(
  /// Small badge above the headline.
  #[prop(optional, into)]
  eyebrow: Option<String>,
  /// Leading (non-highlighted) headline text.
  #[prop(into)]
  title: String,
  /// Optional phrase rendered with the primary→chart-2 gradient.
  #[prop(optional, into)]
  highlight: Option<String>,
  /// Optional headline text after the highlighted phrase.
  #[prop(optional, into)]
  title_suffix: Option<String>,
  /// Supporting paragraph under the headline.
  #[prop(optional, into)]
  subtitle: Option<String>,
  /// Call-to-action buttons, rendered left-to-right.
  #[prop(optional)]
  actions: Vec<CtaAction>,
  /// Fine-print note under the buttons (platforms, tech, …).
  #[prop(optional, into)]
  note: Option<String>,
  /// Show the grid + glow backdrop. Defaults to `true`.
  #[prop(optional, default = true)]
  backdrop: bool,
) -> impl IntoView {
  let eyebrow = eyebrow.filter(|s| !s.is_empty()).map(|e| {
    view! {
      <span class="inline-flex items-center rounded-full border border-border px-3 py-1 text-xs font-medium text-muted-foreground">
        {e}
      </span>
    }
  });
  let highlight = highlight.filter(|s| !s.is_empty()).map(|h| {
    view! {
      <span class="bg-gradient-to-br from-primary to-chart-2 bg-clip-text text-transparent">
        {h}
      </span>
    }
  });
  let subtitle = subtitle
    .filter(|s| !s.is_empty())
    .map(|s| view! { <p class="text-base sm:text-lg md:text-xl text-muted-foreground max-w-2xl text-pretty">{s}</p> });
  let note = note
    .filter(|s| !s.is_empty())
    .map(|n| view! { <p class="text-xs text-muted-foreground/80">{n}</p> });
  let actions = (!actions.is_empty()).then(|| {
    view! {
      <div class="flex flex-col sm:flex-row gap-3 mt-2">
        {actions
          .into_iter()
          .map(|a| {
            let variant = if a.primary { ButtonVariant::Default } else { ButtonVariant::Outline };
            view! {
              <a href=a.href>
                <Button size=ButtonSize::Lg variant=variant class="gap-2">
                  {a.label}
                </Button>
              </a>
            }
          })
          .collect_view()}
      </div>
    }
  });
  view! {
    <section id="top" class="relative isolate overflow-hidden border-b border-border/60">
      {backdrop.then(|| view! { <GridBackdrop /> })}
      <div class="relative mx-auto max-w-6xl px-4 lg:px-6 py-20 md:py-28 lg:py-32">
        <div class="flex flex-col items-center text-center gap-6">
          {eyebrow}
          <h1 class="text-4xl sm:text-5xl md:text-6xl lg:text-7xl font-bold tracking-tight max-w-4xl text-balance">
            {title}
            {highlight}
            {title_suffix}
          </h1>
          {subtitle}
          {actions}
          {note}
        </div>
      </div>
    </section>
  }
}
