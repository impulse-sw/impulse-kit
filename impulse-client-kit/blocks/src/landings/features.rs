//! A grid of feature cards.

use leptos::prelude::*;

use impulse_client_kit_components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};

use super::{HeadingAlign, SectionHeading};

/// One feature card: an optional icon, a title and a description.
pub struct Feature {
  /// Optional leading icon (any view — typically an inline SVG).
  pub icon: Option<AnyView>,
  /// Short feature title.
  pub title: String,
  /// One or two sentences describing the feature.
  pub description: String,
}

impl Feature {
  /// A feature with an icon.
  pub fn new(icon: AnyView, title: impl Into<String>, description: impl Into<String>) -> Self {
    Self { icon: Some(icon), title: title.into(), description: description.into() }
  }

  /// A feature without an icon.
  pub fn text(title: impl Into<String>, description: impl Into<String>) -> Self {
    Self { icon: None, title: title.into(), description: description.into() }
  }
}

/// A responsive grid of feature cards under a [`SectionHeading`].
///
/// Cards lay out one-up on mobile, two-up from `sm` and three-up from `lg`, and
/// gently highlight their border on hover — the treatment shared by both source
/// landings. Each card's icon sits in a rounded, primary-tinted square.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{FeatureGrid, Feature};
/// use leptos::prelude::*;
///
/// view! {
///   <FeatureGrid
///     eyebrow="Features"
///     title="Everything you need"
///     features=vec![
///       Feature::new(view! { <span>"⚙"</span> }.into_any(), "Local pipelines", "Run builds on your own machine."),
///       Feature::text("Signed deploys", "SHA-256 signatures embedded into the archive."),
///     ]
///   />
/// }
/// ```
#[component]
pub fn FeatureGrid(
  /// Eyebrow label for the heading.
  #[prop(optional, into)]
  eyebrow: Option<String>,
  /// Section title.
  #[prop(into)]
  title: String,
  /// Optional section subtitle.
  #[prop(optional, into)]
  subtitle: Option<String>,
  /// The feature cards.
  features: Vec<Feature>,
  /// Anchor `id` for in-page navigation (e.g. `"features"`).
  #[prop(optional, into)]
  id: Option<String>,
) -> impl IntoView {
  view! {
    <section id=id class="border-b border-border/60">
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-20 md:py-24">
        <SectionHeading eyebrow=eyebrow title=title subtitle=subtitle align=HeadingAlign::Center />
        <div class="mt-12 grid gap-4 sm:gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {features
            .into_iter()
            .map(|f| {
              view! {
                <Card class="h-full border-border/60 transition-colors hover:border-primary/40 hover:bg-accent/20">
                  <CardHeader>
                    {f
                      .icon
                      .map(|icon| {
                        view! {
                          <span class="inline-flex h-10 w-10 items-center justify-center rounded-md bg-primary/10 text-primary">
                            {icon}
                          </span>
                        }
                      })}
                    <CardTitle class="mt-3 text-base">{f.title}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <CardDescription class="text-sm leading-relaxed">
                      {f.description}
                    </CardDescription>
                  </CardContent>
                </Card>
              }
            })
            .collect_view()}
        </div>
      </div>
    </section>
  }
}
