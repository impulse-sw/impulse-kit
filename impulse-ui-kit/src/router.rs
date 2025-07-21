//! Methods to work with routes on SPA.

use impulse_utils::prelude::*;

/// Get server's address and port
pub fn get_host() -> CResult<String> {
  let server_host = web_sys::window()
    .ok_or(CliError::from_str("Can't get browser's window parameters."))?
    .document()
    .ok_or(CliError::from_str("Can't get window's document."))?
    .location()
    .ok_or(CliError::from_str("Can't get document's location."))?
    .host()
    .map_err(|e| CliError::from_str(format!("Can't get host: {e:?}")))?
    .to_string();
  Ok(server_host)
}

/// Get server protocol (HTTP/HTTPS: "http:"/"https:")
pub fn get_protocol() -> CResult<String> {
  let server_proto = web_sys::window()
    .ok_or(CliError::from_str("Can't get browser's window parameters."))?
    .document()
    .ok_or(CliError::from_str("Can't get window's document."))?
    .location()
    .ok_or(CliError::from_str("Can't get document's location."))?
    .protocol()
    .map_err(|e| CliError::from_str(format!("Can't get protocol: {e:?}")))?
    .to_string();
  Ok(server_proto)
}

/// Get path
pub fn get_path() -> CResult<String> {
  let path = web_sys::window()
    .ok_or(CliError::from_str("Can't get browser's window parameters."))?
    .document()
    .ok_or(CliError::from_str("Can't get window's document."))?
    .location()
    .ok_or(CliError::from_str("Can't get document's location."))?
    .pathname()
    .map_err(|e| CliError::from_str(format!("Can't get pathname: {e:?}")))?
    .to_string();
  Ok(path)
}

/// Redirect to any URL
pub fn redirect(url: impl AsRef<str>) -> CResult<()> {
  web_sys::window()
    .ok_or(CliError::from_str("Can't get browser's window parameters."))?
    .document()
    .ok_or(CliError::from_str("Can't get window's document."))?
    .location()
    .ok_or(CliError::from_str("Can't get document's location."))?
    .set_href(url.as_ref())
    .map_err(|e| CliError::from_str(format!("Can't redirect: {e:?}")))
}

/// Get endpoint to your backend server
///
/// Example:
///
/// ```rust,ignore
/// use impulse_ui_kit::router::endpoint;
///
/// fn main() {
///   // Let assume that your backend is located at `127.0.0.1:8080` with HTTP schema
///   assert_eq!(endpoint("/some/api/route").as_str(), "http://127.0.0.1:8080/some/api/route");
/// }
/// ```
pub fn endpoint(api_uri: impl AsRef<str>) -> String {
  format!(
    "{}//{}{}",
    get_protocol().unwrap(),
    get_host().unwrap(),
    api_uri.as_ref()
  )
}
