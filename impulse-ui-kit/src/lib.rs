//! UI Kit framework built on top of Leptos.
//!
//! Provides a simple `setup_app` function to launch your
//! CSR (client-side rendered) application.

#![allow(non_snake_case)]
#![warn(missing_docs)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

pub mod router;
pub mod utils;

pub mod prelude;

use leptos::prelude::*;

/// Application entrypoint.
///
/// Just specify log level and needed view:
///
/// ```rust,ignore
/// fn main() {
///   setup_app(log::Level::Info, Box::new(move || view! { <App /> }.into_any()))
/// }
/// ```
pub fn setup_app(#[allow(unused_variables)] log_level: log::Level, children: Children) {
  console_error_panic_hook::set_once();
  #[cfg(debug_assertions)]
  console_log::init_with_level(log::Level::Debug).unwrap();
  #[cfg(not(debug_assertions))]
  console_log::init_with_level(log_level).unwrap();
  leptos::mount::mount_to_body(move || view! { {children()} })
}
