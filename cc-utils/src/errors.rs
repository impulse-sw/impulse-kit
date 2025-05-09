//! Implementation of optional private errors for `salvo` and client errors for `reqwest`.

#[cfg(feature = "salvo")]
use salvo::http::StatusCode;

#[cfg(feature = "salvo")]
use salvo::oapi::{EndpointOutRegister, ToSchema};

#[cfg(feature = "salvo")]
use salvo::{Depot, Request, Response};

#[cfg(feature = "salvo")]
use salvo::Writer as ServerResponseWriter;

/// Data structure responsible for server errors.
#[cfg(feature = "mresult")]
#[derive(Debug)]
pub struct ServerError {
  /// Status code to return.
  #[cfg(feature = "salvo")]
  #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
  pub status_code: Option<StatusCode>,
  /// Text to return (and hide error messages that leads to leak vulnerable data).
  pub public_msg: Option<String>,
  /// Text that really describes error situation.
  pub private_msg: Option<Vec<String>>,
}

/// Public error message.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "salvo", derive(salvo::oapi::ToSchema))]
pub struct ErrorResponse {
  /// Public error message.
  pub err: String,
}

#[cfg(feature = "mresult")]
impl std::fmt::Display for ServerError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&format!(
      "Error: `{}` status code\n  Error message: \"{}\"{}",
      self.status_code.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR).as_str(),
      self.decide_public_msg(),
      if let Some(privates) = self.private_msg.as_ref() {
        format!(
          "\n{}",
          privates
            .iter()
            .map(|e| format!("    Caused by: {}", e))
            .collect::<Vec<_>>()
            .join("\n")
        )
      } else {
        String::new()
      }
    ))
  }
}

#[cfg(feature = "mresult")]
impl std::error::Error for ServerError {}

impl std::fmt::Display for ErrorResponse {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&format!("Error: {}", self.err))
  }
}

#[cfg(feature = "mresult")]
impl std::error::Error for ErrorResponse {}

/// Data structure responsible for client errors.
#[cfg(feature = "cresult")]
#[derive(Debug, Clone)]
pub struct CliError {
  /// Error message.
  pub message: String,
}

#[cfg(feature = "cresult")]
impl std::fmt::Display for CliError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.message.as_str())
  }
}

#[cfg(all(feature = "salvo", feature = "mresult"))]
impl crate::responses::ExplicitServerWrite for ServerError {
  async fn explicit_write(self, res: &mut Response) {
    res.status_code(self.status_code.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    tracing::error!("{}", self);

    res
      .write_body(
        serde_json::to_string(&ErrorResponse {
          err: self.decide_public_msg(),
        })
        .unwrap_or(r#"{"err":"Unknown server error"}"#.to_string()),
      )
      .ok();
  }
}

#[cfg(all(feature = "salvo", feature = "mresult"))]
#[salvo::async_trait]
impl ServerResponseWriter for ServerError {
  /// Method for sending an error message to the client.
  async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
    crate::responses::ExplicitServerWrite::explicit_write(self, res).await
  }
}

#[cfg(all(feature = "salvo", feature = "mresult"))]
impl EndpointOutRegister for ServerError {
  /// Registers error types for OpenAPI.
  fn register(components: &mut salvo::oapi::Components, operation: &mut salvo::oapi::Operation) {
    operation.responses.insert(
      "400",
      salvo::oapi::Response::new("Bad request").add_content("application/json", ErrorResponse::to_schema(components)),
    );
    operation.responses.insert(
      "401",
      salvo::oapi::Response::new("Unauthorized").add_content("application/json", ErrorResponse::to_schema(components)),
    );
    operation.responses.insert(
      "403",
      salvo::oapi::Response::new("Forbidden").add_content("application/json", ErrorResponse::to_schema(components)),
    );
    operation.responses.insert(
      "404",
      salvo::oapi::Response::new("Not found").add_content("application/json", ErrorResponse::to_schema(components)),
    );
    operation.responses.insert(
      "405",
      salvo::oapi::Response::new("Method not allowed")
        .add_content("application/json", ErrorResponse::to_schema(components)),
    );
    operation.responses.insert(
      "423",
      salvo::oapi::Response::new("Locked").add_content("application/json", ErrorResponse::to_schema(components)),
    );
    operation.responses.insert(
      "500",
      salvo::oapi::Response::new("Internal server error")
        .add_content("application/json", ErrorResponse::to_schema(components)),
    );
  }
}

#[cfg(feature = "mresult")]
impl ServerError {
  fn decide_public_msg(&self) -> String {
    if let Some(public_msg) = self.public_msg.as_ref() {
      public_msg.to_owned()
    } else {
      match self.status_code {
        Some(StatusCode::BAD_REQUEST) => "Bad request.",
        Some(StatusCode::UNAUTHORIZED) => "Unauthorized request.",
        Some(StatusCode::FORBIDDEN) => "Access denied.",
        Some(StatusCode::NOT_FOUND) => "Page or method not found.",
        Some(StatusCode::METHOD_NOT_ALLOWED) => "Method not allowed.",
        Some(StatusCode::LOCKED) => "Your actions is locked.",
        Some(StatusCode::INTERNAL_SERVER_ERROR) => "Internal server error. Contact the administrator.",
        _ => "Specific error. Check with the administrator for details.",
      }
      .to_string()
    }
  }

  fn format_error(error: &(dyn std::error::Error + 'static)) -> Vec<String> {
    let mut result = vec![];
    let mut current_error: Option<&(dyn std::error::Error + 'static)> = Some(error);

    while let Some(err) = current_error {
      result.push(err.to_string());
      current_error = err.source();
    }

    result
  }

  /// Makes a new ServerError with actual error.
  pub fn from_private(err: impl std::error::Error + 'static) -> Self {
    let err_data = Self::format_error(&err);

    Self {
      #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
      status_code: None,
      public_msg: None,
      private_msg: Some(err_data),
    }
  }

  /// Makes a new ServerError with actual error provided by plain string.
  pub fn from_private_str(err: impl Into<String>) -> Self {
    Self {
      #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
      status_code: None,
      public_msg: None,
      private_msg: Some(vec![err.into()]),
    }
  }

  /// Adds a private message to the ServerError.
  pub fn with_private_str(mut self, new_private_msg: impl Into<String>) -> Self {
    if let Some(privates) = self.private_msg.as_mut() {
      privates.insert(0, new_private_msg.into());
    } else {
      self.private_msg = Some(vec![new_private_msg.into()]);
    }

    self
  }

  /// Adds a public message to the ServerError.
  ///
  /// If ServerError already have a public message, it goes to private messages stack.
  pub fn with_public(mut self, new_public_msg: impl Into<String>) -> Self {
    if let Some(old_public_msg) = self.public_msg.take() {
      if let Some(privates) = self.private_msg.as_mut() {
        privates.insert(0, old_public_msg);
      } else {
        self.private_msg = Some(vec![old_public_msg]);
      }
    }

    self.public_msg = Some(new_public_msg.into());
    self
  }

  /// Makes a new Server Error from public message.
  pub fn from_public(public_msg: impl Into<String>) -> Self {
    Self {
      #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
      status_code: None,
      public_msg: Some(public_msg.into()),
      private_msg: None,
    }
  }

  /// Error BAD REQUEST (400).
  #[cfg(all(feature = "salvo", feature = "mresult"))]
  pub fn with_400(mut self) -> Self {
    self.status_code = Some(StatusCode::BAD_REQUEST);
    self
  }

  /// Error UNAUTHORIZED (401).
  #[cfg(all(feature = "salvo", feature = "mresult"))]
  pub fn with_401(mut self) -> Self {
    self.status_code = Some(StatusCode::UNAUTHORIZED);
    self
  }

  /// Error FORBIDDEN (403).
  #[cfg(all(feature = "salvo", feature = "mresult"))]
  pub fn with_403(mut self) -> Self {
    self.status_code = Some(StatusCode::FORBIDDEN);
    self
  }

  /// Error NOT FOUND (404).
  #[cfg(all(feature = "salvo", feature = "mresult"))]
  pub fn with_404(mut self) -> Self {
    self.status_code = Some(StatusCode::NOT_FOUND);
    self
  }

  /// Error METHOD NOT ALLOWED (405).
  #[cfg(all(feature = "salvo", feature = "mresult"))]
  pub fn with_405(mut self) -> Self {
    self.status_code = Some(StatusCode::METHOD_NOT_ALLOWED);
    self
  }

  /// Error LOCKED (423).
  #[cfg(all(feature = "salvo", feature = "mresult"))]
  pub fn with_423(mut self) -> Self {
    self.status_code = Some(StatusCode::LOCKED);
    self
  }

  /// Error INTERNAL SERVER ERROR (500).
  #[cfg(all(feature = "salvo", feature = "mresult"))]
  pub fn with_500(mut self) -> Self {
    self.status_code = Some(StatusCode::INTERNAL_SERVER_ERROR);
    self
  }

  /// Converts the ServerError into Result::Err.
  pub fn bail<T>(self) -> Result<T, Self> {
    Err(self)
  }
}

#[cfg(feature = "cresult")]
impl From<String> for CliError {
  /// Creates a new error from a string.
  fn from(value: String) -> Self {
    Self { message: value }
  }
}

#[cfg(feature = "cresult")]
impl From<&str> for CliError {
  /// Creates a new error from a string.
  fn from(value: &str) -> Self {
    Self {
      message: value.to_owned(),
    }
  }
}

/// Macro to simplify `ConsiderCli` trait implementation.
macro_rules! impl_consider_cli {
  ($e:ty) => {
    #[cfg(feature = "cresult")]
    impl From<$e> for CliError {
      /// Создаёт `CliError` из данной ошибки.
      fn from(value: $e) -> Self {
        value.to_string().into()
      }
    }
  };
}

impl_consider_cli!(rmp_serde::encode::Error);
impl_consider_cli!(rmp_serde::decode::Error);
impl_consider_cli!(std::io::Error);
impl_consider_cli!(std::string::FromUtf8Error);
impl_consider_cli!(serde_json::Error);

#[cfg(feature = "reqwest")]
impl_consider_cli!(reqwest::Error);
