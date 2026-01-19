#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn NavigationMenu(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <nav
      data-slot="navigation-menu"
      class=cn(&["relative z-10 flex max-w-max flex-1 items-center justify-center", class.as_str()])
    >
      {children()}
    </nav>
  }
}

#[component]
pub fn NavigationMenuList(
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <ul
      data-slot="navigation-menu-list"
      class=cn(
        &[
          "group flex flex-1 list-none items-center justify-center gap-1",
          class.as_str(),
        ],
      )
    >

      {children()}
    </ul>
  }
}

#[component]
pub fn NavigationMenuItem(
  #[prop(optional, into)] class: String,
  #[prop(optional)] open: Option<RwSignal<bool>>,
  children: Children,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(false));

  provide_context(NavigationMenuItemContext { is_open });

  view! {
    <li data-slot="navigation-menu-item" class=class>
      {children()}
    </li>
  }
}

#[component]
pub fn NavigationMenuTrigger(
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let context = use_context::<NavigationMenuItemContext>()
    .expect("NavigationMenuTrigger must be used within NavigationMenuItem");

  let handle_click = move |_| {
    context.is_open.update(|open| *open = !*open);
  };

  view! {
    <button
      data-slot="navigation-menu-trigger"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      class=cn(
        &[
          "hover:bg-accent hover:text-accent-foreground focus-visible:ring-ring/50 data-[state=open]:bg-accent/50 group inline-flex h-10 w-max items-center justify-center gap-1 rounded-md bg-transparent px-4 py-2 text-sm font-medium transition-colors outline-none focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50",
          class.as_str(),
        ],
      )

      on:click=handle_click
    >
      {children()}
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
        class="relative top-px ml-1 size-3 transition-transform duration-200 group-data-[state=open]:rotate-180"
        aria-hidden="true"
      >
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>
  }
}

#[component]
pub fn NavigationMenuContent(
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<NavigationMenuItemContext>()
    .expect("NavigationMenuContent must be used within NavigationMenuItem");

  let children = StoredValue::new(children);

  view! {
    <div
      data-slot="navigation-menu-content"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      class=cn(
        &[
          "data-[state=closed]:animate-out data-[state=open]:animate-in data-[state=closed]:fade-out data-[state=open]:fade-in data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 absolute left-0 top-full mt-1.5 w-full data-[state=closed]:invisible data-[state=closed]:pointer-events-none md:w-auto",
          class.as_str(),
        ],
      )
    >

      <div class="bg-popover text-popover-foreground overflow-hidden rounded-md border shadow-lg">
        {children.read_value()()}
      </div>
    </div>
  }
}

#[component]
pub fn NavigationMenuLink(
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] href: String,
  children: Children,
) -> impl IntoView {
  view! {
    <a
      data-slot="navigation-menu-link"
      href=href
      class=cn(
        &[
          "hover:bg-accent hover:text-accent-foreground focus-visible:ring-ring/50 block select-none space-y-1 rounded-md p-3 leading-none no-underline outline-none transition-colors focus-visible:ring-[3px]",
          class.as_str(),
        ],
      )
    >

      {children()}
    </a>
  }
}

#[component]
pub fn NavigationMenuViewport(
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      data-slot="navigation-menu-viewport"
      class=cn(
        &[
          "origin-top-center bg-popover text-popover-foreground relative mt-1.5 h-[var(--radix-navigation-menu-viewport-height)] w-full overflow-hidden rounded-md border shadow-lg data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-90 md:w-[var(--radix-navigation-menu-viewport-width)]",
          class.as_str(),
        ],
      )
    >

      {children()}
    </div>
  }
}

#[component]
pub fn NavigationMenuIndicator(#[prop(optional, into)] class: String) -> impl IntoView {
  view! {
    <div
      data-slot="navigation-menu-indicator"
      class=cn(
        &[
          "data-[state=visible]:animate-in data-[state=hidden]:animate-out data-[state=hidden]:fade-out data-[state=visible]:fade-in top-full z-[1] flex h-1.5 items-end justify-center overflow-hidden",
          class.as_str(),
        ],
      )
    >

      <div class="bg-border relative top-[60%] size-2 rotate-45 rounded-tl-sm shadow-md" />
    </div>
  }
}

#[derive(Clone, Copy)]
struct NavigationMenuItemContext {
  is_open: RwSignal<bool>,
}
