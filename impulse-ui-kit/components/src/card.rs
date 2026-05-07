#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Card(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="card"
      class=cn(
        &[
          "bg-card text-card-foreground flex flex-col gap-6 rounded-xl border py-6 shadow-sm",
          class.as_str(),
        ],
      )
    >
      {children()}
    </div>
  }
}

#[component]
pub fn CardHeader(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="card-header"
      class=cn(
        &[
          "@container/card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-2 px-6 has-data-[slot=card-action]:grid-cols-[1fr_auto] [.border-b]:pb-6",
          class.as_str(),
        ],
      )
    >
      {children()}
    </div>
  }
}

#[component]
pub fn CardTitle(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="card-title" class=cn(&["leading-none font-semibold", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn CardDescription(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="card-description" class=cn(&["text-muted-foreground text-sm", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn CardAction(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="card-action"
      class=cn(&["col-start-2 row-span-2 row-start-1 self-start justify-self-end", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn CardContent(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="card-content" class=cn(&["px-6", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn CardFooter(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="card-footer"
      class=cn(&["flex items-center px-6 [.border-t]:pt-6", class.as_str()])
    >
      {children()}
    </div>
  }
}
