//! `sitemap.xml`, static or built per request.
//!
//! A sitemap is a list of the URLs a site wants found, with an optional
//! "last changed" date for each. It does not *grant* anything — a URL in a
//! sitemap that is disallowed in `robots.txt`, or served `noindex`, still will
//! not be indexed — it only saves a crawler from having to discover pages by
//! following links, which is what makes it worth having for anything published
//! faster than crawlers wander.
//!
//! A fixed list is mounted directly:
//!
//! ```rust,ignore
//! let router = router.push(
//!   Sitemap::new()
//!     .url(SitemapUrl::new("https://example.com/"))
//!     .url(SitemapUrl::new("https://example.com/pricing").changefreq(ChangeFreq::Monthly))
//!     .into_router(),
//! );
//! ```
//!
//! A list that changes is written per request instead. [`Sitemap`] is a salvo
//! `Writer`, so a handler that returns one is the whole integration:
//!
//! ```rust,ignore
//! #[handler]
//! async fn sitemap(depot: &mut Depot, req: &mut Request) -> MResult<Sitemap> {
//!   let origin = request_origin(req);
//!   let mut map = Sitemap::new();
//!   for doc in published_documents(depot).await? {
//!     map.push(SitemapUrl::new(format!("{origin}/p/{}", doc.slug)).lastmod(doc.updated_at.to_rfc3339()));
//!   }
//!   Ok(map)
//! }
//! ```
//!
//! Two things the format insists on, both enforced here: every `<loc>` must be
//! an **absolute** URL — hence [`request_origin`](super::request_origin) — and a
//! single file may carry at most [`MAX_SITEMAP_URLS`] of them.

use std::sync::Arc;

use salvo::http::header::{CONTENT_TYPE, HeaderValue};
use salvo::prelude::*;
use salvo::writing::Text;

use super::xml_escape;

/// Where a sitemap conventionally lives. Nothing requires this path — a
/// `Sitemap:` line in `robots.txt` can point anywhere — but a crawler that goes
/// looking without being told looks here.
pub const SITEMAP_XML_PATH: &str = "sitemap.xml";

/// The most URLs one sitemap file may carry (the limit Google, Bing and Yandex
/// all enforce). Past it the file is rejected whole, so a site with more pages
/// than this needs several files and a sitemap index pointing at them — split
/// them by advertising more than one `Sitemap:` line in `robots.txt`.
pub const MAX_SITEMAP_URLS: usize = 50_000;

/// How often a page is expected to change. Advisory: Google ignores it outright,
/// and the others treat it as a hint, so it is worth setting only where it is
/// actually true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeFreq {
  /// Changes every time it is accessed.
  Always,
  /// Hourly.
  Hourly,
  /// Daily.
  Daily,
  /// Weekly.
  Weekly,
  /// Monthly.
  Monthly,
  /// Yearly.
  Yearly,
  /// Archived: it will not change again.
  Never,
}

impl ChangeFreq {
  fn as_str(self) -> &'static str {
    match self {
      Self::Always => "always",
      Self::Hourly => "hourly",
      Self::Daily => "daily",
      Self::Weekly => "weekly",
      Self::Monthly => "monthly",
      Self::Yearly => "yearly",
      Self::Never => "never",
    }
  }
}

/// One entry of a sitemap.
#[derive(Clone, Debug)]
pub struct SitemapUrl {
  loc: String,
  lastmod: Option<String>,
  changefreq: Option<ChangeFreq>,
  priority: Option<f32>,
}

impl SitemapUrl {
  /// An entry for `loc`, which must be an **absolute** URL — a crawler drops a
  /// relative one, and a sitemap of relative URLs is a sitemap of nothing. Build
  /// it from [`request_origin`](super::request_origin) when the site's own
  /// address is not in configuration.
  pub fn new(loc: impl Into<String>) -> Self {
    Self {
      loc: loc.into(),
      lastmod: None,
      changefreq: None,
      priority: None,
    }
  }

  /// When the page last changed, as a W3C datetime: `2026-08-12` or
  /// `2026-08-12T15:04:00Z` (`chrono`'s `to_rfc3339()` produces exactly this).
  ///
  /// Worth setting and worth being honest about — it is the one field crawlers
  /// act on, and a file where every page changed today teaches them to ignore it.
  /// A value that is not a date is dropped, with a warning, rather than rendered
  /// into a file that would fail validation as a whole.
  pub fn lastmod(mut self, when: impl Into<String>) -> Self {
    let when = when.into();
    if is_w3c_datetime(&when) {
      self.lastmod = Some(when);
    } else {
      tracing::warn!(lastmod = %when, loc = %self.loc, "dropping sitemap lastmod that is not a W3C datetime");
    }
    self
  }

  /// Sets the advisory change frequency.
  pub fn changefreq(mut self, freq: ChangeFreq) -> Self {
    self.changefreq = Some(freq);
    self
  }

  /// Sets this URL's priority *relative to the rest of this site*, clamped to
  /// `0.0..=1.0`. Advisory, and ignored by Google; it never affects how a page
  /// ranks against anybody else's.
  pub fn priority(mut self, priority: f32) -> Self {
    self.priority = Some(priority.clamp(0.0, 1.0));
    self
  }

  fn render_into(&self, out: &mut String) {
    out.push_str("  <url>\n    <loc>");
    out.push_str(&xml_escape(self.loc.trim()));
    out.push_str("</loc>\n");
    if let Some(lastmod) = &self.lastmod {
      out.push_str("    <lastmod>");
      out.push_str(&xml_escape(lastmod));
      out.push_str("</lastmod>\n");
    }
    if let Some(freq) = self.changefreq {
      out.push_str("    <changefreq>");
      out.push_str(freq.as_str());
      out.push_str("</changefreq>\n");
    }
    if let Some(priority) = self.priority {
      out.push_str(&format!("    <priority>{priority:.1}</priority>\n"));
    }
    out.push_str("  </url>\n");
  }
}

/// A sitemap document.
#[derive(Clone, Debug, Default)]
pub struct Sitemap {
  urls: Vec<SitemapUrl>,
}

impl Sitemap {
  /// An empty sitemap.
  pub fn new() -> Self {
    Self::default()
  }

  /// Adds one URL, chaining.
  pub fn url(mut self, url: SitemapUrl) -> Self {
    self.urls.push(url);
    self
  }

  /// Adds every URL of an iterator, chaining.
  pub fn urls(mut self, urls: impl IntoIterator<Item = SitemapUrl>) -> Self {
    self.urls.extend(urls);
    self
  }

  /// Adds one URL in place, for building inside a loop.
  pub fn push(&mut self, url: SitemapUrl) {
    self.urls.push(url);
  }

  /// How many URLs the document carries.
  pub fn len(&self) -> usize {
    self.urls.len()
  }

  /// Whether the document carries no URLs. An empty sitemap is valid, and says
  /// "nothing to offer" rather than "something went wrong" — which is why a
  /// handler that finds nothing should still answer with one.
  pub fn is_empty(&self) -> bool {
    self.urls.is_empty()
  }

  /// Renders the XML.
  pub fn render(&self) -> String {
    if self.urls.len() > MAX_SITEMAP_URLS {
      // Not truncated: dropping URLs silently would leave a file that validates
      // and is quietly wrong, which is harder to notice than one that is
      // rejected. Split the site across several sitemaps instead.
      tracing::warn!(
        urls = self.urls.len(),
        limit = MAX_SITEMAP_URLS,
        "sitemap exceeds the maximum size crawlers accept and will be rejected whole"
      );
    }
    let mut out = String::with_capacity(128 + self.urls.len() * 96);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for url in &self.urls {
      url.render_into(&mut out);
    }
    out.push_str("</urlset>\n");
    out
  }

  /// The handler serving this fixed document, for mounting somewhere other than
  /// the root. Most callers want [`Sitemap::into_router`].
  pub fn into_handler(self) -> SitemapHandler {
    SitemapHandler {
      body: Arc::from(self.render()),
    }
  }

  /// A router serving this fixed document at `/sitemap.xml`.
  ///
  /// For a sitemap whose contents change, write a handler that returns a
  /// `Sitemap` instead — see the [module docs](self) — because this one renders
  /// once, when the router is built.
  pub fn into_router(self) -> Router {
    Router::with_path(SITEMAP_XML_PATH).get(self.into_handler())
  }
}

/// Writes `body` as a sitemap response: the XML, labelled as XML.
///
/// `Text::Plain` carries the body without re-encoding it and the content type is
/// then corrected, which is the same route [`Text`] offers for any type it has
/// no variant of.
fn write_sitemap(body: &str, res: &mut Response) {
  res.render(Text::Plain(body.to_string()));
  res
    .headers_mut()
    .insert(CONTENT_TYPE, HeaderValue::from_static("application/xml; charset=utf-8"));
}

#[salvo::async_trait]
impl Writer for Sitemap {
  async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
    write_sitemap(&self.render(), res);
  }
}

/// Serves a fixed [`Sitemap`]. Built by [`Sitemap::into_handler`].
pub struct SitemapHandler {
  /// Rendered once: a fixed document cannot change between requests, and this
  /// one is the size of the site.
  body: Arc<str>,
}

#[salvo::async_trait]
impl Handler for SitemapHandler {
  async fn handle(&self, _req: &mut Request, _depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    write_sitemap(&self.body, res);
  }
}

/// A W3C datetime, as far as this needs to check: a `YYYY-MM-DD` date, possibly
/// followed by a time. Deliberately structural rather than a real date parse —
/// this crate carries no date library, and what it has to catch is a value that
/// would make the whole file fail validation, not a 31st of February.
fn is_w3c_datetime(s: &str) -> bool {
  let b = s.as_bytes();
  if b.len() < 10 {
    return false;
  }
  let digits = |range: std::ops::Range<usize>| b[range].iter().all(u8::is_ascii_digit);
  digits(0..4) && b[4] == b'-' && digits(5..7) && b[7] == b'-' && digits(8..10) && (b.len() == 10 || b[10] == b'T')
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn renders_the_document_a_crawler_expects() {
    let rendered = Sitemap::new()
      .url(SitemapUrl::new("https://example.com/"))
      .url(
        SitemapUrl::new("https://example.com/p/article")
          .lastmod("2026-08-12T15:04:00Z")
          .changefreq(ChangeFreq::Weekly)
          .priority(0.8),
      )
      .render();

    assert_eq!(
      rendered,
      "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
       <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
       \x20 <url>\n    <loc>https://example.com/</loc>\n  </url>\n\
       \x20 <url>\n    <loc>https://example.com/p/article</loc>\n\
       \x20   <lastmod>2026-08-12T15:04:00Z</lastmod>\n\
       \x20   <changefreq>weekly</changefreq>\n\
       \x20   <priority>0.8</priority>\n  </url>\n\
       </urlset>\n"
    );
  }

  /// A slug is user input often enough, and an unescaped `&` in one does not
  /// break that entry — it breaks the file, and a crawler reports it as no
  /// sitemap at all.
  #[test]
  fn escapes_a_url_that_would_break_the_file() {
    let rendered = Sitemap::new()
      .url(SitemapUrl::new("https://example.com/p/a&b?x=1&y=<2>"))
      .render();
    assert!(
      rendered.contains("<loc>https://example.com/p/a&amp;b?x=1&amp;y=&lt;2&gt;</loc>"),
      "got:\n{rendered}"
    );
  }

  /// `lastmod` is the one field crawlers act on, so a value that is not a date
  /// is left out rather than rendered — one bad row must not cost the file.
  #[test]
  fn leaves_out_a_lastmod_that_is_not_a_date() {
    let rendered = Sitemap::new()
      .url(SitemapUrl::new("https://example.com/").lastmod("вчера"))
      .render();
    assert!(!rendered.contains("<lastmod>"), "got:\n{rendered}");
  }

  #[test]
  fn accepts_both_shapes_of_w3c_datetime() {
    assert!(is_w3c_datetime("2026-08-12"));
    assert!(is_w3c_datetime("2026-08-12T15:04:00Z"));
    assert!(is_w3c_datetime("2026-08-12T15:04:00+03:00"));
    assert!(!is_w3c_datetime("12.08.2026"));
    assert!(!is_w3c_datetime("2026-08-12 15:04"));
    assert!(!is_w3c_datetime(""));
  }

  /// Priority is a fraction; a caller who thinks in percentages must not be able
  /// to render an invalid document.
  #[test]
  fn clamps_priority_into_the_range_the_format_allows() {
    let rendered = Sitemap::new()
      .url(SitemapUrl::new("https://example.com/").priority(80.0))
      .render();
    assert!(rendered.contains("<priority>1.0</priority>"), "got:\n{rendered}");
  }

  /// An empty sitemap is a valid answer, and the one a site with nothing
  /// published yet should give.
  #[test]
  fn renders_an_empty_document() {
    let rendered = Sitemap::new().render();
    assert!(rendered.contains("<urlset"), "got:\n{rendered}");
    assert!(!rendered.contains("<url>"), "got:\n{rendered}");
    assert!(Sitemap::new().is_empty());
  }
}
