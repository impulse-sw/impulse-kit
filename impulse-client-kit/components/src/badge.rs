#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::attribute_interceptor::AttributeInterceptor;
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
  #[default]
  Default,
  Secondary,
  Destructive,
  Outline,
}

impl BadgeVariant {
  pub fn class(&self) -> &'static str {
    match self {
      Self::Default => "border-transparent bg-primary text-primary-foreground [a&]:hover:bg-primary/90",
      Self::Secondary => "border-transparent bg-secondary text-secondary-foreground [a&]:hover:bg-secondary/90",
      Self::Destructive => {
        "border-transparent bg-destructive text-white [a&]:hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60"
      }
      Self::Outline => "text-foreground [a&]:hover:bg-accent [a&]:hover:text-accent-foreground",
    }
  }
}

const BASE_CLASSES: &str = "inline-flex items-center justify-center rounded-full border px-2 py-0.5 text-xs font-medium w-fit whitespace-nowrap shrink-0 [&>svg]:size-3 gap-1 [&>svg]:pointer-events-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive transition-[color,box-shadow] overflow-hidden";

#[component]
pub fn Badge(
  #[prop(into, optional)] class: String,
  #[prop(optional)] variant: BadgeVariant,
  #[prop(optional, default = false)] as_child: bool,
  children: ChildrenFragmentFn,
) -> impl IntoView {
  let children = StoredValue::new_local(children);

  view! {
    <AttributeInterceptor let:attrs>
      {
        let attrs = StoredValue::new_local(attrs);
        if as_child {
          let mut backup = view! { <span></span> }.into_any();
          let child_views = children.get_value()();
          let mut nodes = child_views.nodes.into_iter().collect::<Vec<_>>();
          let first = nodes.first_mut().unwrap_or(&mut backup);
          let first = std::mem::replace(first, ().into_any());
          let mut first = first.attr("class", cn(&[BASE_CLASSES, variant.class(), class.as_str()]));
          first = first.attr("data-slot", "badge");
          first = first.add_any_attr(attrs.get_value());
          first.into_any()
        } else {

          view! {
            <span
              data-slot="badge"
              class=cn(&[BASE_CLASSES, variant.class(), class.as_str()])
              {..attrs.get_value()}
            >
              {children.get_value()().nodes}
            </span>
          }
            .into_any()
        }
      }
    </AttributeInterceptor>
  }
}
