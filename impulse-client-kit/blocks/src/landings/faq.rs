//! A frequently-asked-questions accordion.

use leptos::prelude::*;

use impulse_client_kit_components::accordion::{Accordion, AccordionContent, AccordionItem, AccordionTrigger, AccordionType};

use super::{HeadingAlign, SectionHeading};

/// One question/answer pair in a [`Faq`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FaqItem {
  /// The question (accordion trigger).
  pub question: String,
  /// The answer (accordion content).
  pub answer: String,
}

impl FaqItem {
  /// Build a Q&A pair from any string-likes.
  pub fn new(question: impl Into<String>, answer: impl Into<String>) -> Self {
    Self { question: question.into(), answer: answer.into() }
  }
}

/// A centered, single-open accordion of questions and answers under a
/// [`SectionHeading`].
///
/// Constrained to a readable measure (`max-w-3xl`) like both source landings.
/// Only one item is open at a time.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::landings::{Faq, FaqItem};
/// use leptos::prelude::*;
///
/// view! {
///   <Faq
///     title="Frequently asked questions"
///     items=vec![
///       FaqItem::new("Which platforms are supported?", "Linux, macOS and Windows."),
///       FaqItem::new("Is it a GitHub Actions replacement?", "More of a complement — export to 9 formats."),
///     ]
///   />
/// }
/// ```
#[component]
pub fn Faq(
  /// Eyebrow label for the heading.
  #[prop(optional, into)]
  eyebrow: Option<String>,
  /// Section title.
  #[prop(into)]
  title: String,
  /// Optional section subtitle.
  #[prop(optional, into)]
  subtitle: Option<String>,
  /// The questions and answers.
  items: Vec<FaqItem>,
  /// Anchor `id` for in-page navigation.
  #[prop(optional, into)]
  id: Option<String>,
) -> impl IntoView {
  view! {
    <section id=id class="border-b border-border/60 bg-muted/30">
      <div class="mx-auto max-w-3xl px-4 lg:px-6 py-20 md:py-24">
        <SectionHeading eyebrow=eyebrow title=title subtitle=subtitle align=HeadingAlign::Center />
        <div class="mt-10">
          <Accordion accordion_type=AccordionType::Single>
            {items
              .into_iter()
              .enumerate()
              .map(|(i, item)| {
                // `AccordionContent` takes a `ChildrenFn` (called more than once),
                // so the answer is stashed in a `Copy` `StoredValue` to keep the
                // closure `Fn`.
                let answer = StoredValue::new(item.answer);
                view! {
                  <AccordionItem value=format!("faq-{i}")>
                    <AccordionTrigger class="text-left text-base font-medium">
                      {item.question}
                    </AccordionTrigger>
                    <AccordionContent class="text-sm text-muted-foreground leading-relaxed">
                      {move || answer.get_value()}
                    </AccordionContent>
                  </AccordionItem>
                }
              })
              .collect_view()}
          </Accordion>
        </div>
      </div>
    </section>
  }
}
