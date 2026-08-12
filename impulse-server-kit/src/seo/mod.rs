//! What crawlers are told about a site: `robots.txt` and `sitemap.xml`.
//!
//! Both files are built here rather than dropped into the static directory,
//! because both of them have to know things a file on disk does not: which of
//! an application's routes are meant to be public, what it has published since
//! it started, and — the one that trips up every static `robots.txt` — the
//! origin it is being served on.
//!
//! ```rust,ignore
//! use impulse_server_kit::prelude::*;
//!
//! let robots = RobotsTxt::new()
//!   .comment("Only /p/ is meant to be crawled.")
//!   .disallow("/s/")
//!   .disallow("/api/")
//!   .sitemap("/sitemap.xml");   // resolved against the request's origin
//!
//! let router = get_root_router(&state)
//!   .push(robots.into_router())
//!   .push(Router::with_path(SITEMAP_XML_PATH).get(sitemap));
//!
//! // A sitemap whose contents change writes itself per request; `Sitemap` is a
//! // salvo `Writer`, so a handler is all it takes.
//! #[handler]
//! async fn sitemap(depot: &mut Depot, req: &mut Request) -> MResult<Sitemap> {
//!   let origin = request_origin(req);
//!   Ok(
//!     Sitemap::new().urls(
//!       published_slugs(depot)
//!         .await?
//!         .into_iter()
//!         .map(|slug| SitemapUrl::new(format!("{origin}/p/{slug}"))),
//!     ),
//!   )
//! }
//! ```
//!
//! The two halves of keeping something *out* of an index live here too, and
//! they are not interchangeable: [`RobotsTxt`] is the only one that stops a
//! crawler from *fetching* a URL, and [`set_x_robots_tag`] is the only one that
//! binds a crawler which fetched anyway — because it ignored `robots.txt`, or
//! because it was handed the URL directly rather than finding it. Most sites
//! want both, and should know that pairing them has a documented cost: a URL
//! disallowed in `robots.txt` can still be listed bare, without a snippet, when
//! something links to it, precisely because the crawler never fetched it and so
//! never saw the `noindex`.

mod robots;
mod sitemap;

pub use robots::{ROBOTS_TXT_PATH, RobotsGroup, RobotsTag, RobotsTxt, RobotsTxtHandler, set_x_robots_tag};
pub use sitemap::{ChangeFreq, MAX_SITEMAP_URLS, SITEMAP_XML_PATH, Sitemap, SitemapHandler, SitemapUrl};

use salvo::Request;
use salvo::http::header::HOST;

/// The origin — `scheme://host` — the request arrived on.
///
/// Both files this module builds are required to carry absolute URLs, and an
/// application that is not told its own address in configuration has exactly one
/// place to learn it: the request in front of it.
///
/// Behind a reverse proxy that terminates TLS, the scheme is knowable *only*
/// from `X-Forwarded-Proto` — the connection this server accepted is plain HTTP,
/// and taking that as the truth publishes `http://` URLs for an `https://` site,
/// which a crawler files away as a second, duplicate host. `X-Forwarded-Host` is
/// read for the same reason, falling back to `Host`. Both headers list proxies
/// left to right, so the first value is the one the client actually spoke to.
///
/// **Trust:** these headers are whatever the client sent unless a proxy
/// overwrites them, so a server exposed directly to the internet can be told any
/// origin at all. That is survivable for what this module does — a poisoned
/// `robots.txt` or `sitemap.xml` is served to whoever poisoned it, and nobody
/// else — but do not reach for this to build anything a *different* user will
/// follow, such as a link in an e-mail.
pub fn request_origin(req: &Request) -> String {
  let scheme = req
    .uri()
    .scheme_str()
    .map(str::to_string)
    .or_else(|| forwarded(req, "x-forwarded-proto"))
    .unwrap_or_else(|| "http".to_string());
  let host = req
    .uri()
    .host()
    .map(str::to_string)
    .or_else(|| forwarded(req, "x-forwarded-host"))
    .or_else(|| forwarded(req, HOST.as_str()))
    .unwrap_or_default();
  format!("{scheme}://{host}")
}

/// The first entry of a comma-separated forwarding header, if it looks like one
/// value rather than something to paste into a URL unexamined.
fn forwarded(req: &Request, header: &str) -> Option<String> {
  let raw = req.headers().get(header)?.to_str().ok()?;
  let first = raw.split(',').next().unwrap_or(raw).trim();
  let usable = !first.is_empty()
    && !first
      .chars()
      .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '/' | '"' | '\'' | '<' | '>'));
  usable.then(|| first.to_string())
}

/// Escapes the five XML entities. Shared by [`Sitemap`] and used on everything
/// that reaches the document, because a `&` in a URL is ordinary and an
/// unescaped one makes the file unparseable — which a crawler reports as "no
/// sitemap" rather than "broken sitemap".
fn xml_escape(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    match c {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&apos;"),
      // Control characters are not representable in XML 1.0 at all, so they are
      // dropped rather than escaped into something equally unparseable.
      c if c.is_control() && c != '\t' && c != '\n' && c != '\r' => {}
      c => out.push(c),
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req_with(headers: &[(&str, &str)]) -> Request {
    let mut req = Request::default();
    for (name, value) in headers {
      req.headers_mut().insert(
        salvo::http::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
        value.parse().unwrap(),
      );
    }
    req
  }

  /// The case every deployment behind a TLS-terminating proxy hits: the
  /// connection this server accepted is plain HTTP, and believing it would put
  /// `http://` URLs in the sitemap of an `https://` site.
  #[test]
  fn takes_the_scheme_from_the_proxy() {
    let req = req_with(&[("host", "example.com"), ("x-forwarded-proto", "https")]);
    assert_eq!(request_origin(&req), "https://example.com");
  }

  /// Forwarding headers accumulate one entry per proxy, and the client spoke to
  /// the first.
  #[test]
  fn reads_the_first_hop_of_a_chain() {
    let req = req_with(&[
      ("host", "internal:8080"),
      ("x-forwarded-proto", "https, http"),
      ("x-forwarded-host", "example.com, internal:8080"),
    ]);
    assert_eq!(request_origin(&req), "https://example.com");
  }

  /// A junk forwarding header falls back to `Host` instead of being pasted into
  /// a URL — a value with a slash or a space in it is not a host, and whatever
  /// it is, it does not belong in the middle of one.
  #[test]
  fn ignores_a_forwarded_value_that_is_not_a_host() {
    let req = req_with(&[("host", "example.com"), ("x-forwarded-host", "evil.example/../")]);
    assert_eq!(request_origin(&req), "http://example.com");
  }

  #[test]
  fn escapes_what_would_break_the_document() {
    assert_eq!(
      xml_escape("/p/a?x=1&y=2<z>"),
      "/p/a?x=1&amp;y=2&lt;z&gt;",
      "an ampersand in a URL is ordinary and must not end the document"
    );
  }
}
