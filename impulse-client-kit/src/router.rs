//! Methods to work with routes.
//!
//! On the client (`csr`/`hydrate`) these helpers query the browser's
//! `window.location`. On the server (`ssr`) they read from the per-request
//! `impulse_client_kit::ssr::RequestUrlCtx` context populated by the SSR handler.

use impulse_utils::prelude::*;

/// Get server's address and port.
#[cfg(any(feature = "csr", feature = "hydrate"))]
pub fn get_host() -> CResult<String> {
  let server_host = web_sys::window()
    .ok_or(ClientError::from_str("Can't get browser's window parameters."))?
    .document()
    .ok_or(ClientError::from_str("Can't get window's document."))?
    .location()
    .ok_or(ClientError::from_str("Can't get document's location."))?
    .host()
    .map_err(|e| ClientError::from_str(format!("Can't get host: {e:?}")))?
    .to_string();
  Ok(server_host)
}

/// Get server protocol (HTTP/HTTPS: "http:"/"https:").
#[cfg(any(feature = "csr", feature = "hydrate"))]
pub fn get_protocol() -> CResult<String> {
  let server_proto = web_sys::window()
    .ok_or(ClientError::from_str("Can't get browser's window parameters."))?
    .document()
    .ok_or(ClientError::from_str("Can't get window's document."))?
    .location()
    .ok_or(ClientError::from_str("Can't get document's location."))?
    .protocol()
    .map_err(|e| ClientError::from_str(format!("Can't get protocol: {e:?}")))?
    .to_string();
  Ok(server_proto)
}

/// Get path.
#[cfg(any(feature = "csr", feature = "hydrate"))]
pub fn get_path() -> CResult<String> {
  let path = web_sys::window()
    .ok_or(ClientError::from_str("Can't get browser's window parameters."))?
    .document()
    .ok_or(ClientError::from_str("Can't get window's document."))?
    .location()
    .ok_or(ClientError::from_str("Can't get document's location."))?
    .pathname()
    .map_err(|e| ClientError::from_str(format!("Can't get pathname: {e:?}")))?
    .to_string();
  Ok(path)
}

/// Redirect to any URL.
#[cfg(any(feature = "csr", feature = "hydrate"))]
pub fn redirect(url: impl AsRef<str>) -> CResult<()> {
  web_sys::window()
    .ok_or(ClientError::from_str("Can't get browser's window parameters."))?
    .document()
    .ok_or(ClientError::from_str("Can't get window's document."))?
    .location()
    .ok_or(ClientError::from_str("Can't get document's location."))?
    .set_href(url.as_ref())
    .map_err(|e| ClientError::from_str(format!("Can't redirect: {e:?}")))
}

/// Get endpoint to your backend server.
///
/// Example:
///
/// ```rust,ignore
/// use impulse_client_kit::router::endpoint;
///
/// fn main() {
///   // Let assume that your backend is located at `127.0.0.1:8080` with HTTP schema
///   assert_eq!(endpoint("/some/api/route").as_str(), "http://127.0.0.1:8080/some/api/route");
/// }
/// ```
#[cfg(any(feature = "csr", feature = "hydrate"))]
pub fn endpoint(api_uri: impl AsRef<str>) -> String {
  format!(
    "{}//{}{}",
    get_protocol().unwrap(),
    get_host().unwrap(),
    api_uri.as_ref()
  )
}

/// Get server's host (e.g. "127.0.0.1:8801") from the per-request SSR context.
#[cfg(feature = "ssr")]
pub fn get_host() -> CResult<String> {
  use leptos::context::use_context;
  let ctx = use_context::<crate::ssr::RequestUrlCtx>()
    .ok_or(ClientError::from_str("RequestUrlCtx not provided in SSR context"))?;
  Ok(ctx.host)
}

/// Get protocol scheme with trailing colon ("http:" / "https:") from the SSR context.
#[cfg(feature = "ssr")]
pub fn get_protocol() -> CResult<String> {
  use leptos::context::use_context;
  let ctx = use_context::<crate::ssr::RequestUrlCtx>()
    .ok_or(ClientError::from_str("RequestUrlCtx not provided in SSR context"))?;
  Ok(format!("{}:", ctx.scheme))
}

/// Get the requested path from the SSR context.
#[cfg(feature = "ssr")]
pub fn get_path() -> CResult<String> {
  use leptos::context::use_context;
  let ctx = use_context::<crate::ssr::RequestUrlCtx>()
    .ok_or(ClientError::from_str("RequestUrlCtx not provided in SSR context"))?;
  Ok(ctx.path)
}

/// On SSR, "redirect" stores the redirect target on [`crate::ssr::LeptosResponseOptions`].
/// The SSR handler reads it after rendering and short-circuits the response.
#[cfg(feature = "ssr")]
pub fn redirect(url: impl AsRef<str>) -> CResult<()> {
  use leptos::context::use_context;
  let opts = use_context::<crate::ssr::LeptosResponseOptions>().ok_or(ClientError::from_str(
    "LeptosResponseOptions not provided in SSR context",
  ))?;
  opts.set_redirect(url.as_ref().to_string());
  Ok(())
}

/// Get endpoint to your backend server (SSR variant).
#[cfg(feature = "ssr")]
pub fn endpoint(api_uri: impl AsRef<str>) -> String {
  format!(
    "{}//{}{}",
    get_protocol().unwrap_or_else(|_| "http:".to_string()),
    get_host().unwrap_or_default(),
    api_uri.as_ref()
  )
}
