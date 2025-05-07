//! Implementation of utilities for working with MessagePack with requests in `salvo` and `reqwest`.

#[cfg(feature = "salvo")]
use crate::prelude::*;

#[cfg(feature = "salvo")]
use serde::Deserialize;

#[cfg(feature = "salvo")]
use salvo::Request;

/// MessagePack parser from `salvo::Request`.
#[cfg(feature = "salvo")]
#[allow(async_fn_in_trait)]
pub trait MsgPackParser {
  /// Parses `msgpack` body.
  async fn parse_msgpack<'de, T: Deserialize<'de>>(&'de mut self) -> MResult<T>;
  /// Parses `msgpack` body with size limit.
  async fn parse_msgpack_with_max_size<'de, T: Deserialize<'de>>(&'de mut self, max_size: usize) -> MResult<T>;
}

#[cfg(feature = "salvo")]
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
      tracing::debug!("{:?}", payload);
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
