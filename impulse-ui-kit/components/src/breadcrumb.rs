#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Breadcrumb(children: Children) -> impl IntoView {
  view! {
    <nav aria-label="breadcrumb" data-slot="breadcrumb">
      {children()}
    </nav>
  }
}

#[component]
pub fn BreadcrumbList(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <ol
      data-slot="breadcrumb-list"
      class=cn(
        &[
          "text-muted-foreground flex flex-wrap items-center gap-1.5 text-sm break-words sm:gap-2.5",
          class.as_str(),
        ],
      )
    >
      {children()}
    </ol>
  }
}

#[component]
pub fn BreadcrumbItem(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <li data-slot="breadcrumb-item" class=cn(&["inline-flex items-center gap-1.5", class.as_str()])>
      {children()}
    </li>
  }
}

#[component]
pub fn BreadcrumbLink(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <a
      class=cn(&["hover:text-foreground transition-colors", class.as_str()])
      data-slot="breadcrumb-link"
    >
      {children()}
    </a>
  }
}

#[component]
pub fn BreadcrumbPage(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <span
      data-slot="breadcrumb-page"
      role="link"
      aria-disabled="true"
      aria-current="page"
      class=cn(&["text-foreground font-normal", class.as_str()])
    >
      {children()}
    </span>
  }
}

#[component]
pub fn BreadcrumbSeparator(
  #[prop(into, optional)] class: String,
  #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
  if let Some(children) = children {
    view! {
      <li
        data-slot="breadcrumb-separator"
        role="presentation"
        aria-hidden="true"
        class=cn(&["[&>svg]:size-3.5", class.as_str()])
      >
        {children()}
      </li>
    }
    .into_any()
  } else {
    view! {
      <li
        data-slot="breadcrumb-separator"
        role="presentation"
        aria-hidden="true"
        class=cn(&["[&>svg]:size-3.5", class.as_str()])
      >
        <ChevronRight class="size-3.5" />
      </li>
    }
    .into_any()
  }
}

#[component]
pub fn BreadcrumbEllipsis(#[prop(into, optional)] class: String) -> impl IntoView {
  view! {
    <span
      data-slot="breadcrumb-ellipsis"
      role="presentation"
      aria-hidden="true"
      class=cn(&["flex size-9 items-center justify-center", class.as_str()])
    >
      <MoreHorizontal class="size-4" />
      <span class="sr-only">"More"</span>
    </span>
  }
}

#[component]
fn ChevronRight(#[prop(into, optional)] class: String) -> impl IntoView {
  view! {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class=class
    >
      <path d="m9 18 6-6-6-6" />
    </svg>
  }
}

#[component]
fn MoreHorizontal(#[prop(into, optional)] class: String) -> impl IntoView {
  view! {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class=class
    >
      <circle cx="12" cy="12" r="1" />
      <circle cx="19" cy="12" r="1" />
      <circle cx="5" cy="12" r="1" />
    </svg>
  }
}
