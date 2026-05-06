//! First-party Leptos SSR adapter for Salvo.
//!
//! This module provides a minimal, native server-side rendering integration
//! for Leptos 0.8 on top of Salvo. It is intentionally framework-agnostic —
//! we do not depend on `leptos_axum`/`leptos_actix`. The current iteration
//! delivers SEO-grade HTML rendering (with `leptos_meta` integration) but
//! does NOT yet wire up hydration or `#[server]` functions; both are
//! reserved for the next iteration and the public API is shaped so that
//! adding them is non-breaking.
//!
//! Typical usage:
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
mod theme;

pub use assets::assets_only_router;
pub use handler::{LeptosSsrHandler, leptos_router};
pub use options::{FallbackStrategy, LeptosOptions, SeoDefaults};
pub use theme::{InitialTheme, parse_theme_cookie};

pub use leptos;
pub use leptos_meta;
