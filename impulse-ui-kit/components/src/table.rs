#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Table(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div class="relative w-full overflow-auto">
      <table data-slot="table" class=cn(&["w-full caption-bottom text-sm", class.as_str()])>
        {children()}
      </table>
    </div>
  }
}

#[component]
pub fn TableHeader(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <thead data-slot="table-header" class=cn(&["[&_tr]:border-b", class.as_str()])>
      {children()}
    </thead>
  }
}

#[component]
pub fn TableBody(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <tbody data-slot="table-body" class=cn(&["[&_tr:last-child]:border-0", class.as_str()])>
      {children()}
    </tbody>
  }
}

#[component]
pub fn TableFooter(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <tfoot
      data-slot="table-footer"
      class=cn(&["bg-muted/50 border-t font-medium [&>tr]:last:border-b-0", class.as_str()])
    >
      {children()}
    </tfoot>
  }
}

#[component]
pub fn TableRow(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <tr
      data-slot="table-row"
      class=cn(
        &[
          "border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted",
          class.as_str(),
        ],
      )
    >
      {children()}
    </tr>
  }
}

#[component]
pub fn TableHead(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <th
      data-slot="table-head"
      class=cn(
        &[
          "text-muted-foreground h-12 px-4 text-left align-middle font-medium [&:has([role=checkbox])]:pr-0",
          class.as_str(),
        ],
      )
    >
      {children()}
    </th>
  }
}

#[component]
pub fn TableCell(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <td
      data-slot="table-cell"
      class=cn(&["p-4 align-middle [&:has([role=checkbox])]:pr-0", class.as_str()])
    >
      {children()}
    </td>
  }
}

#[component]
pub fn TableCaption(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <caption
      data-slot="table-caption"
      class=cn(&["text-muted-foreground mt-4 text-sm", class.as_str()])
    >
      {children()}
    </caption>
  }
}
