//! First-party Leptos SSR adapter for Salvo.
//!
//! This module provides a native server-side rendering integration for Leptos
//! 0.8 on top of Salvo. It does not depend on `leptos_axum`/`leptos_actix`.
//!
//! Capabilities:
//!
//! - SEO-grade HTML rendering with `leptos_meta` (`<title>`, `<meta>`,
//!   `<link>`, OpenGraph, Twitter Cards, canonical, robots, locale).
//! - `<Suspense>` streaming via in-order or out-of-order modes
//!   ([`handler::SsrStreamMode`]).
//! - Hydration bootstrap: when [`LeptosOptions::include_hydration_script`] is
//!   `true`, the rendered HTML embeds the `<script>` that loads the wasm
//!   bundle and calls `hydrate()` on the client.
//! - `#[server]` functions through [`server_fn_router`], which bridges Salvo
//!   to `server_fn`'s axum-compatible adapter.
//!
//! Typical wiring:
//!
//! ```rust,ignore
//! use impulse_server_kit::prelude::*;
//!
//! #[tokio::main]
//! async fn main() {
//!   let setup = load_generic_config::<Setup>("server").await.unwrap();
//!   let state = load_generic_state(&setup, true).await.unwrap();
//!   let opts  = LeptosOptions::from_generic_values(setup.generic_values());
//!
//!   let router = get_root_router_autoinject(&state, setup.clone())
//!     .push(server_fn_router(opts.server_fn_prefix.clone()))
//!     .push(leptos_router(opts, || leptos::view! { <App/> }));
//!
//!   let (server, _) = start(state, &setup, router).await.unwrap();
//!   server.await
//! }
//! ```

mod assets;
mod handler;
mod options;
mod prefix;
#[cfg(feature = "leptos-server-fn")]
mod server_fn;
mod theme;

pub use assets::assets_only_router;
pub use handler::{LeptosSsrHandler, SsrStreamMode, leptos_router};
pub use options::{
  CONTAINER_FRONTEND_DISTRIBUTABLE, FRONTEND_DIST_ENV, FallbackStrategy, LOCAL_FRONTEND_DISTRIBUTABLE, LeptosOptions,
  PKG_SUBDIR, SeoDefaults,
};
pub use theme::{InitialTheme, parse_theme_cookie};

#[cfg(feature = "leptos-server-fn")]
pub use server_fn::{ServerFnSalvoHandler, server_fn_router};

pub use leptos;
pub use leptos_meta;
