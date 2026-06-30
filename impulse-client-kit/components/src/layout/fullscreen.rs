#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

/// A section that fills at least the full height of the viewport.
///
/// Set `center` to center its children on both axes — handy for splash
/// screens, loaders, and empty states.
#[component]
pub fn FullScreen(
  #[prop(optional, default = false)] center: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let center_class = if center { "flex items-center justify-center" } else { "" };
  view! {
    <div data-slot="full-screen" class=cn(&["min-h-screen w-full", center_class, class.as_str()])>
      {children()}
    </div>
  }
}
