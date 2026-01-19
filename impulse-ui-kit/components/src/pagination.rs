#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Pagination(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <nav
      data-slot="pagination"
      role="navigation"
      aria-label="pagination"
      class=cn(&["mx-auto flex w-full justify-center", class.as_str()])
    >
      {children()}
    </nav>
  }
}

#[component]
pub fn PaginationContent(
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <ul data-slot="pagination-content" class=cn(&["flex flex-row items-center gap-1", class.as_str()])>
      {children()}
    </ul>
  }
}

#[component]
pub fn PaginationItem(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <li data-slot="pagination-item" class=class>
      {children()}
    </li>
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaginationLinkVariant {
  Default,
  Active,
}

impl PaginationLinkVariant {
  pub fn class(&self) -> &'static str {
    match self {
      Self::Default => "hover:bg-accent hover:text-accent-foreground",
      Self::Active => "bg-accent text-accent-foreground hover:bg-accent/90",
    }
  }
}

#[component]
pub fn PaginationLink(
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] href: String,
  #[prop(optional)] variant: PaginationLinkVariant,
  #[prop(optional)] is_active: bool,
  #[prop(optional)] disabled: bool,
  children: Children,
) -> impl IntoView {
  let effective_variant = if is_active {
    PaginationLinkVariant::Active
  } else {
    variant
  };

  view! {
    <a
      data-slot="pagination-link"
      href=href
      aria-current=move || if is_active { "page" } else { "" }
      aria-disabled=disabled
      class=cn(
        &[
          "focus-visible:ring-ring/50 inline-flex size-9 items-center justify-center gap-1 whitespace-nowrap rounded-md text-sm font-medium transition-colors outline-none focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50",
          effective_variant.class(),
          class.as_str(),
        ],
      )
    >

      {children()}
    </a>
  }
}

#[component]
pub fn PaginationPrevious(
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] href: String,
  #[prop(optional)] disabled: bool,
) -> impl IntoView {
  view! {
    <PaginationLink class=cn(&["gap-1 pl-2.5", class.as_str()]) href=href disabled=disabled>
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="size-4"
      >
        <path d="m15 18-6-6 6-6" />
      </svg>
      <span>"Previous"</span>
    </PaginationLink>
  }
}

#[component]
pub fn PaginationNext(
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] href: String,
  #[prop(optional)] disabled: bool,
) -> impl IntoView {
  view! {
    <PaginationLink class=cn(&["gap-1 pr-2.5", class.as_str()]) href=href disabled=disabled>
      <span>"Next"</span>
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="size-4"
      >
        <path d="m9 18 6-6-6-6" />
      </svg>
    </PaginationLink>
  }
}

#[component]
pub fn PaginationEllipsis(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <span
      data-slot="pagination-ellipsis"
      aria-hidden="true"
      class=cn(&["flex size-9 items-center justify-center", class.as_str()])
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="size-4"
      >
        <circle cx="12" cy="12" r="1" />
        <circle cx="19" cy="12" r="1" />
        <circle cx="5" cy="12" r="1" />
      </svg>
      <span class="sr-only">"More pages"</span>
    </span>
  }
}
