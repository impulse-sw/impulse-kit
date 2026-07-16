//! Transport-agnostic HTTP wire types and a small endpoint/router abstraction.
//!
//! This crate is the neutral foundation of the request stack: it depends on
//! neither reqwest, salvo, tauri nor leptos, so it can be shared by
//!
//! * the **client transport** (`impulse_client_kit::client`) — which executes an
//!   [`HttpRequest`] via reqwest (browser fetch / native TLS) or forwards it over
//!   Tauri IPC, and reads back an [`HttpResponse`];
//! * the **Tauri engine** (`impulse-tauri-engine`) — which handles an
//!   [`HttpRequest`] against a local [`Router`] while offline;
//! * a **server adapter** — which mounts the same [`Router`] into salvo.
//!
//! The point is that request-handling logic ([`Endpoint`]s in a [`Router`]) is
//! written once and mounted on either host.
//!
//! The optional `reqwest` feature adds `impl From<Method> for reqwest::Method`
//! for the client transport; it is off by default so the server / engine can use
//! the wire types and router without pulling reqwest.

#![deny(warnings, clippy::todo, clippy::unimplemented, missing_docs)]

mod router;
mod wire;

pub use router::{
  Endpoint, EndpointCtx, EndpointFuture, EndpointResponse, PathParams, PathPattern, Route, Router,
};
pub use wire::{HttpRequest, HttpResponse, Method};
