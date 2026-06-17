//! Cookie-based theme detection on the server side.

use salvo::http::HeaderMap;

/// Cookie name used to persist the theme between requests.
pub const THEME_COOKIE_KEY: &str = "impulse_theme";

/// Initial theme resolved from the request, mirrored from
/// `impulse-client-kit::ssr::InitialTheme`. The value is provided as a Leptos
/// context so user components can read the current theme during SSR.
#[derive(Clone, Debug, Default)]
pub struct InitialTheme(pub Option<String>);

/// Parse the `impulse_theme` cookie out of a request's header map.
///
/// Returns `None` when the cookie is absent or the header is malformed. The
/// caller decides what default to apply.
pub fn parse_theme_cookie(headers: &HeaderMap) -> Option<String> {
  let raw = headers.get("cookie")?.to_str().ok()?;
  for piece in raw.split(';') {
    let piece = piece.trim();
    let Some((name, value)) = piece.split_once('=') else {
      continue;
    };
    if name == THEME_COOKIE_KEY {
      return Some(value.to_string());
    }
  }
  None
}
