//! Standard prelude to import needed tools at once.

pub use cc_utils::{self, prelude::*};

pub use crate::generic_setup::{GenericSetup, GenericValues, load_generic_config, load_generic_state};
pub use crate::startup::{get_root_router, get_root_router_autoinject, start};
pub use salvo;
pub use tracing;
pub use tracing::instrument;

pub use salvo::handler;
pub use salvo::{Depot, Request, Response, Router};

#[cfg(feature = "oapi")]
pub use salvo::oapi::endpoint;

#[cfg(feature = "otel")]
pub use crate::otel;

#[cfg(feature = "test")]
pub use crate::test_exts::*;
