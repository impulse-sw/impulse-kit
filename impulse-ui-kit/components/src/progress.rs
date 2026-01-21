#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Progress(
  #[prop(into, optional)] value: MaybeSignal<f64>,
  #[prop(optional)] max: Option<f64>,
  #[prop(into, optional)] class: String,
) -> impl IntoView {
  let max = max.unwrap_or(100.0);

  let percentage = move || {
    let v = value.get();
    ((v / max) * 100.0).clamp(0.0, 100.0)
  };

  view! {
    <div
      data-slot="progress"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax=max
      aria-valuenow=move || value.get()
      class=cn(&["relative h-4 w-full overflow-hidden rounded-full bg-secondary", class.as_str()])
    >
      <div
        data-slot="progress-indicator"
        class="h-full w-full flex-1 bg-primary transition-all"
        style:transform=move || format!("translateX(-{}%)", 100.0 - percentage())
      />
    </div>
  }
}
