//! Static server for your frontend.
//!
//! This crate is a thin re-export of the `static_server` module in
//! `impulse-server-kit`. Use the binary `iks` to run a stand-alone static
//! server, or pull these items in as a library:
//!
//! ```rust,ignore
//! use impulse_static_server::{frontend_router, StaticRouter};
//! ```

#![warn(missing_docs)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

pub use impulse_server_kit::static_server::{
  CacheMap, CachedFile, NoRedirectStaticRouter, ProvidedRoutesStaticRouter, StaticRouter, assets_only_router_from,
  cache_runner, frontend_router, frontend_router_from_given_dist, send_file, send_html,
};
