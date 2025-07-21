//! Static server for your frontend.
//!
//! You can use it by itself (with `cargo build`) or include
//! as a library and use `frontend_router()`.

#![warn(missing_docs)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]
#![feature(if_let_guard, let_chains, string_from_utf8_lossy_owned)]

mod caching;
mod static_routes;

pub use caching::CacheMap;
pub use static_routes::{
  NoRedirectStaticRouter, ProvidedRoutesStaticRouter, StaticRouter, frontend_router, frontend_router_from_given_dist,
};
