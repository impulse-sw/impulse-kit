//! A row of pricing tiers.

use leptos::prelude::*;

use impulse_client_kit_components::button::{Button, ButtonVariant};
use impulse_client_kit_components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};

use super::icons::Check;
use super::{CtaAction, HeadingAlign, SectionHeading};

/// One pricing tier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PricingTier {
  /// Plan name (e.g. "Team").
  pub name: String,
  /// Headline price (e.g. "₽300", "Contact us").
  pub price: String,
  /// Period / qualifier under the price (e.g. "per developer / month").
  pub period: String,
  /// Optional secondary line (e.g. an annual price or seat count).
  pub note: Option<String>,
  /// Bullet list of included features.
  pub features: Vec<String>,
  /// The tier's call-to-action button.
  pub cta: CtaAction,
  /// Visually emphasise this tier (ring + primary border).
  pub highlighted: bool,
}

impl PricingTier {
  /// Build a tier with the required fields; chain the setters for the rest.
  pub fn new(name: impl Into<String>, price: impl Into<String>, period: impl Into<String>, cta: CtaAction) -> Self {
    Self {
      name: name.into(),
      price: price.into(),
      period: period.into(),
      note: None,
      features: Vec::new(),
      cta,
      highlighted: false,
    }
  }

  /// Add the secondary note line.
  pub fn note(mut self, note: impl Into<String>) -> Self {
    self.note = Some(note.into());
    self
  }

  /// Set the feature bullets.
  pub fn features(mut self, features: impl IntoIterator<Item = impl Into<String>>) -> Self {
    self.features = features.into_iter().map(Into::into).collect();
    self
  }

  /// Mark this tier as the highlighted / recommended one.
  pub fn highlighted(mut self) -> Self {
    self.highlighted = true;
    self
  }
}

/// A responsive row of pricing cards under a [`SectionHeading`].
///
/// Cards lay out one-up on mobile, two-up from `md` and four-up from `lg`. A
/// highlighted tier gets a primary border and ring; its CTA renders filled
/// while the rest render outline.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{Pricing, PricingTier, CtaAction};
/// use leptos::prelude::*;
///
/// view! {
///   <Pricing
///     eyebrow="Pricing"
///     title="Plans for developers and teams"
///     tiers=vec![
///       PricingTier::new("Individual", "₽300", "per dev / month", CtaAction::secondary("Request a license", "#contact"))
///         .features(["Full CLI & TUI", "AI init", "All 9 export formats"]),
///       PricingTier::new("Team", "₽100 000", "per year", CtaAction::primary("Request a demo", "#contact"))
///         .note("25 seats")
///         .features(["Everything in Individual", "Signed deploys"])
///         .highlighted(),
///     ]
///   />
/// }
/// ```
#[component]
pub fn Pricing(
  /// Eyebrow label for the heading.
  #[prop(optional, into)]
  eyebrow: Option<String>,
  /// Section title.
  #[prop(into)]
  title: String,
  /// Optional section subtitle.
  #[prop(optional, into)]
  subtitle: Option<String>,
  /// The pricing tiers.
  tiers: Vec<PricingTier>,
  /// Optional fine print under the cards (footnotes, source, …).
  #[prop(optional, into)]
  footnote: Option<String>,
  /// Anchor `id` for in-page navigation.
  #[prop(optional, into)]
  id: Option<String>,
) -> impl IntoView {
  view! {
    <section id=id class="border-b border-border/60">
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-20 md:py-24">
        <SectionHeading eyebrow=eyebrow title=title subtitle=subtitle align=HeadingAlign::Center />
        <div class="mt-12 grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          {tiers.into_iter().map(|t| view! { <PricingCard tier=t /> }).collect_view()}
        </div>
        {footnote
          .filter(|s| !s.is_empty())
          .map(|f| view! { <p class="mt-6 text-center text-xs text-muted-foreground">{f}</p> })}
      </div>
    </section>
  }
}

#[component]
fn PricingCard(tier: PricingTier) -> impl IntoView {
  let card_class = if tier.highlighted {
    "h-full border-primary ring-2 ring-primary/30"
  } else {
    "h-full"
  };
  let cta_variant = if tier.cta.primary {
    ButtonVariant::Default
  } else {
    ButtonVariant::Outline
  };
  view! {
    <Card class=card_class>
      <CardHeader>
        <CardTitle class="text-base font-semibold">{tier.name}</CardTitle>
        <div class="mt-2 flex items-baseline gap-2">
          <span class="text-3xl font-bold tracking-tight">{tier.price}</span>
          <span class="text-sm text-muted-foreground">{tier.period}</span>
        </div>
        {tier.note.map(|n| view! { <CardDescription class="mt-1">{n}</CardDescription> })}
      </CardHeader>
      <CardContent class="flex flex-col gap-4">
        <ul class="space-y-2 text-sm">
          {tier
            .features
            .into_iter()
            .map(|f| {
              view! {
                <li class="flex items-start gap-2">
                  <Check class="h-4 w-4 flex-shrink-0 text-primary" />
                  <span>{f}</span>
                </li>
              }
            })
            .collect_view()}
        </ul>
        <a href=tier.cta.href class="mt-auto">
          <Button variant=cta_variant class="w-full">
            {tier.cta.label}
          </Button>
        </a>
      </CardContent>
    </Card>
  }
}
