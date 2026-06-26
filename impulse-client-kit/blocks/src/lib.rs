#![deny(warnings)]
// Leptos' `TypedBuilder` derive (used by `#[component]`) can emit phantom
// lifetimes that newer clippy flags; the generated code is correct.
#![allow(clippy::extra_unused_lifetimes)]

//! Higher-level **blocks** for the Impulse Client Kit.
//!
//! Where [`impulse-client-kit-components`](impulse_client_kit_components) ships the
//! low-level building bricks (buttons, inputs, dialogs, …), this crate ships
//! ready-made *blocks*: small, self-contained widgets that solve a concrete
//! task and are themselves composed of those components — charts, graphs,
//! markdown, and the like. Think of them as pre-assembled blocks rather than
//! individual bricks.
//!
//! Blocks are wired into Tailwind through the same `build.rs` mechanism as the
//! components crate (see the workspace `README.md` and `impulse-tailwind-sources`).

pub mod charts;
pub mod graph;
pub mod landings;
pub mod markdown;
