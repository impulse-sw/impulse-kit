//! `robots.txt`, built rather than kept on disk.
//!
//! ```rust,ignore
//! use impulse_server_kit::prelude::*;
//!
//! let router = get_root_router(&state).push(
//!   RobotsTxt::new()
//!     .comment("Only /p/ — published documents — is meant to be crawled.")
//!     .disallow("/s/")
//!     .disallow("/api/")
//!     .group(RobotsGroup::for_agent("GPTBot").disallow("/"))
//!     .sitemap("/sitemap.xml")
//!     .into_router(),
//! );
//! ```
//!
//! renders
//!
//! ```text
//! # Only /p/ — published documents — is meant to be crawled.
//! User-agent: *
//! Disallow: /s/
//! Disallow: /api/
//!
//! User-agent: GPTBot
//! Disallow: /
//!
//! Sitemap: https://example.com/sitemap.xml
//! ```
//!
//! Every field is `Deserialize`, so the same document can come out of the
//! application's own YAML instead of its code:
//!
//! ```yaml
//! robots:
//!   comment: Staging. Nothing here is meant to be found.
//!   groups:
//!     - agents: ["*"]
//!       disallow: ["/"]
//! ```
//!
//! Rules that cannot appear in a valid `robots.txt` — a path that starts with
//! neither `/` nor `*`, anything carrying a `#` (which would comment out the
//! rest of its own line) or a control character — are dropped with a warning
//! when the handler is built, so a typo in a config file costs one log line
//! rather than a file that reads as "crawl everything".

use std::sync::Arc;

use salvo::prelude::*;
use salvo::writing::Text;
use serde::Deserialize;

use super::CanonicalOrigin;

/// Where `robots.txt` has to live: the root of the origin, and nowhere else.
/// Crawlers do not look anywhere else, and a `robots.txt` under a sub-path
/// governs nothing.
pub const ROBOTS_TXT_PATH: &str = "robots.txt";

/// Ready-made `X-Robots-Tag` (and `<meta name="robots">`) values.
///
/// Named constants because the two places a page states its indexability — the
/// header and the markup — must agree, and the way they stop agreeing is
/// somebody spelling one of them out again by hand.
pub struct RobotsTag;

impl RobotsTag {
  /// Index the page, follow its links. The default behaviour, worth stating
  /// explicitly on a page that sits among others which are not indexable.
  pub const INDEX_FOLLOW: &'static str = "index,follow";
  /// Keep the page out of the index, but follow its links — for a duplicate
  /// rendering of a page that already has a canonical URL.
  pub const NOINDEX: &'static str = "noindex";
  /// Keep the page out of the index and follow nothing on it — for a URL that
  /// was addressed to someone rather than published.
  pub const NOINDEX_NOFOLLOW: &'static str = "noindex,nofollow";
}

/// Sets `X-Robots-Tag` on a response.
///
/// The header is the half of a page's indexing rules that survives a body
/// nobody parses as HTML — a JSON read, a PDF, plain Markdown — and reaches a
/// crawler that takes the headers and stops there. On an HTML page it belongs
/// *beside* the `<meta name="robots">`, not instead of it, and both should be
/// given the same [`RobotsTag`] constant.
///
/// An invalid header value is logged and skipped rather than panicking, in the
/// same spirit as [`crate::security_headers`] — a malformed directive must not
/// take a response down.
pub fn set_x_robots_tag(res: &mut Response, directive: &str) {
  match salvo::http::HeaderValue::from_str(directive) {
    Ok(value) => {
      res.headers_mut().insert("x-robots-tag", value);
    }
    Err(e) => tracing::warn!(error = %e, directive, "ignoring invalid X-Robots-Tag value"),
  }
}

/// One `User-agent:` block: the crawlers it addresses and what they may do.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct RobotsGroup {
  /// The crawlers this block addresses. Empty means `*` — every crawler that
  /// has not been given a block of its own.
  pub agents: Vec<String>,
  /// Path prefixes this group may crawl. Only ever needed to carve an exception
  /// out of a `Disallow` in the same group; nothing is forbidden by default.
  pub allow: Vec<String>,
  /// Path prefixes this group must not crawl. `*` matches any run of
  /// characters and `$` anchors the end, both understood by Google, Bing and
  /// Yandex — and by nobody else, which is why a rule that relies on them
  /// should also be safe when it is read literally.
  pub disallow: Vec<String>,
  /// Seconds a crawler should wait between requests. Ignored by Google (which
  /// has its own setting) and honoured by most others.
  pub crawl_delay: Option<u32>,
}

impl RobotsGroup {
  /// A block addressing every crawler (`User-agent: *`).
  pub fn all_agents() -> Self {
    Self::default()
  }

  /// A block addressing one named crawler.
  pub fn for_agent(agent: impl Into<String>) -> Self {
    Self {
      agents: vec![agent.into()],
      ..Self::default()
    }
  }

  /// A block addressing several named crawlers at once.
  pub fn for_agents<I, S>(agents: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    Self {
      agents: agents.into_iter().map(Into::into).collect(),
      ..Self::default()
    }
  }

  /// Allows a path prefix, carving an exception out of a `Disallow`.
  pub fn allow(mut self, path: impl Into<String>) -> Self {
    self.allow.push(path.into());
    self
  }

  /// Disallows a path prefix.
  pub fn disallow(mut self, path: impl Into<String>) -> Self {
    self.disallow.push(path.into());
    self
  }

  /// Asks this group's crawlers to wait `seconds` between requests.
  pub fn crawl_delay(mut self, seconds: u32) -> Self {
    self.crawl_delay = Some(seconds);
    self
  }

  fn render_into(&self, out: &mut String) {
    if self.agents.is_empty() {
      out.push_str("User-agent: *\n");
    } else {
      for agent in self.agents.iter().filter(|a| is_valid_agent(a)) {
        out.push_str("User-agent: ");
        out.push_str(agent.trim());
        out.push('\n');
      }
    }
    for path in self.allow.iter().filter(|p| is_valid_rule(p)) {
      out.push_str("Allow: ");
      out.push_str(path.trim());
      out.push('\n');
    }
    for path in self.disallow.iter().filter(|p| is_valid_rule(p)) {
      out.push_str("Disallow: ");
      out.push_str(path.trim());
      out.push('\n');
    }
    // A group has to say *something*, and an empty `Disallow:` is how the format
    // spells "nothing is off limits". Without it a group of nothing but a
    // `User-agent:` line is malformed, and a crawler is free to read the file as
    // far as the malformed part and stop.
    if !self.has_rules() {
      out.push_str("Disallow:\n");
    }
    if let Some(delay) = self.crawl_delay {
      out.push_str(&format!("Crawl-delay: {delay}\n"));
    }
  }

  fn has_rules(&self) -> bool {
    self.allow.iter().any(|p| is_valid_rule(p)) || self.disallow.iter().any(|p| is_valid_rule(p))
  }
}

/// A `robots.txt` document: groups of rules, plus the sitemaps to advertise.
///
/// Build it with the methods below, or deserialize it from configuration — see
/// the [module docs](self). Nothing is disallowed until you say so: an empty
/// document renders as a single group that permits everything, which is what a
/// site without a `robots.txt` gets anyway.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct RobotsTxt {
  /// A note rendered as `#` comments at the top of the file. For the humans who
  /// open `robots.txt` to find out what a site's policy is *supposed* to be —
  /// crawlers ignore it.
  pub comment: Option<String>,
  /// The `User-agent:` blocks, in the order they are rendered.
  pub groups: Vec<RobotsGroup>,
  /// Sitemap URLs to advertise. An absolute URL is used as given; a value
  /// starting with `/` is resolved against the origin each request arrives on
  /// (see [`request_origin`](super::request_origin)), which is what lets an
  /// application that has never been told its own domain still advertise its
  /// sitemap correctly.
  pub sitemaps: Vec<String>,
  /// The host this site is published under, when it answers on several. Left
  /// unset the file simply follows whichever host asked for it, which is right
  /// for a site with one hostname; see [`RobotsTxt::canonical_origin`].
  pub canonical: CanonicalOrigin,
}

impl RobotsTxt {
  /// An empty document: no restrictions, no sitemaps.
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the comment rendered at the top of the file. Multi-line text is fine;
  /// every line gets its own `#`.
  pub fn comment(mut self, text: impl Into<String>) -> Self {
    self.comment = Some(text.into());
    self
  }

  /// Allows a path prefix for every crawler, carving an exception out of a
  /// `Disallow` added by [`RobotsTxt::disallow`].
  pub fn allow(mut self, path: impl Into<String>) -> Self {
    self.all_agents_group().allow.push(path.into());
    self
  }

  /// Disallows a path prefix for every crawler.
  pub fn disallow(mut self, path: impl Into<String>) -> Self {
    self.all_agents_group().disallow.push(path.into());
    self
  }

  /// Adds a block addressing particular crawlers.
  ///
  /// A crawler obeys exactly one group — the most specific one that names it —
  /// so a rule that should apply to a named crawler *as well as* everyone else
  /// has to be repeated in its group. That is the format's rule, not this
  /// builder's.
  pub fn group(mut self, group: RobotsGroup) -> Self {
    self.groups.push(group);
    self
  }

  /// Advertises a sitemap, absolute (`https://example.com/sitemap.xml`) or
  /// rooted (`/sitemap.xml`, resolved per request).
  pub fn sitemap(mut self, url: impl Into<String>) -> Self {
    self.sitemaps.push(url.into());
    self
  }

  /// Pins the host this site is published under, for an application that a
  /// reverse proxy hands more than one.
  ///
  /// Two things follow. Rooted sitemap paths resolve against *this* origin
  /// rather than the one asked, so both hosts advertise the same file; and the
  /// `Sitemap:` line is emitted **only** on the canonical host, because a
  /// sitemap describes the host it is served from — one that lists another
  /// host's URLs is cross-submission, which a crawler ignores unless the owner
  /// has proved ownership of both.
  ///
  /// The rules themselves are unchanged on the other hosts, deliberately: they
  /// serve the same pages, and the way a crawler learns that two addresses are
  /// one page is by fetching both and finding the same `<link rel="canonical">`.
  /// Slamming the door with `Disallow: /` instead would leave it unable to see
  /// that, and able to list the alias as a bare URL anyway.
  pub fn canonical_origin(mut self, canonical: CanonicalOrigin) -> Self {
    self.canonical = canonical;
    self
  }

  /// Renders the file as the request in front of it should see it: sitemap
  /// paths resolved against the canonical origin, and the `Sitemap:` line left
  /// off when this is not the canonical host.
  pub fn render_for(&self, req: &Request) -> String {
    self.render_inner(&self.canonical.resolve(req), self.canonical.is_canonical(req))
  }

  /// Renders the file.
  ///
  /// `origin` is the `scheme://host` that rooted sitemap paths resolve against
  /// — [`request_origin`](super::request_origin) in a handler, the site's own
  /// URL anywhere else. It is unused when every sitemap URL is already
  /// absolute, and irrelevant when there are none, so `""` is a fine argument
  /// in both cases.
  pub fn render(&self, origin: &str) -> String {
    self.render_inner(origin, true)
  }

  fn render_inner(&self, origin: &str, advertise_sitemaps: bool) -> String {
    let mut out = String::new();
    if let Some(comment) = &self.comment {
      for line in comment.lines() {
        out.push_str("# ");
        out.push_str(line.trim_end());
        out.push('\n');
      }
    }
    // An empty document still has to be a valid file. A permit-all group says
    // the same thing as having no `robots.txt` at all, which is what an operator
    // who mounted an empty one meant.
    let fallback = [RobotsGroup::all_agents()];
    let groups = if self.groups.is_empty() {
      &fallback[..]
    } else {
      &self.groups[..]
    };
    for group in groups {
      // Blocks are separated by a blank line, and so is the first block from the
      // comment above it — which is why this asks what has been written so far
      // rather than which group it is on.
      if !out.is_empty() {
        out.push('\n');
      }
      group.render_into(&mut out);
    }
    let sitemaps: Vec<String> = if advertise_sitemaps {
      self
        .sitemaps
        .iter()
        .filter_map(|url| resolve_sitemap(url, origin))
        .collect()
    } else {
      Vec::new()
    };
    if !sitemaps.is_empty() {
      out.push('\n');
      for url in sitemaps {
        out.push_str("Sitemap: ");
        out.push_str(&url);
        out.push('\n');
      }
    }
    out
  }

  /// The handler serving this document, for mounting somewhere other than the
  /// root — a nested router, a host-specific branch. Most callers want
  /// [`RobotsTxt::into_router`].
  pub fn into_handler(self) -> RobotsTxtHandler {
    self.warn_about_dropped_rules();
    RobotsTxtHandler { robots: Arc::new(self) }
  }

  /// A router serving this document at `/robots.txt`.
  ///
  /// Mount it ahead of any catch-all: a fallback route that answers every
  /// unmatched path with an application shell will happily answer this one too,
  /// and an HTML body where a crawler expects rules is read as no rules at all.
  pub fn into_router(self) -> Router {
    Router::with_path(ROBOTS_TXT_PATH).get(self.into_handler())
  }

  /// The `*` group, appended if the document has not got one yet.
  fn all_agents_group(&mut self) -> &mut RobotsGroup {
    let existing = self.groups.iter().position(|g| g.agents.is_empty());
    let at = match existing {
      Some(at) => at,
      None => {
        self.groups.push(RobotsGroup::all_agents());
        self.groups.len() - 1
      }
    };
    &mut self.groups[at]
  }

  /// Says once, at startup, what [`RobotsTxt::render`] then drops silently on
  /// every request. Rendering cannot be the place that warns: a crawler asking
  /// for the file would be all it takes to fill the log with the same line.
  fn warn_about_dropped_rules(&self) {
    for group in &self.groups {
      for agent in group.agents.iter().filter(|a| !is_valid_agent(a)) {
        tracing::warn!(agent, "dropping unusable robots.txt user-agent");
      }
      for path in group.allow.iter().chain(&group.disallow).filter(|p| !is_valid_rule(p)) {
        tracing::warn!(
          path,
          "dropping unusable robots.txt rule: a path must start with `/` or `*` and carry no `#`"
        );
      }
    }
    for url in self
      .sitemaps
      .iter()
      .filter(|u| resolve_sitemap(u, "https://x").is_none())
    {
      tracing::warn!(
        url,
        "dropping unusable sitemap URL: expected an absolute URL or a `/` path"
      );
    }
  }
}

/// A rule this file can carry without lying about what it forbids.
///
/// The `#` matters more than it looks: `robots.txt` comments run to the end of
/// the line, so `Disallow: /a#b` forbids `/a` and quietly permits everything the
/// author thought they had just closed off.
fn is_valid_rule(path: &str) -> bool {
  let path = path.trim();
  (path.starts_with('/') || path.starts_with('*'))
    && !path.contains('#')
    && !path.chars().any(|c| c.is_control() || c == ' ')
}

fn is_valid_agent(agent: &str) -> bool {
  let agent = agent.trim();
  !agent.is_empty() && !agent.contains('#') && !agent.chars().any(|c| c.is_control())
}

/// A sitemap URL as it should appear in the file, or nothing if it is neither an
/// absolute URL nor a path this server can turn into one.
fn resolve_sitemap(url: &str, origin: &str) -> Option<String> {
  let url = url.trim();
  if url.chars().any(|c| c.is_control() || c.is_whitespace()) || url.contains('#') {
    return None;
  }
  if url.starts_with("http://") || url.starts_with("https://") {
    return Some(url.to_string());
  }
  url
    .starts_with('/')
    .then(|| format!("{}{url}", origin.trim_end_matches('/')))
}

/// Serves a [`RobotsTxt`]. Built by [`RobotsTxt::into_handler`].
pub struct RobotsTxtHandler {
  robots: Arc<RobotsTxt>,
}

#[salvo::async_trait]
impl Handler for RobotsTxtHandler {
  async fn handle(&self, req: &mut Request, _depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    // Rendered per request rather than once at startup, because both halves of
    // the answer depend on the request: a rooted sitemap path resolves against
    // an origin, and which host asked decides whether the sitemap is this host's
    // to advertise. The file is a few hundred bytes and is fetched about once a
    // day per crawler.
    res.render(Text::Plain(self.robots.render_for(req)));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// The shape everything else is checked against: a comment, one anonymous
  /// group, and a sitemap line separated from the rules.
  #[test]
  fn renders_a_file_a_crawler_can_read() {
    let rendered = RobotsTxt::new()
      .comment("Only /p/ is meant to be crawled.")
      .disallow("/s/")
      .disallow("/api/")
      .sitemap("https://example.com/sitemap.xml")
      .render("");

    assert_eq!(
      rendered,
      "# Only /p/ is meant to be crawled.\n\
       \n\
       User-agent: *\n\
       Disallow: /s/\n\
       Disallow: /api/\n\
       \n\
       Sitemap: https://example.com/sitemap.xml\n"
    );
  }

  /// The reason the handler renders per request: an application that was never
  /// told its own domain still has to advertise its sitemap under the right one.
  #[test]
  fn resolves_a_rooted_sitemap_against_the_origin() {
    let robots = RobotsTxt::new().sitemap("/sitemap.xml");
    assert!(
      robots
        .render("https://example.com")
        .contains("Sitemap: https://example.com/sitemap.xml")
    );
    assert!(
      robots
        .render("https://other.example")
        .contains("Sitemap: https://other.example/sitemap.xml")
    );
  }

  /// Named groups are rendered as their own blocks, because a crawler obeys the
  /// most specific block that names it and nothing else.
  #[test]
  fn keeps_named_agents_in_their_own_block() {
    let rendered = RobotsTxt::new()
      .disallow("/s/")
      .group(RobotsGroup::for_agents(["GPTBot", "CCBot"]).disallow("/"))
      .render("");

    assert_eq!(
      rendered,
      "User-agent: *\n\
       Disallow: /s/\n\
       \n\
       User-agent: GPTBot\n\
       User-agent: CCBot\n\
       Disallow: /\n"
    );
  }

  /// An empty document is still a file, and what it says is "nothing is off
  /// limits" — the same thing a site without a `robots.txt` says.
  #[test]
  fn renders_an_empty_document_as_a_permit_all() {
    assert_eq!(RobotsTxt::new().render(""), "User-agent: *\nDisallow:\n");
  }

  /// A rule that cannot mean what it says is dropped rather than rendered. The
  /// `#` case is the one worth the test: comments run to the end of the line, so
  /// `Disallow: /private#draft` would forbid `/private` — and a reviewer reading
  /// the config would believe something narrower had been forbidden.
  #[test]
  fn drops_rules_that_would_forbid_the_wrong_thing() {
    let rendered = RobotsTxt::new()
      .disallow("/private#draft")
      .disallow("relative/path")
      .disallow("/s/")
      .render("");

    assert_eq!(rendered, "User-agent: *\nDisallow: /s/\n");
  }

  /// Dropping every rule in a group must not leave a `User-agent:` line with
  /// nothing under it: that is a malformed block, and a crawler may stop reading
  /// the file where it starts.
  #[test]
  fn a_group_left_with_no_rules_still_renders_a_valid_block() {
    assert_eq!(
      RobotsTxt::new().disallow("nonsense").render(""),
      "User-agent: *\nDisallow:\n"
    );
  }

  /// A sitemap URL that is neither absolute nor rooted has nothing to resolve
  /// against, and a `Sitemap:` line pointing at a relative path is ignored by
  /// every crawler — better to leave it out than to look advertised.
  #[test]
  fn drops_a_sitemap_url_it_cannot_make_absolute() {
    assert!(
      !RobotsTxt::new()
        .sitemap("sitemap.xml")
        .render("https://example.com")
        .contains("Sitemap:")
    );
  }

  /// A site on two hostnames publishes one set of URLs, so both hosts point at
  /// the canonical sitemap — and only the canonical host advertises it, because
  /// a sitemap listing another host's URLs is cross-submission and is ignored
  /// unless the owner has proved ownership of both.
  #[test]
  fn advertises_the_sitemap_only_where_it_belongs() {
    let robots = RobotsTxt::new()
      .disallow("/s/")
      .sitemap("/sitemap.xml")
      .canonical_origin(CanonicalOrigin::fixed("https://blog.example.com"));

    let mut on_canonical = Request::default();
    on_canonical
      .headers_mut()
      .insert("host", "blog.example.com".parse().unwrap());
    let mut on_alias = Request::default();
    on_alias
      .headers_mut()
      .insert("host", "app.example.com".parse().unwrap());

    assert!(
      robots
        .render_for(&on_canonical)
        .contains("Sitemap: https://blog.example.com/sitemap.xml"),
      "the canonical host advertises it"
    );
    assert!(
      !robots.render_for(&on_alias).contains("Sitemap:"),
      "the alias does not — the file would not be its own to hand over"
    );
    assert!(
      robots.render_for(&on_alias).contains("Disallow: /s/"),
      "but the rules are the same on both: the alias serves the same pages, and a        crawler learns they are one page by fetching them and finding the same canonical"
    );
  }

  /// The config path has to reach the same document as the builder, because the
  /// whole point of deriving `Deserialize` is that an operator can move a policy
  /// into YAML without it meaning something else.
  #[test]
  fn reads_the_same_document_out_of_yaml() {
    let robots: RobotsTxt = serde_pretty_yaml::from_str(
      "comment: Staging.\ngroups:\n  - agents: [\"*\"]\n    disallow: [\"/\"]\n    crawl_delay: 10\n",
    )
    .expect("the document parses");

    assert_eq!(
      robots.render(""),
      "# Staging.\n\nUser-agent: *\nDisallow: /\nCrawl-delay: 10\n"
    );
  }
}
