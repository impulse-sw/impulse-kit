#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Skeleton(#[prop(into, optional)] class: String) -> impl IntoView {
  view! {
    <div data-slot="skeleton" class=cn(&["animate-pulse rounded-md bg-muted", class.as_str()]) />
  }
}
