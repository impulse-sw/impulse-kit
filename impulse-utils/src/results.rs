//! Result types for `ServerError` (`cc-server-kit`) and `CliError` (`cc-ui-kit`) errors.

#[cfg(feature = "mresult")]
use crate::errors::ServerError;

/// Simple backend result type.
#[cfg(feature = "mresult")]
pub type MResult<T> = Result<T, ServerError>;

#[cfg(feature = "cresult")]
use crate::errors::CliError;

/// Simple frontend result type.
#[cfg(feature = "cresult")]
pub type CResult<T> = Result<T, CliError>;
