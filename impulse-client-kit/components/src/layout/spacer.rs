#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

/// A flexible spacer that grows to fill the free space along a [`Row`] or
/// [`Column`]'s main axis, pushing siblings apart.
///
/// [`Row`]: super::Row
/// [`Column`]: super::Column
#[component]
pub fn Spacer(#[prop(optional, into)] class: String) -> impl IntoView {
  view! { <div data-slot="spacer" class=cn(&["flex-1", class.as_str()]) /> }
}
