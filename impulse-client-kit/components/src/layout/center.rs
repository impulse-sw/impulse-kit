#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

/// Centers its children on both axes.
///
/// Set `inline` to use `inline-flex` instead of `flex`, so the container only
/// takes up as much width as its content.
#[component]
pub fn Center(
  #[prop(optional, default = false)] inline: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let display = if inline { "inline-flex" } else { "flex" };
  view! {
    <div data-slot="center" class=cn(&[display, "items-center justify-center", class.as_str()])>
      {children()}
    </div>
  }
}
