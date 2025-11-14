#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[derive(Default)]
pub enum ButtonVariant {
  #[default]
  Default,
  Destructive,
  Outline,
  Secondary,
  Ghost,
  Link,
}

#[derive(Default)]
pub enum ButtonSize {
  #[default]
  Default,
  Sm,
  Lg,
  Icon,
  IconSm,
  IconLg,
}

const BASE_CLASSES: &str = "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive";

#[component]
pub fn Button(
  #[prop(optional)] variant: ButtonVariant,
  #[prop(optional)] size: ButtonSize,
  #[prop(into, optional)] class: String,
  #[prop(optional)] node_ref: NodeRef<leptos::html::Button>,
  children: Children,
) -> impl IntoView {
  let variant_str = match variant {
        ButtonVariant::Default => "bg-primary text-primary-foreground hover:bg-primary/90",
        ButtonVariant::Destructive => "bg-destructive text-white hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60",
        ButtonVariant::Outline => "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:bg-input/30 dark:border-input dark:hover:bg-input/50",
        ButtonVariant::Secondary => "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ButtonVariant::Ghost => "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
        ButtonVariant::Link => "text-primary underline-offset-4 hover:underline",
    }.to_string();

  let size_str = match size {
    ButtonSize::Default => "h-9 px-4 py-2 has-[>svg]:px-3",
    ButtonSize::Sm => "h-8 rounded-md gap-1.5 px-3 has-[>svg]:px-2.5",
    ButtonSize::Lg => "h-10 rounded-md px-6 has-[>svg]:px-4",
    ButtonSize::Icon => "size-9",
    ButtonSize::IconSm => "size-8",
    ButtonSize::IconLg => "size-10",
  }
  .to_string();

  let classes = cn(&[BASE_CLASSES.to_string(), variant_str, size_str, class]);

  view! {
    <button attr:data-slot="button" class=classes node_ref=node_ref>
      {children()}
    </button>
  }
}
