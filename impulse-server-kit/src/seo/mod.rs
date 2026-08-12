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
//! An application answering on more than one hostname — a product domain and a
//! vanity one, an apex and its `www` — needs [`CanonicalOrigin`], which is the
//! difference between publishing one set of URLs and publishing the same article
//! twice for a crawler to choose between.
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
use serde::Deserialize;

/// The one origin a site publishes itself under, when it answers on more than
/// one.
///
/// A reverse proxy can point several hostnames at the same socket — a product
/// domain and a vanity one, an apex and its `www`, an old name kept alive. The
/// application behind it then serves every page at two addresses, and a crawler
/// that finds both has found two copies of one article: it picks a winner on its
/// own, splits the signals between them, and may index whichever one you did not
/// mean. Nothing in the request distinguishes "the host you were asked on" from
/// "the host you should be found under" — only configuration can.
///
/// Setting one makes every *published* URL name the same host regardless of
/// which one was asked: the `<link rel="canonical">` on a page, the `<loc>`s in
/// [`Sitemap`], and the `Sitemap:` line in [`RobotsTxt`] — which is emitted only
/// on the canonical host, because a sitemap belongs to the host it lists.
/// Crawling the other host is still fine and, in fact, the point: it serves the
/// same pages with a canonical pointing here, which is what tells a crawler the
/// two are one page rather than two.
///
/// Unset (the default) means "follow the request", which is right for a site on
/// exactly one hostname — and is also the setting to leave alone in development,
/// where the host is `localhost:8801` and no crawler is watching.
///
/// It is worth setting for a second reason, one that bites single-domain
/// deployments too: it pins the *scheme*. A proxy that terminates TLS without
/// adding `X-Forwarded-Proto` leaves this server with no way to know it is an
/// HTTPS site, and [`request_origin`] then honestly reports `http://`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct CanonicalOrigin(Option<String>);

impl CanonicalOrigin {
  /// Follow whatever host each request arrived on.
  pub fn from_request() -> Self {
    Self(None)
  }

  /// Publish under `origin` (`https://example.com`), whatever host the request
  /// arrived on. A value without a scheme is taken as `https://`, and a trailing
  /// slash is trimmed; an empty one is the same as [`CanonicalOrigin::from_request`].
  pub fn fixed(origin: impl Into<String>) -> Self {
    let origin = origin.into();
    let origin = origin.trim().trim_end_matches('/');
    if origin.is_empty() {
      return Self(None);
    }
    Self(Some(if origin.contains("://") {
      origin.to_string()
    } else {
      format!("https://{origin}")
    }))
  }

  /// Reads the origin from an environment variable, falling back to following
  /// the request when it is unset or empty — so one deployment can pin its
  /// domain and another (or a developer's laptop) needs no configuration at all.
  pub fn from_env(var: &str) -> Self {
    std::env::var(var).map(Self::fixed).unwrap_or_default()
  }

  /// The origin to build published URLs with.
  pub fn resolve(&self, req: &Request) -> String {
    self.0.clone().unwrap_or_else(|| request_origin(req))
  }

  /// Whether this request arrived on the canonical host — always true when no
  /// canonical origin is configured, because then there is only one host as far
  /// as this server knows.
  ///
  /// Compared by host, not by whole origin: the scheme is exactly the part a
  /// proxy may have failed to forward, and a canonical host would otherwise stop
  /// recognising itself over a missing `X-Forwarded-Proto`.
  pub fn is_canonical(&self, req: &Request) -> bool {
    match &self.0 {
      None => true,
      Some(origin) => host_of(origin).eq_ignore_ascii_case(host_of(&request_origin(req))),
    }
  }

  /// The configured origin, if there is one.
  pub fn configured(&self) -> Option<&str> {
    self.0.as_deref()
  }
}

/// The host of an origin: what is left after the scheme and before any path.
fn host_of(origin: &str) -> &str {
  let after_scheme = origin.split_once("://").map(|(_, rest)| rest).unwrap_or(origin);
  after_scheme.split('/').next().unwrap_or(after_scheme)
}

/// The origin — `scheme://host` — the request arrived on.
///
/// Both files this module builds are required to carry absolute URLs, and an
/// application that is not told its own address in configuration has exactly one
/// place to learn it: the request in front of it.
///
/// Three sources, in order:
///
/// 1. **`X-Forwarded-Origin`** — a proxy stating outright what this service is
///    published as. It is the only one that can be *right* about an application
///    reachable on several hostnames, because it comes from the side that knows
///    which of them is the site's own name; the LBRP gateway sends it when a
///    service sets `provide_origin_as_header`. Taken whole, so the scheme and
///    the host cannot arrive from different hops and disagree.
/// 2. **`X-Forwarded-Proto` + `X-Forwarded-Host`** — the conventional pair. The
///    scheme matters most: behind a proxy that terminates TLS the connection
///    this server accepted is plain HTTP, and taking that as the truth publishes
///    `http://` URLs for an `https://` site, which a crawler files away as a
///    second, duplicate host. Both headers list proxies left to right, so the
///    first value is the one the client actually spoke to.
/// 3. The request's own `Host`.
///
/// An application with its own [`CanonicalOrigin`] configured overrides all
/// three: local configuration outranks anything the network claims.
///
/// **Trust:** these headers are whatever the client sent unless a proxy
/// overwrites them, so a server exposed directly to the internet can be told any
/// origin at all. That is survivable for what this module does — a poisoned
/// `robots.txt` or `sitemap.xml` is served to whoever poisoned it, and nobody
/// else — but do not reach for this to build anything a *different* user will
/// follow, such as a link in an e-mail.
pub fn request_origin(req: &Request) -> String {
  if let Some(origin) = forwarded_origin(req) {
    return origin;
  }
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

/// `X-Forwarded-Origin`, if it is an origin and nothing else.
///
/// Parsed strictly rather than trusted: this value is pasted into every URL the
/// application publishes, so a path, a stray space or a credential-looking
/// `user@host` is dropped rather than carried into a sitemap. What survives is
/// `http(s)://host[:port]`, normalised without a trailing slash.
fn forwarded_origin(req: &Request) -> Option<String> {
  let raw = req.headers().get("x-forwarded-origin")?.to_str().ok()?;
  let first = raw.split(',').next().unwrap_or(raw).trim().trim_end_matches('/');
  let (scheme, host) = first.split_once("://")?;
  let usable = matches!(scheme, "http" | "https")
    && !host.is_empty()
    && !host
      .chars()
      .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '/' | '@' | '?' | '#' | '"' | '\'' | '<' | '>'));
  usable.then(|| format!("{scheme}://{host}"))
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

  /// The header the LBRP gateway sends when a service asks it to: the one
  /// source that can be right about which of several hostnames is the site's
  /// own name, because it is the only one that knows.
  #[test]
  fn prefers_the_origin_a_proxy_states_outright() {
    let req = req_with(&[
      ("host", "blog.example.com"),
      ("x-forwarded-proto", "https"),
      ("x-forwarded-origin", "https://app.example.com"),
    ]);
    assert_eq!(request_origin(&req), "https://app.example.com");
  }

  /// It goes straight into every published URL, so anything that is not plainly
  /// an origin is dropped back to the headers that are — better a right answer
  /// from a lesser source than a wrong one from a better.
  #[test]
  fn refuses_a_forwarded_origin_that_is_not_one() {
    for claim in [
      "https://app.example.com/../evil",
      "https://user@evil.example",
      "javascript:alert(1)",
      "app.example.com",
      "https://",
    ] {
      let req = req_with(&[("host", "blog.example.com"), ("x-forwarded-origin", claim)]);
      assert_eq!(request_origin(&req), "http://blog.example.com", "for {claim:?}");
    }
  }

  /// A trailing slash is how a proxy or an operator writes an origin half the
  /// time, and `https://x.example//p/a` is a different URL from `…/p/a`.
  #[test]
  fn normalises_the_forwarded_origin() {
    let req = req_with(&[("x-forwarded-origin", "https://app.example.com/")]);
    assert_eq!(request_origin(&req), "https://app.example.com");
  }

  /// Local configuration outranks anything the network says it is.
  #[test]
  fn configuration_outranks_the_forwarded_origin() {
    let req = req_with(&[("x-forwarded-origin", "https://app.example.com")]);
    let canonical = CanonicalOrigin::fixed("https://blog.example.com");
    assert_eq!(canonical.resolve(&req), "https://blog.example.com");
  }

  /// The whole point of configuring one: an application answering on two
  /// hostnames must publish one set of URLs, or a crawler files the same article
  /// twice and picks a winner itself.
  #[test]
  fn publishes_one_origin_however_it_was_asked() {
    let canonical = CanonicalOrigin::fixed("https://blog.example.com");
    let asked_on_alias = req_with(&[("host", "app.example.com"), ("x-forwarded-proto", "https")]);
    let asked_on_canonical = req_with(&[("host", "blog.example.com"), ("x-forwarded-proto", "https")]);

    assert_eq!(canonical.resolve(&asked_on_alias), "https://blog.example.com");
    assert_eq!(canonical.resolve(&asked_on_canonical), "https://blog.example.com");
    assert!(!canonical.is_canonical(&asked_on_alias));
    assert!(canonical.is_canonical(&asked_on_canonical));
  }

  /// Unset is the single-domain deployment, and a developer's laptop: follow the
  /// request, and never decide a request is on the "wrong" host.
  #[test]
  fn follows_the_request_when_no_origin_is_configured() {
    let req = req_with(&[("host", "localhost:8801")]);
    assert_eq!(CanonicalOrigin::default().resolve(&req), "http://localhost:8801");
    assert!(CanonicalOrigin::default().is_canonical(&req));
    assert_eq!(CanonicalOrigin::fixed("  ").resolve(&req), "http://localhost:8801");
  }

  /// The scheme is exactly what a proxy forgets to forward, so recognising the
  /// canonical host must not depend on it — otherwise the canonical host stops
  /// admitting to being itself and stops advertising its own sitemap.
  #[test]
  fn recognises_its_host_without_the_scheme() {
    let canonical = CanonicalOrigin::fixed("blog.example.com");
    assert_eq!(canonical.configured(), Some("https://blog.example.com"));
    assert!(canonical.is_canonical(&req_with(&[("host", "blog.example.com")])));
    assert!(canonical.is_canonical(&req_with(&[("host", "BLOG.example.com")])));
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
