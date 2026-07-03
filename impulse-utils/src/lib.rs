//! Bunch of fullstack utils.

#![warn(missing_docs)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

pub mod errors;
pub mod requests;
pub mod responses;
pub mod results;

/// Recovery hooks for pages resumed from bfcache / a frozen background tab.
/// Browser-only: gated on `wasm32` and the `page-lifecycle` feature.
#[cfg(all(target_arch = "wasm32", feature = "page-lifecycle"))]
pub mod page_lifecycle;

#[cfg(feature = "telemetry")]
pub mod telemetry;

pub mod prelude;
