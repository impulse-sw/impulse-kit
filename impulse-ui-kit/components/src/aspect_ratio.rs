#![allow(missing_docs, dead_code)]

use leptos::prelude::*;

#[component]
pub fn AspectRatio(
  #[prop(into, default = 1.0)] ratio: f32,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      data-slot="aspect-ratio"
      class=class
      style=format!("position: relative; width: 100%; padding-bottom: {}%;", 100.0 / ratio)
    >
      <div style="position: absolute; inset: 0;">{children()}</div>
    </div>
  }
}
