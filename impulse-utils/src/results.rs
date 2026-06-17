//! Result types for `ServerError` (`impulse-server-kit`) and `ClientError` (`impulse-client-kit`) errors.

#[cfg(feature = "mresult")]
use crate::errors::ServerError;

/// Simple backend result type.
#[cfg(feature = "mresult")]
pub type MResult<T> = Result<T, ServerError>;

#[cfg(feature = "cresult")]
use crate::errors::ClientError;

/// Simple frontend result type.
#[cfg(feature = "cresult")]
pub type CResult<T> = Result<T, ClientError>;
