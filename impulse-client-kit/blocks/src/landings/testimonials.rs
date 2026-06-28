//! A grid of customer testimonials / quotes.

use leptos::prelude::*;

use impulse_client_kit_components::card::{Card, CardContent};

use super::{HeadingAlign, SectionHeading};

/// One testimonial: a quote and its attribution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Testimonial {
  /// The quote body (without surrounding quotation marks — the block adds the
  /// decorative mark).
  pub quote: String,
  /// Who said it.
  pub author: String,
  /// Optional role / company line under the author.
  pub role: Option<String>,
  /// Optional avatar image `src`. When omitted, the author's initial is shown.
  pub avatar_src: Option<String>,
}

impl Testimonial {
  /// Build a testimonial with just a quote and an author.
  pub fn new(quote: impl Into<String>, author: impl Into<String>) -> Self {
    Self {
      quote: quote.into(),
      author: author.into(),
      role: None,
      avatar_src: None,
    }
  }

  /// Set the role / company line.
  pub fn role(mut self, role: impl Into<String>) -> Self {
    self.role = Some(role.into());
    self
  }

  /// Set the avatar image `src`.
  pub fn avatar(mut self, src: impl Into<String>) -> Self {
    self.avatar_src = Some(src.into());
    self
  }
}

/// A responsive grid of testimonial cards under a [`SectionHeading`].
///
/// Cards lay out one-up on mobile, two-up from `sm` and three-up from `lg`.
/// Each card shows the quote above an avatar (or a generated initial) with the
/// author and role. New in the landings set — there was no testimonials section
/// in either source page.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{Testimonials, Testimonial};
/// use leptos::prelude::*;
///
/// view! {
///   <Testimonials
///     eyebrow="Loved by teams"
///     title="What people say"
///     items=vec![
///       Testimonial::new("Cut our release time from weeks to hours.", "Alex K.").role("CTO, Acme"),
///     ]
///   />
/// }
/// ```
#[component]
pub fn Testimonials(
  /// Eyebrow label for the heading.
  #[prop(optional, into)]
  eyebrow: Option<String>,
  /// Section title.
  #[prop(into)]
  title: String,
  /// Optional section subtitle.
  #[prop(optional, into)]
  subtitle: Option<String>,
  /// The testimonials.
  items: Vec<Testimonial>,
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
        <div class="mt-12 grid gap-4 sm:gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {items.into_iter().map(|t| view! { <TestimonialCard data=t /> }).collect_view()}
        </div>
      </div>
    </section>
  }
}

#[component]
fn TestimonialCard(data: Testimonial) -> impl IntoView {
  let initial = data
    .author
    .chars()
    .next()
    .map(|c| c.to_uppercase().to_string())
    .unwrap_or_default();
  let avatar = match data.avatar_src {
    Some(src) => view! { <img src=src alt=data.author.clone() class="h-9 w-9 rounded-full object-cover" /> }
    .into_any(),
    None => view! {
      <span class="flex h-9 w-9 items-center justify-center rounded-full bg-primary/10 text-primary text-sm font-semibold">
        {initial}
      </span>
    }
    .into_any(),
  };
  view! {
    <Card class="h-full border-border/60">
      <CardContent class="flex h-full flex-col gap-4 pt-6">
        <p class="text-sm leading-relaxed text-foreground/90 text-pretty">
          <span class="text-primary">"“"</span>
          {data.quote}
          <span class="text-primary">"”"</span>
        </p>
        <div class="mt-auto flex items-center gap-3">
          {avatar} <div class="leading-tight">
            <div class="text-sm font-medium">{data.author}</div>
            {data.role.map(|r| view! { <div class="text-xs text-muted-foreground">{r}</div> })}
          </div>
        </div>
      </CardContent>
    </Card>
  }
}
