//! Static server for your frontend.
//!
//! You can use it by itself (with `cargo build`) or include
//! as a library and use `frontend_router()`.

#![warn(missing_docs)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

mod static_routes;

pub use static_routes::frontend_router;
