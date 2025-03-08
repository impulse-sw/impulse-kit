//! Result types for `ErrorResponse` (`salvo`) and `CliError` (`reqwest`) errors.

#[cfg(feature = "mresult")]
use crate::errors::ErrorResponse;

/// Simple backend result type.
#[cfg(feature = "mresult")]
pub type MResult<T> = Result<T, ErrorResponse>;

#[cfg(feature = "cresult")]
use crate::errors::CliError;

/// Simple frontend result type.
#[cfg(feature = "cresult")]
pub type CResult<T> = Result<T, CliError>;
