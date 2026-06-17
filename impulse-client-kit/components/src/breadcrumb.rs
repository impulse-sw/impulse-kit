#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
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

/// A collapsible breadcrumb item that reveals its children in a small dropdown.
///
/// Handy for narrow / mobile headers where the full trail (or a set of section
/// links and actions) does not fit: render the secondary items as children and
/// they fold away behind `label` until tapped. Built on the native `<details>`
/// disclosure, so it needs no client-side state and works without JS.
#[component]
pub fn BreadcrumbMenu(
  #[prop(into, optional)] class: String,
  #[prop(into, optional)] label: String,
  children: Children,
) -> impl IntoView {
  let label = if label.is_empty() { "…".to_string() } else { label };
  view! {
    <li data-slot="breadcrumb-menu" class=cn(&["inline-flex items-center", class.as_str()])>
      <details class="group relative">
        <summary class="text-muted-foreground hover:text-foreground flex cursor-pointer list-none items-center gap-1 transition-colors [&::-webkit-details-marker]:hidden">
          {label} <ChevronRight class="size-3.5 transition-transform group-open:rotate-90" />
        </summary>
        <ul class="bg-popover text-popover-foreground absolute left-0 z-50 mt-1 flex min-w-40 flex-col gap-1 rounded-md border p-1 shadow-md">
          {children()}
        </ul>
      </details>
    </li>
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
