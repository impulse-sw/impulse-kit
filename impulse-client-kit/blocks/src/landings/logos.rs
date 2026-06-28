//! A "trusted by" logo cloud.

use leptos::prelude::*;

/// One logo: a `name` and an optional image `src`.
///
/// When `src` is set the image is shown (muted, grayscale-ish via opacity);
/// otherwise the `name` is rendered as a wordmark — handy before you have real
/// logo assets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Logo {
  /// Company / product name (also the image `alt`).
  pub name: String,
  /// Optional logo image `src`.
  pub src: Option<String>,
}

impl Logo {
  /// A wordmark logo (text only).
  pub fn wordmark(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      src: None,
    }
  }

  /// An image logo.
  pub fn image(name: impl Into<String>, src: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      src: Some(src.into()),
    }
  }
}

/// A centered, muted "trusted by" / "as seen in" logo cloud.
///
/// Logos sit in a wrapping, dimmed row that lifts to full opacity on hover.
/// New in the landings set. Pair it with a short `title` such as
/// "Trusted by teams shipping every day".
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{LogoCloud, Logo};
/// use leptos::prelude::*;
///
/// view! {
///   <LogoCloud
///     title="Trusted by teams that ship daily"
///     logos=vec![Logo::wordmark("Acme"), Logo::image("Globex", "/globex.svg")]
///   />
/// }
/// ```
#[component]
pub fn LogoCloud(
  /// Optional muted line above the logos.
  #[prop(optional, into)]
  title: Option<String>,
  /// The logos.
  logos: Vec<Logo>,
) -> impl IntoView {
  view! {
    <section class="border-b border-border/60">
      <div class="mx-auto max-w-6xl px-4 lg:px-6 py-16">
        {title
          .filter(|s| !s.is_empty())
          .map(|t| {
            view! { <p class="text-center text-sm text-muted-foreground">{t}</p> }
          })}
        <div class="mt-8 flex flex-wrap items-center justify-center gap-x-10 gap-y-6">
          {logos
            .into_iter()
            .map(|logo| {
              match logo.src {
                Some(src) => {
                  view! {
                    <img
                      src=src
                      alt=logo.name
                      class="h-7 w-auto opacity-60 transition-opacity hover:opacity-100"
                    />
                  }
                    .into_any()
                }
                None => {
                  view! {
                    <span class="text-lg font-semibold tracking-tight text-muted-foreground/70 transition-colors hover:text-foreground">
                      {logo.name}
                    </span>
                  }
                    .into_any()
                }
              }
            })
            .collect_view()}
        </div>
      </div>
    </section>
  }
}
