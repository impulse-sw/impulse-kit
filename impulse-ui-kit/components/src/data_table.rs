#![allow(missing_docs, dead_code)]

// Data Table component - builds on Table with sorting, filtering, and pagination
// This is a simplified implementation that can be expanded

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

use super::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};

#[component]
pub fn DataTable(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="data-table" class=cn(&["w-full", class.as_str()])>
      <Table>{children()}</Table>
    </div>
  }
}

#[component]
pub fn DataTableHeader(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! { <TableHeader class=class>{children()}</TableHeader> }
}

#[component]
pub fn DataTableBody(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! { <TableBody class=class>{children()}</TableBody> }
}

#[component]
pub fn DataTableRow(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! { <TableRow class=class>{children()}</TableRow> }
}

#[component]
pub fn DataTableHead(
  #[prop(into, optional)] class: String,
  #[prop(optional)] sortable: bool,
  #[prop(optional)] on_sort: Option<Callback<()>>,
  children: Children,
) -> impl IntoView {
  let handle_click = move |_| {
    if sortable && let Some(callback) = on_sort {
      callback.run(());
    }
  };

  view! {
    <TableHead
      class=cn(
        &[
          if sortable { "cursor-pointer select-none hover:bg-muted/50" } else { "" },
          class.as_str(),
        ],
      )
      on:click=handle_click
    >
      {children()}
    </TableHead>
  }
}

#[component]
pub fn DataTableCell(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! { <TableCell class=class>{children()}</TableCell> }
}

#[component]
pub fn DataTablePagination(
  #[prop(into, optional)] class: String,
  #[prop(optional)] page: RwSignal<usize>,
  #[prop(optional)] total_pages: usize,
  #[prop(optional)] on_page_change: Option<Callback<usize>>,
) -> impl IntoView {
  let handle_prev = move |_| {
    let current = page.get();
    if current > 1 {
      let new_page = current - 1;
      page.set(new_page);
      if let Some(callback) = on_page_change {
        callback.run(new_page);
      }
    }
  };

  view! {
    <div
      data-slot="data-table-pagination"
      class=cn(&["flex items-center justify-between px-2", class.as_str()])
    >
      <div class="text-sm text-muted-foreground">
        "Page " {move || page.get()} " of " {total_pages}
      </div>
      <div class="flex items-center gap-2">
        <button
          type="button"
          class="inline-flex h-8 items-center justify-center rounded-md border border-input bg-background px-3 text-sm font-medium ring-offset-background transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
          disabled=move || page.get() <= 1
          on:click=handle_prev
        >
          "Previous"
        </button>
        <button
          type="button"
          class="inline-flex h-8 items-center justify-center rounded-md border border-input bg-background px-3 text-sm font-medium ring-offset-background transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
          disabled=move || page.get()
        >
          = total_pages
          on:click=move |_|
          {
            let current = page.get();
            if current < total_pages {
              let new_page = current + 1;
              page.set(new_page);
              if let Some(callback) = on_page_change {
                callback.run(new_page);
              }
            }
          }
          >
          "Next"
        </button>
      </div>
    </div>
  }
}
