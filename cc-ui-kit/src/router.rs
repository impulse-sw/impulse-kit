//! Methods to work with routes on SPA.

use cc_utils::results::CResult;

/// Get server's address and port
pub fn get_host() -> CResult<String> {
  let server_host = web_sys::window()
    .ok_or::<String>("Can't get browser's window parameters.".into())?
    .document()
    .ok_or::<String>("Can't get window's document.".into())?
    .location()
    .ok_or::<String>("Can't get document's location.".into())?
    .host()
    .map_err(|e| format!("Can't get host: {:?}", e))?
    .to_string();
  Ok(server_host)
}

/// Get server protocol (HTTP/HTTPS: "http:"/"https:")
pub fn get_protocol() -> CResult<String> {
  let server_proto = web_sys::window()
    .ok_or::<String>("Can't get browser's window parameters.".into())?
    .document()
    .ok_or::<String>("Can't get window's document.".into())?
    .location()
    .ok_or::<String>("Can't get document's location.".into())?
    .protocol()
    .map_err(|e| format!("Can't get protocol: {:?}", e))?
    .to_string();
  Ok(server_proto)
}

/// Get path
pub fn get_path() -> CResult<String> {
  let path = web_sys::window()
    .ok_or::<String>("Can't get browser's window parameters.".into())?
    .document()
    .ok_or::<String>("Can't get window's document.".into())?
    .location()
    .ok_or::<String>("Can't get document's location.".into())?
    .pathname()
    .map_err(|e| format!("Can't get path: {:?}", e))?
    .to_string();
  Ok(path)
}

/// Redirect to any uri
pub fn redirect(uri: String) -> CResult<()> {
  web_sys::window()
    .ok_or::<String>("Can't get browser's window parameters.".into())?
    .document()
    .ok_or::<String>("Can't get window's document.".into())?
    .location()
    .ok_or::<String>("Can't get document's location.".into())?
    .set_href(uri.as_str())
    .map_err(|e| format!("Can't redirect: {:?}", e))?;
  Ok(())
}
