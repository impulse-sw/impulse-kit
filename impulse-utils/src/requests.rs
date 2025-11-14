//! Implementation of utilities for working with MessagePack with requests in `salvo` and `reqwest`.

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
use crate::prelude::*;

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
use serde::Deserialize;

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
use salvo::Request;

/// MessagePack parser from `salvo::Request`.
#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[allow(async_fn_in_trait)]
pub trait MsgPackParser {
  /// Parses `msgpack` body.
  async fn parse_msgpack<'de, T: Deserialize<'de>>(&'de mut self) -> MResult<T>;
  /// Parses `msgpack` body with size limit.
  async fn parse_msgpack_with_max_size<'de, T: Deserialize<'de>>(&'de mut self, max_size: usize) -> MResult<T>;
}

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
impl MsgPackParser for Request {
  /// Parse MessagePack body as type `T` from request with default max size limit.
  #[inline]
  async fn parse_msgpack<'de, T: Deserialize<'de>>(&'de mut self) -> MResult<T> {
    self
      .parse_msgpack_with_max_size(salvo::http::request::global_secure_max_size())
      .await
  }

  /// Parse MessagePack body as type `T` from request with max size limit.
  #[inline]
  async fn parse_msgpack_with_max_size<'de, T: Deserialize<'de>>(&'de mut self, max_size: usize) -> MResult<T> {
    let ctype = self.content_type();
    if ctype.is_some_and(|ct| ct.subtype() == salvo::http::mime::MSGPACK) {
      let payload = self.payload_with_max_size(max_size).await.map_err(|e| {
        ServerError::from_private(e)
          .with_public("Payload parse error")
          .with_400()
      })?;
      let payload = if payload.is_empty() {
        "null".as_bytes()
      } else {
        payload.as_ref()
      };
      tracing::trace!("Got payload: {:?}", payload);
      rmp_serde::from_slice::<T>(payload).map_err(|e| {
        ServerError::from_private(e)
          .with_public("Payload parse error")
          .with_400()
      })
    } else {
      Err(ServerError::from_public("Bad content type, must be `application/msgpack`.").with_400())
    }
  }
}

/// JSON parser from `salvo::Request` by `sonic-rs`.
#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[allow(async_fn_in_trait)]
pub trait SimdJsonParser {
  /// Parses `json` body.
  ///
  /// To use actual SIMD parsing, compile your backend with option `-C target-cpu=native`.
  async fn parse_json_simd<'de, T: Deserialize<'de>>(&'de mut self) -> MResult<T>;
  /// Parses `json` body with size limit.
  ///
  /// To use actual SIMD parsing, compile your backend with option `-C target-cpu=native`.
  async fn parse_json_simd_with_max_size<'de, T: Deserialize<'de>>(&'de mut self, max_size: usize) -> MResult<T>;
}

#[cfg(feature = "salvo")]
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
impl SimdJsonParser for Request {
  #[inline]
  async fn parse_json_simd<'de, T: Deserialize<'de>>(&'de mut self) -> MResult<T> {
    self
      .parse_json_simd_with_max_size(salvo::http::request::global_secure_max_size())
      .await
  }

  #[inline]
  async fn parse_json_simd_with_max_size<'de, T: Deserialize<'de>>(&'de mut self, max_size: usize) -> MResult<T> {
    let ctype = self.content_type();
    if ctype.is_some_and(|ct| ct.subtype() == salvo::http::mime::JSON) {
      let payload = self.payload_with_max_size(max_size).await.map_err(|e| {
        ServerError::from_private(e)
          .with_public("Payload parse error")
          .with_400()
      })?;
      let payload = if payload.is_empty() {
        "null".as_bytes()
      } else {
        payload.as_ref()
      };
      tracing::trace!("Got payload: {:?}", payload);
      sonic_rs::from_slice::<T>(payload).map_err(|e| {
        ServerError::from_private(e)
          .with_public("Payload parse error")
          .with_400()
      })
    } else {
      Err(ServerError::from_public("Bad content type, must be `application/json`.").with_400())
    }
  }
}

/// MessagePack writer to `reqwest::RequestBuilder`.
#[cfg(all(feature = "reqwest", feature = "cresult"))]
pub trait MsgPackRequest {
  /// Writes `T` as MsgPack binary data into RequestBuilder.
  fn msgpack<T: serde::Serialize + ?Sized>(self, msgpack: &T) -> crate::prelude::CResult<Self>
  where
    Self: Sized;
}

#[cfg(all(feature = "reqwest", feature = "cresult"))]
impl MsgPackRequest for reqwest::RequestBuilder {
  fn msgpack<T: serde::Serialize + ?Sized>(self, msgpack: &T) -> crate::prelude::CResult<Self> {
    use crate::errors::ClientError;

    Ok(
      self
        .header(reqwest::header::CONTENT_TYPE, "application/msgpack")
        .body(rmp_serde::to_vec(msgpack).map_err(|e| ClientError::from_str(format!("Can't serialize body: {e:?}")))?),
    )
  }
}
