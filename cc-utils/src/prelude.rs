//! Fast access to nice things.

#[cfg(feature = "salvo")]
pub use crate::requests::{MsgPackParser, SimdJsonParser};

#[cfg(all(feature = "reqwest", feature = "cresult"))]
pub use crate::requests::MsgPackRequest;

#[cfg(feature = "mresult")]
pub use crate::results::MResult;

#[cfg(feature = "cresult")]
pub use crate::results::CResult;

#[cfg(feature = "salvo")]
pub use crate::responses::{File, Html, Json, MsgPack, OK, Plain};

#[cfg(feature = "salvo")]
pub use crate::{file_upload, html, json, msgpack, ok, plain};

#[cfg(all(feature = "reqwest", feature = "cresult"))]
pub use crate::responses::MsgPackResponse;

#[cfg(all(feature = "reqwest", feature = "cresult"))]
pub use crate::responses::CollectServerError;

#[cfg(all(feature = "reqwest", feature = "mresult"))]
pub use crate::responses::RedirectServerError;

#[cfg(feature = "mresult")]
pub use crate::errors::ServerError;

#[cfg(feature = "cresult")]
pub use crate::errors::CliError;

pub use crate::errors::ErrorResponse;

#[cfg(feature = "salvo")]
pub use salvo::oapi::endpoint;

#[cfg(feature = "salvo")]
pub use salvo::http::StatusCode;
