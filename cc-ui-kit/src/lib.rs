//! UI Kit framework built on top of Leptos and Thaw.
//!
//! Provides a simple `setup_app` function to launch your
//! CSR (client-side rendered) application.

#![feature(let_chains)]
#![allow(non_snake_case)]
#![warn(missing_docs)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

pub mod router;
pub mod utils;

pub mod prelude;
