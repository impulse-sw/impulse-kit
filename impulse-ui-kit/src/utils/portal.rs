//! Portal component for rendering content outside the current DOM hierarchy.
//!
//! This is useful for overlays, modals, tooltips, and other components that need
//! to break out of the parent's overflow/z-index constraints.

use leptos::prelude::*;

/// Portal component that renders children into a different DOM location.
///
/// By default, renders into `document.body`, but you can specify a custom mount point.
///
/// # Example
///
/// ```rust
/// use impulse_ui_kit::utils::Portal;
///
/// #[component]
/// fn MyModal() -> impl IntoView {
///   view! {
///     <Portal>
///       <div class="fixed inset-0 z-50">
///         "This will be rendered at document.body level"
///       </div>
///     </Portal>
///   }
/// }
/// ```
#[component]
pub fn Portal(
  /// Optional mount point selector (defaults to body)
  #[prop(optional)]
  mount: Option<String>,
  /// Content to render in the portal
  children: ChildrenFn,
) -> impl IntoView {
  let _mount_point = mount.unwrap_or_else(|| "body".to_string());
  let children = StoredValue::new(children);

  view! {
    <leptos::portal::Portal mount=leptos::tachys::dom::body()>
      {children.read_value()()}
    </leptos::portal::Portal>
  }
}
