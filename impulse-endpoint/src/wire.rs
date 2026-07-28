//! Transport-agnostic HTTP wire types.
//!
//! Shared by the client transport (`impulse_client_kit::client`), the Tauri engine
//! (`impulse-tauri-engine`) and the server adapter, so a request/response is the
//! same type whether it is executed via reqwest, forwarded over Tauri IPC, or
//! handled locally. Everything is serialisable so it can cross the IPC boundary
//! unchanged.

use impulse_utils::prelude::{CResult, ClientError};
use serde::{Deserialize, Serialize};

/// HTTP verb. A small serialisable enum so a request can cross the Tauri IPC
/// boundary without pulling a heavier `Method` type into the wire format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
  /// `GET`
  Get,
  /// `POST`
  Post,
  /// `PUT`
  Put,
  /// `PATCH`
  Patch,
  /// `DELETE`
  Delete,
  /// `HEAD`
  Head,
}

impl Method {
  /// The uppercase method name.
  pub fn as_str(&self) -> &'static str {
    match self {
      Method::Get => "GET",
      Method::Post => "POST",
      Method::Put => "PUT",
      Method::Patch => "PATCH",
      Method::Delete => "DELETE",
      Method::Head => "HEAD",
    }
  }

  /// Whether this is a safe/read method (`GET`/`HEAD`) — e.g. cacheable, and not
  /// queued for offline replay by the engine.
  pub fn is_read(&self) -> bool {
    matches!(self, Method::Get | Method::Head)
  }
}

#[cfg(feature = "reqwest")]
impl From<Method> for reqwest::Method {
  fn from(m: Method) -> Self {
    match m {
      Method::Get => reqwest::Method::GET,
      Method::Post => reqwest::Method::POST,
      Method::Put => reqwest::Method::PUT,
      Method::Patch => reqwest::Method::PATCH,
      Method::Delete => reqwest::Method::DELETE,
      Method::Head => reqwest::Method::HEAD,
    }
  }
}

/// Response header an offline-capable client engine (`impulse-tauri-engine`)
/// stamps on every response it produced **without reaching the server** — both a
/// locally-served success and a "not available offline" error.
///
/// It is the wire form of a *no-connection* state, which a plain status code
/// can't express: a `401` from the server means the session is genuinely
/// rejected, while a `503` carrying this header means nobody was asked. Callers
/// that must tell the two apart — an auth gate deciding between "sign in again"
/// and "keep working from the local copy" — check
/// [`HttpResponse::is_offline`].
pub const OFFLINE_HEADER: &str = "x-ik-offline";

/// A fully-described request. Serialisable so it can be executed in-process
/// (reqwest) or across Tauri IPC by the native engine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpRequest {
  /// HTTP verb.
  pub method: Method,
  /// Absolute or app-relative URL (already resolved via `router::endpoint`).
  pub url: String,
  /// Header name/value pairs, applied in order.
  pub headers: Vec<(String, String)>,
  /// Raw request body, if any.
  pub body: Option<Vec<u8>>,
  /// Whether ambient credentials should ride along: on wasm this sets fetch
  /// `credentials: include` (cookies); the engine reads it to decide whether to
  /// attach the stored token. Auth material itself is added by an interceptor.
  pub credentials: bool,
}

/// A collected response: status, headers and the full body buffered in memory.
/// Buffering keeps the type serialisable across IPC and identical in both modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpResponse {
  /// HTTP status code.
  pub status: u16,
  /// Response header name/value pairs.
  pub headers: Vec<(String, String)>,
  /// Raw response body.
  pub body: Vec<u8>,
}

impl HttpResponse {
  /// The HTTP status code.
  pub fn status(&self) -> u16 {
    self.status
  }

  /// `true` for a 2xx status.
  pub fn is_success(&self) -> bool {
    (200..300).contains(&self.status)
  }

  /// First value of a response header, case-insensitively.
  pub fn header(&self, name: &str) -> Option<&str> {
    self
      .headers
      .iter()
      .find(|(k, _)| k.eq_ignore_ascii_case(name))
      .map(|(_, v)| v.as_str())
  }

  /// Whether this response was produced without reaching the server — i.e. it
  /// carries [`OFFLINE_HEADER`]. `false` for anything that came off the wire, so
  /// a server's own `401`/`503` is never mistaken for a lost connection.
  pub fn is_offline(&self) -> bool {
    self.header(OFFLINE_HEADER).is_some()
  }

  /// The body as UTF-8 text.
  pub fn text(&self) -> CResult<String> {
    String::from_utf8(self.body.clone()).map_err(ClientError::from)
  }

  /// The raw body bytes.
  pub fn bytes(self) -> Vec<u8> {
    self.body
  }

  /// Decode a JSON body.
  pub fn json<T: serde::de::DeserializeOwned>(&self) -> CResult<T> {
    serde_json::from_slice(&self.body).map_err(ClientError::from)
  }

  /// Decode a MessagePack body.
  pub fn msgpack<T: serde::de::DeserializeOwned>(&self) -> CResult<T> {
    rmp_serde::from_slice(&self.body).map_err(ClientError::from)
  }

  /// Turns a non-2xx response into an `Err`, otherwise passes it through.
  pub fn error_for_status(self) -> CResult<Self> {
    if self.is_success() {
      Ok(self)
    } else {
      Err(ClientError::from_str(format!("HTTP {}", self.status)))
    }
  }
}
