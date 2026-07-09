//! Client Kit framework built on top of Leptos.
//!
//! Selects between three rendering modes via mutually exclusive Cargo features:
//! `csr` (client-side rendering), `hydrate` (client hydration of SSR markup,
//! reserved for the upcoming iteration), and `ssr` (server-side rendering, used
//! together with `impulse-server-kit` under its `leptos-ssr` feature).

#![allow(non_snake_case)]
#![warn(missing_docs)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

#[cfg(not(any(feature = "csr", feature = "hydrate", feature = "ssr")))]
compile_error!("impulse-client-kit: enable exactly one of `csr`, `hydrate`, `ssr` features");

#[cfg(any(
  all(feature = "csr", feature = "hydrate"),
  all(feature = "csr", feature = "ssr"),
  all(feature = "hydrate", feature = "ssr"),
))]
compile_error!("impulse-client-kit: features `csr`, `hydrate`, `ssr` are mutually exclusive");

#[cfg(all(feature = "websocket", not(any(feature = "csr", feature = "hydrate"))))]
compile_error!("impulse-client-kit: feature `websocket` requires `csr` or `hydrate`");

#[cfg(all(feature = "webtransport", not(any(feature = "csr", feature = "hydrate"))))]
compile_error!("impulse-client-kit: feature `webtransport` requires `csr` or `hydrate`");

pub mod router;
pub mod utils;

/// Unified REST client with one API across the direct (reqwest) and Tauri-IPC
/// backends. See the module docs for the auth-interceptor pattern.
#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "telemetry")]
pub mod telemetry;

#[cfg(any(feature = "websocket", feature = "webtransport"))]
pub mod reconnect;
#[cfg(feature = "websocket")]
pub mod ws;
#[cfg(feature = "webtransport")]
pub mod wt;

#[cfg(feature = "ssr")]
pub mod ssr;

pub mod prelude;

use leptos::prelude::*;

/// Application entrypoint.
///
/// On `csr`, mounts the app to the document body via [`leptos::mount::mount_to_body`].
/// On `hydrate`, hydrates pre-rendered SSR markup (placeholder until hydration
/// support lands; see `impulse-ssr-support` roadmap).
/// On `ssr`, this is a no-op — the server entrypoint is
/// `impulse_server_kit::leptos_ssr::leptos_router`.
///
/// ```rust,ignore
/// fn main() {
///   setup_app(log::Level::Info, Box::new(move || view! { <App /> }.into_any()))
/// }
/// ```
#[cfg(feature = "csr")]
pub fn setup_app(#[allow(unused_variables)] log_level: log::Level, children: Children) {
  console_error_panic_hook::set_once();
  #[cfg(debug_assertions)]
  console_log::init_with_level(log::Level::Debug).unwrap();
  #[cfg(not(debug_assertions))]
  console_log::init_with_level(log_level).unwrap();
  leptos::mount::mount_to_body(move || view! { {children()} })
}

/// Application entrypoint (hydration mode).
///
/// Hydrates server-rendered HTML in place via [`leptos::mount::hydrate_body`].
/// The wasm bundle that calls `setup_app` must export a `hydrate` function via
/// `#[wasm_bindgen]` so the SSR-emitted bootstrap `<script>` can drive
/// hydration:
///
/// ```rust,ignore
/// #[wasm_bindgen::prelude::wasm_bindgen]
/// pub fn hydrate() {
///   impulse_client_kit::setup_app(log::Level::Info, Box::new(move || view! { <App/> }.into_any()))
/// }
/// ```
#[cfg(feature = "hydrate")]
pub fn setup_app(#[allow(unused_variables)] log_level: log::Level, children: Children) {
  console_error_panic_hook::set_once();
  #[cfg(debug_assertions)]
  console_log::init_with_level(log::Level::Debug).unwrap();
  #[cfg(not(debug_assertions))]
  console_log::init_with_level(log_level).unwrap();
  leptos::mount::hydrate_body(move || view! { {children()} })
}

/// Application entrypoint (SSR mode).
///
/// No-op on the server. Server-side rendering is driven by
/// `impulse-server-kit::leptos_ssr::leptos_router`, which sets up per-request
/// contexts and renders your `App` component into HTML.
#[cfg(feature = "ssr")]
pub fn setup_app(_log_level: log::Level, _children: Children) {}
