//! Implementation of utilities for working with responses in `salvo` and `reqwest`.

#[cfg(feature = "salvo")]
use salvo::http::HeaderValue;

#[cfg(feature = "salvo")]
use salvo::hyper::header::CONTENT_TYPE;

#[cfg(feature = "salvo")]
use salvo::oapi::{EndpointOutRegister, ToSchema};

#[cfg(feature = "salvo")]
use salvo::{Depot, Request, Response};

#[cfg(feature = "salvo")]
use salvo::Writer as ServerResponseWriter;

#[cfg(feature = "salvo")]
use salvo::fs::NamedFile;

/// Macro to define the function that called the response.
#[macro_export]
macro_rules! fn_name {
  () => {{
    fn f() {}
    fn type_name_of<T>(_: T) -> &'static str {
      std::any::type_name::<T>()
    }
    let name = type_name_of(f);

    // For `#[endpoint]` path can be shortened as follows:
    match name[..name.len() - 3].rsplit("::").nth(2) {
      Some(el) => el,
      None => &name[..name.len() - 3],
    }
  }};
}

/// Macro for automating `EndpointOutRegister` implementations (for simple types)
#[cfg(feature = "salvo")]
macro_rules! impl_oapi_endpoint_out {
  ($t:tt, $c:expr) => {
    #[cfg(feature = "salvo")]
    impl EndpointOutRegister for $t {
      #[inline]
      fn register(components: &mut salvo::oapi::Components, operation: &mut salvo::oapi::Operation) {
        operation.responses.insert(
          "200",
          salvo::oapi::Response::new("Ok").add_content($c, String::to_schema(components)),
        );
      }
    }
  };
}

/// Macro for automating `EndpointOutRegister` implementations (for template types)
#[cfg(feature = "salvo")]
macro_rules! impl_oapi_endpoint_out_t {
  ($t:tt, $c:expr) => {
    #[cfg(feature = "salvo")]
    impl<T> EndpointOutRegister for $t<T> {
      #[inline]
      fn register(components: &mut salvo::oapi::Components, operation: &mut salvo::oapi::Operation) {
        operation.responses.insert(
          "200",
          salvo::oapi::Response::new("Ok").add_content($c, String::to_schema(components)),
        );
      }
    }
  };
}

/// Sends 200 without data.
#[cfg(feature = "salvo")]
pub struct OK(pub &'static str);

#[cfg(feature = "salvo")]
impl_oapi_endpoint_out!(OK, "text/plain");

/// Returns empty `200 OK` response.
#[cfg(feature = "salvo")]
#[macro_export]
macro_rules! ok {
  () => {
    Ok::<cc_utils::responses::OK, cc_utils::errors::ErrorResponse>(cc_utils::responses::OK($crate::fn_name!()))
  };
}

#[cfg(feature = "salvo")]
#[salvo::async_trait]
impl ServerResponseWriter for OK {
  async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
    res.status_code(salvo::http::StatusCode::OK);
    res.render("");
    tracing::trace!("[{}] => Received and sent result 200", self.0);
  }
}

/// Sends 200 and plain text.
#[cfg(feature = "salvo")]
#[derive(Debug)]
pub struct Plain(pub String, pub &'static str);

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
impl_oapi_endpoint_out!(Plain, "text/plain");

/// Returns given plain text.
#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[macro_export]
macro_rules! plain {
  ($plain_text:expr) => {
    Ok::<cc_utils::responses::Plain, cc_utils::errors::ErrorResponse>(cc_utils::responses::Plain(
      $plain_text,
      $crate::fn_name!(),
    ))
  };
}

#[cfg(feature = "salvo")]
#[salvo::async_trait]
impl ServerResponseWriter for Plain {
  async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
    res.status_code(salvo::http::StatusCode::OK);
    res.render(&self.0);
    tracing::trace!("[{}] => Received and sent result 200 with text: {}", self.1, self.0);
  }
}

/// Sends 200 and HTML.
#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[derive(Debug)]
pub struct Html(pub String, pub &'static str);

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
impl_oapi_endpoint_out!(Html, "text/html");

/// Returns given HTML code.
#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[macro_export]
macro_rules! html {
  ($html_data:expr) => {
    Ok::<cc_utils::responses::Html, cc_utils::errors::ErrorResponse>(cc_utils::responses::Html(
      $html_data,
      $crate::fn_name!(),
    ))
  };
}

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[salvo::async_trait]
impl ServerResponseWriter for Html {
  async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
    res.status_code(salvo::http::StatusCode::OK);
    res.render(salvo::writing::Text::Html(&self.0));
    tracing::trace!("[{}] => Received and sent result 200 with HTML", self.1);
  }
}

/// Sends 200 and file.
#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[derive(Debug)]
pub struct File(pub std::path::PathBuf, pub String, pub &'static str);

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
impl_oapi_endpoint_out!(File, "application/octet-stream");

/// File response.
///
/// Usage:
///
/// ```rust
/// use cc_utils::prelude::*;
/// use salvo::prelude::*;
/// use std::path::PathBuf;
///
/// pub async fn some_endpoint() -> MResult<File> {
///   file_upload!(PathBuf::from("filepath.txt"), "Normal file name.txt".to_string())
/// }
/// ```
#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[macro_export]
macro_rules! file_upload {
  ($filepath:expr, $attached_filename:expr) => {
    Ok::<cc_utils::responses::File, cc_utils::errors::ErrorResponse>(cc_utils::responses::File(
      $filepath,
      $attached_filename,
      $crate::fn_name!(),
    ))
  };
}

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[salvo::async_trait]
impl ServerResponseWriter for File {
  async fn write(self, req: &mut Request, _depot: &mut Depot, res: &mut Response) {
    res.status_code(salvo::http::StatusCode::OK);
    res.headers_mut().append(
      "Cache-Control",
      HeaderValue::from_static("public, max-age=0, must-revalidate"),
    );
    NamedFile::builder(&self.0)
      .attached_name(&self.1)
      .use_etag(true)
      .use_last_modified(true)
      .send(req.headers(), res)
      .await;
    tracing::trace!("[{}] => Received and sent result 200 with file {}", self.2, self.1);
  }
}

/// Sends 200 and JSON.
#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[derive(Debug)]
pub struct Json<T>(pub T, pub &'static str);

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
impl_oapi_endpoint_out_t!(Json, "application/json");

/// Serializes to JSON and returns given object.
#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[macro_export]
macro_rules! json {
  ($json_data:expr) => {
    Ok::<cc_utils::responses::Json<_>, cc_utils::errors::ErrorResponse>(cc_utils::responses::Json(
      $json_data,
      $crate::fn_name!(),
    ))
  };
}

#[cfg(all(feature = "salvo", feature = "mresult"))]
#[salvo::async_trait]
impl<T: serde::Serialize + Send> ServerResponseWriter for Json<T> {
  async fn write(self, req: &mut Request, depot: &mut Depot, res: &mut Response) {
    res.status_code(salvo::http::StatusCode::OK);
    match serde_json::to_string(&self.0) {
      Ok(s) => {
        res.headers_mut().insert(
          CONTENT_TYPE,
          HeaderValue::from_static("application/json; charset=utf-8"),
        );
        tracing::trace!("[{}] => Sending JSON: {:?}", self.1, s.as_str());
        res.write_body(s).ok();
        tracing::trace!("[{}] => Received and sent result 200 with JSON", self.1);
      }
      Err(e) => {
        tracing::error!("[{}] => Failed to serialize data: {:?}", e, self.1);
        crate::prelude::ErrorResponse::from("Failed to serialize data.")
          .with_500()
          .build()
          .write(req, depot, res)
          .await;
      }
    }
  }
}

/// Sends 200 and MsgPack.
#[cfg(feature = "salvo")]
#[derive(Debug)]
pub struct MsgPack<T>(pub T, pub &'static str);

#[cfg(feature = "salvo")]
impl_oapi_endpoint_out_t!(MsgPack, "application/msgpack");

/// Serializes to MsgPack and returns given object.
#[cfg(feature = "salvo")]
#[macro_export]
macro_rules! msgpack {
  ($msgpack_data:expr) => {
    Ok::<cc_utils::responses::MsgPack<_>, cc_utils::errors::ErrorResponse>(cc_utils::responses::MsgPack(
      $msgpack_data,
      $crate::fn_name!(),
    ))
  };
}

#[cfg(all(feature = "salvo", feature = "mresult"))]
#[salvo::async_trait]
impl<T: serde::Serialize + Send> ServerResponseWriter for MsgPack<T> {
  async fn write(self, req: &mut Request, depot: &mut Depot, res: &mut Response) {
    res.status_code(salvo::http::StatusCode::OK);
    match rmp_serde::to_vec(&self.0) {
      Ok(bytes) => {
        res.headers_mut().insert(
          CONTENT_TYPE,
          HeaderValue::from_static("application/msgpack; charset=utf-8"),
        );
        tracing::trace!("[{}] => Sending bytes: {:04X?}", self.1, bytes);
        res.write_body(bytes).ok();
        tracing::trace!("[{}] => Received and sent result 200 with MsgPack", self.1);
      }
      Err(e) => {
        tracing::error!("[{}] => Failed to serialize data: {:?}", e, self.1);
        crate::prelude::ErrorResponse::from("Failed to serialize data.")
          .with_500()
          .build()
          .write(req, depot, res)
          .await;
      }
    }
  }
}

/// Trait to parse MessagePack responses from `reqwest` library.
#[cfg(all(feature = "reqwest", feature = "cresult"))]
#[allow(async_fn_in_trait)]
pub trait MsgPackResponse {
  /// Parses MessagePack from body.
  async fn msgpack<T: serde::de::DeserializeOwned>(self) -> crate::prelude::CResult<T>;
}

#[cfg(all(feature = "reqwest", feature = "cresult"))]
impl MsgPackResponse for reqwest::Response {
  async fn msgpack<T: serde::de::DeserializeOwned>(self) -> crate::prelude::CResult<T> {
    use crate::errors::ConsiderCli;

    let full = self.bytes().await?;
    rmp_serde::from_slice(&full).consider_cli(None)
  }
}
