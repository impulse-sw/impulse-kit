#![allow(missing_docs, dead_code)]

use leptos::prelude::*;

#[component]
pub fn AspectRatio(
  #[prop(into, default = RwSignal::new(1.0))] ratio: RwSignal<f32>,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      data-slot="aspect-ratio"
      class=class
      style=move || {
        format!("position: relative; width: 100%; padding-bottom: {}%;", 100.0 / ratio.get())
      }
    >
      <div style="position: absolute; inset: 0;">{children()}</div>
    </div>
  }
}
