//! UI Kit prelude.

pub use cc_utils;
pub use console_error_panic_hook;
pub use leptos::prelude::*;

pub use console_log;
pub use log;

pub use thaw::*;

/// Application entrypoint.
///
/// Just specify log level and needed view:
///
/// ```rust,ignore
/// fn main() {
///   setup_app(log::Level::Info, Box::new(move || { view! { <App /> }.into_any() }))
/// }
/// ```
pub fn setup_app(#[allow(unused_variables)] log_level: log::Level, children: Children) {
  console_error_panic_hook::set_once();
  #[cfg(debug_assertions)]
  console_log::init_with_level(log::Level::Debug).unwrap();
  #[cfg(not(debug_assertions))]
  console_log::init_with_level(log_level).unwrap();
  leptos::mount::mount_to_body(move || {
    view! { <UIApp children /> }
  })
}

/// Also, you can use main styled `UIApp` component without `setup_app`, if you want more flexibility.
#[component]
pub fn UIApp(children: Children) -> impl IntoView {
  use crate::utils::{cn, dark_theme, light_theme};

  let leptos_use::UseColorModeReturn { mode, .. } = leptos_use::use_color_mode();
  let tw_dark_class = RwSignal::new(if let leptos_use::ColorMode::Dark = mode.get() {
    Some("dark")
  } else {
    None
  });
  let theme = RwSignal::new({
    if let leptos_use::ColorMode::Dark = mode.get() {
      dark_theme()
    } else {
      light_theme()
    }
  });
  Effect::new(move |_| {
    theme.set(if let leptos_use::ColorMode::Dark = mode.get() {
      dark_theme()
    } else {
      light_theme()
    })
  });

  view! {
    <ConfigProvider theme class=cn("uikit-app-container", tw_dark_class.get())>
      <div class="uikit-app-content">{children()}</div>
    </ConfigProvider>
  }
}
