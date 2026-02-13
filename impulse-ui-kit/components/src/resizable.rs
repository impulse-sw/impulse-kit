#![allow(missing_docs, dead_code)]

// Resizable panels component
// This is a simplified implementation that can be expanded with full drag-to-resize functionality

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[derive(Copy, Clone, PartialEq)]
pub enum ResizableDirection {
  Horizontal,
  Vertical,
}

#[component]
pub fn ResizablePanelGroup(
  #[prop(optional)] direction: Option<ResizableDirection>,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let direction = direction.unwrap_or(ResizableDirection::Horizontal);

  let direction_class = match direction {
    ResizableDirection::Horizontal => "flex-row",
    ResizableDirection::Vertical => "flex-col",
  };

  view! {
    <div
      data-slot="resizable-panel-group"
      data-direction=match direction {
        ResizableDirection::Horizontal => "horizontal",
        ResizableDirection::Vertical => "vertical",
      }
      class=cn(&["flex h-full w-full", direction_class, class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ResizablePanel(
  #[prop(optional)] default_size: Option<f64>,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let default_size = default_size.unwrap_or(50.0);

  view! {
    <div
      data-slot="resizable-panel"
      class=cn(&["relative", class.as_str()])
      style:flex=format!("{} 1 0%", default_size)
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ResizableHandle(#[prop(into, optional)] class: String) -> impl IntoView {
  view! {
    <div
      data-slot="resizable-handle"
      class=cn(
        &[
          "relative flex w-px items-center justify-center bg-border after:absolute after:inset-y-0 after:left-1/2 after:w-1 after:-translate-x-1/2 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-offset-1 data-[panel-group-direction=vertical]:h-px data-[panel-group-direction=vertical]:w-full data-[panel-group-direction=vertical]:after:left-0 data-[panel-group-direction=vertical]:after:h-1 data-[panel-group-direction=vertical]:after:w-full data-[panel-group-direction=vertical]:after:-translate-y-1/2 data-[panel-group-direction=vertical]:after:translate-x-0 [&[data-panel-group-direction=vertical]>div]:rotate-90",
          class.as_str(),
        ],
      )
    >
      <div class="z-10 flex h-4 w-3 items-center justify-center rounded-sm border bg-border">
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
          class="h-2.5 w-2.5"
        >
          <circle cx="9" cy="12" r="1" />
          <circle cx="9" cy="5" r="1" />
          <circle cx="9" cy="19" r="1" />
          <circle cx="15" cy="12" r="1" />
          <circle cx="15" cy="5" r="1" />
          <circle cx="15" cy="19" r="1" />
        </svg>
      </div>
    </div>
  }
}
