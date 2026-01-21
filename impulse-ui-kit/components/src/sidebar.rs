#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy)]
struct SidebarContext {
  is_open: RwSignal<bool>,
  side: SidebarSide,
}

#[derive(Copy, Clone, PartialEq, Default)]
pub enum SidebarSide {
  #[default]
  Left,
  Right,
}

#[component]
pub fn SidebarProvider(
  #[prop(optional)] open: Option<RwSignal<bool>>,
  #[prop(optional)] default_open: Option<bool>,
  #[prop(optional)] side: Option<SidebarSide>,
  children: Children,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(default_open.unwrap_or(true)));
  let side = side.unwrap_or(SidebarSide::Left);

  provide_context(SidebarContext { is_open, side });

  view! { <div data-slot="sidebar-provider" class="flex min-h-screen">{children()}</div> }
}

#[component]
pub fn Sidebar(#[prop(into, optional)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<SidebarContext>().expect("Sidebar must be used within SidebarProvider");
  let children = StoredValue::new(children);

  view! {
    <Show when=move || context.is_open.get()>
      <aside
        data-slot="sidebar"
        class=cn(
          &[
            "flex h-full w-64 flex-col border-r bg-background transition-transform",
            class.as_str(),
          ],
        )
      >
        {children.get_value()()}
      </aside>
    </Show>
  }
}

#[component]
pub fn SidebarHeader(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="sidebar-header" class=cn(&["flex items-center gap-2 border-b p-4", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn SidebarContent(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="sidebar-content" class=cn(&["flex-1 overflow-auto p-4", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn SidebarFooter(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="sidebar-footer" class=cn(&["border-t p-4", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn SidebarTrigger(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<SidebarContext>().expect("SidebarTrigger must be used within SidebarProvider");

  let handle_click = move |_| {
    context.is_open.update(|open| *open = !*open);
  };

  view! {
    <button type="button" data-slot="sidebar-trigger" class=class on:click=handle_click>
      {children()}
    </button>
  }
}

#[component]
pub fn SidebarMenu(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <nav data-slot="sidebar-menu" class=cn(&["space-y-1", class.as_str()])>
      {children()}
    </nav>
  }
}

#[component]
pub fn SidebarMenuItem(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="sidebar-menu-item"
      class=cn(
        &[
          "flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-accent hover:text-accent-foreground cursor-pointer",
          class.as_str(),
        ],
      )
    >
      {children()}
    </div>
  }
}
