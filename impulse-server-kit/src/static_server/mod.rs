//! Static-file server with in-memory caching, conditional requests and
//! filesystem-watch-driven invalidation.
//!
//! Originally extracted from `impulse-static-server`. Lives here so the
//! Leptos SSR adapter can reuse the same logged handlers for asset serving
//! instead of `salvo::serve_static::StaticDir`, which is silent.
//!
//! Typical usage:
//!
//! ```rust,ignore
//! use impulse_server_kit::prelude::*;
//!
//! let router = get_root_router(&state)
//!   .push(frontend_router()?);
//! ```

mod caching;
mod routes;

pub use caching::{CacheMap, CachedFile, cache_runner, send_file, send_html};
pub use routes::{
  NoRedirectStaticRouter, ProvidedRoutesStaticRouter, StaticRouter, assets_only_router_from, frontend_router,
  frontend_router_from_given_dist,
};
