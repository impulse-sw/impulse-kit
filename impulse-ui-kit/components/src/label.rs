#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn Label(
  #[prop(optional, into)] r#for: String,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <label
      data-slot="label"
      for=r#for
      class=cn(
        &[
          "flex items-center gap-2 text-sm leading-none font-medium select-none group-data-[disabled=true]:pointer-events-none group-data-[disabled=true]:opacity-50 peer-disabled:cursor-not-allowed peer-disabled:opacity-50",
          class.as_str(),
        ],
      )
    >
      {children()}
    </label>
  }
}
