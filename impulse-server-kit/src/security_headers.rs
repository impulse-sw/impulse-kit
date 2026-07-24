//! Conservative security response headers.
//!
//! When [`crate::startup::get_root_router_autoinject`] is used, every
//! response gets a default set of security headers unless the operator
//! opts out via the YAML config (`security_headers:` block).
//!
//! Defaults:
//!
//! | Header                       | Value                                  |
//! |------------------------------|----------------------------------------|
//! | `Strict-Transport-Security`  | `max-age=31536000; includeSubDomains`  |
//! | `X-Content-Type-Options`     | `nosniff`                              |
//! | `X-Frame-Options`            | `SAMEORIGIN`                           |
//! | `Referrer-Policy`            | `strict-origin-when-cross-origin`      |
//!
//! `Content-Security-Policy`, `Permissions-Policy` and
//! `Cross-Origin-Opener-Policy` are off by default because they are
//! highly application-specific.
//!
//! Set a field to `null` in YAML to disable a specific header without
//! turning the whole hoop off:
//!
//! ```yaml
//! security_headers:
//!   hsts: null  # acceptable for `http_localhost` dev setups
//! ```
//!
//! Whatever the YAML says, HSTS is *also* skipped automatically when
//! the server starts in `http_localhost` or `unsafe_http` mode — sending
//! `Strict-Transport-Security` over plain HTTP would pin localhost in
//! the developer's browser for a year. Headers already present on the
//! response are left untouched, so per-route overrides win.

use std::sync::Arc;

use salvo::http::HeaderValue;
use salvo::http::header::{
  CONTENT_SECURITY_POLICY, HeaderName, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS,
  X_FRAME_OPTIONS,
};
use salvo::prelude::*;
use serde::Deserialize;

use crate::setup::GenericServerState;

/// YAML-configurable values for the security headers hoop.
///
/// All fields are populated with sensible defaults via [`Default`];
/// `#[serde(default)]` on the struct means a missing block in YAML
/// produces the same defaults. Explicit `null` for any optional field
/// disables that header.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct SecurityHeadersOptions {
  /// Master switch. Set to `false` to skip the hoop entirely.
  pub enabled: bool,
  /// `Strict-Transport-Security`. Defaults to one year + subdomains.
  /// Forced off when the listener is HTTP-only (see module docs).
  pub hsts: Option<String>,
  /// `X-Content-Type-Options`. Default `nosniff`.
  pub x_content_type_options: Option<String>,
  /// `X-Frame-Options`. Default `SAMEORIGIN`.
  pub x_frame_options: Option<String>,
  /// `Referrer-Policy`. Default `strict-origin-when-cross-origin`.
  pub referrer_policy: Option<String>,
  /// `Content-Security-Policy`. Off by default — apps must opt in.
  pub content_security_policy: Option<String>,
  /// `Permissions-Policy`. Off by default.
  pub permissions_policy: Option<String>,
  /// `Cross-Origin-Opener-Policy`. Off by default.
  pub cross_origin_opener_policy: Option<String>,
}

impl Default for SecurityHeadersOptions {
  fn default() -> Self {
    Self {
      enabled: true,
      hsts: Some("max-age=31536000; includeSubDomains".to_string()),
      x_content_type_options: Some("nosniff".to_string()),
      x_frame_options: Some("SAMEORIGIN".to_string()),
      referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
      content_security_policy: None,
      permissions_policy: None,
      cross_origin_opener_policy: None,
    }
  }
}

/// Salvo hoop that injects the configured security headers into every
/// response. Build via [`SecurityHeaders::new`] and attach with
/// `router.hoop(...)`.
pub struct SecurityHeaders {
  always: Arc<Vec<(HeaderName, HeaderValue)>>,
  hsts: Option<(HeaderName, HeaderValue)>,
}

fn parse(name: HeaderName, value: &Option<String>) -> Option<(HeaderName, HeaderValue)> {
  let raw = value.as_deref()?;
  match HeaderValue::from_str(raw) {
    Ok(v) => Some((name, v)),
    Err(e) => {
      tracing::warn!(error = %e, header = %name, raw = %raw, "ignoring invalid security header value");
      None
    }
  }
}

impl SecurityHeaders {
  /// Build a hoop from `options`. Invalid header values are logged and
  /// skipped — they never block startup.
  pub fn new(options: &SecurityHeadersOptions) -> Self {
    let mut always = Vec::new();
    always.extend(parse(X_CONTENT_TYPE_OPTIONS, &options.x_content_type_options));
    always.extend(parse(X_FRAME_OPTIONS, &options.x_frame_options));
    always.extend(parse(REFERRER_POLICY, &options.referrer_policy));
    always.extend(parse(CONTENT_SECURITY_POLICY, &options.content_security_policy));
    always.extend(parse(
      HeaderName::from_static("permissions-policy"),
      &options.permissions_policy,
    ));
    always.extend(parse(
      HeaderName::from_static("cross-origin-opener-policy"),
      &options.cross_origin_opener_policy,
    ));
    let hsts = parse(STRICT_TRANSPORT_SECURITY, &options.hsts);
    Self {
      always: Arc::new(always),
      hsts,
    }
  }

  fn apply(&self, hsts_allowed: bool, headers: &mut salvo::http::HeaderMap) {
    for (name, value) in self.always.iter() {
      if !headers.contains_key(name) {
        headers.insert(name.clone(), value.clone());
      }
    }
    if hsts_allowed
      && let Some((name, value)) = &self.hsts
      && !headers.contains_key(name)
    {
      headers.insert(name.clone(), value.clone());
    }
  }
}

#[salvo::async_trait]
impl Handler for SecurityHeaders {
  async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    ctrl.call_next(req, depot, res).await;

    // HSTS over plain HTTP is wrong (RFC 6797) and pinning a cleartext
    // listener for a year is a footgun developers hit on first run; only
    // advertise it when the response is served over TLS — HTTP/3 over QUIC,
    // or HTTPS over HTTP/1.1 / HTTP/2 when a certificate is configured.
    let hsts_allowed = depot
      .get_typed::<GenericServerState>()
      .map(|s| s.uses_https())
      .unwrap_or(true);

    self.apply(hsts_allowed, res.headers_mut());
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use salvo::http::HeaderMap;

  fn build(opts: SecurityHeadersOptions) -> SecurityHeaders {
    SecurityHeaders::new(&opts)
  }

  #[test]
  fn defaults_emit_the_four_baseline_headers() {
    let mut headers = HeaderMap::new();
    build(SecurityHeadersOptions::default()).apply(true, &mut headers);

    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(headers["x-frame-options"], "SAMEORIGIN");
    assert_eq!(headers["referrer-policy"], "strict-origin-when-cross-origin");
    assert_eq!(
      headers["strict-transport-security"],
      "max-age=31536000; includeSubDomains"
    );
    assert!(!headers.contains_key("content-security-policy"));
    assert!(!headers.contains_key("permissions-policy"));
  }

  #[test]
  fn skips_hsts_over_plain_http() {
    let mut headers = HeaderMap::new();
    build(SecurityHeadersOptions::default()).apply(false, &mut headers);
    assert!(!headers.contains_key("strict-transport-security"));
    // Other headers still go through.
    assert_eq!(headers["x-content-type-options"], "nosniff");
  }

  #[test]
  fn null_disables_specific_header() {
    let opts = SecurityHeadersOptions {
      x_frame_options: None,
      ..Default::default()
    };
    let mut headers = HeaderMap::new();
    build(opts).apply(true, &mut headers);
    assert!(!headers.contains_key("x-frame-options"));
    assert_eq!(headers["x-content-type-options"], "nosniff");
  }

  #[test]
  fn does_not_overwrite_existing_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    build(SecurityHeadersOptions::default()).apply(true, &mut headers);
    // Per-route override wins.
    assert_eq!(headers["x-frame-options"], "DENY");
  }

  #[test]
  fn optional_headers_appear_when_set() {
    let opts = SecurityHeadersOptions {
      content_security_policy: Some("default-src 'self'".to_string()),
      permissions_policy: Some("camera=()".to_string()),
      cross_origin_opener_policy: Some("same-origin".to_string()),
      ..Default::default()
    };
    let mut headers = HeaderMap::new();
    build(opts).apply(true, &mut headers);
    assert_eq!(headers["content-security-policy"], "default-src 'self'");
    assert_eq!(headers["permissions-policy"], "camera=()");
    assert_eq!(headers["cross-origin-opener-policy"], "same-origin");
  }

  #[test]
  fn invalid_value_is_dropped_not_panicked() {
    let opts = SecurityHeadersOptions {
      x_frame_options: Some("bad\nvalue".to_string()),
      ..Default::default()
    };
    let mut headers = HeaderMap::new();
    build(opts).apply(true, &mut headers);
    // The bad value is dropped; the rest still works.
    assert!(!headers.contains_key("x-frame-options"));
    assert_eq!(headers["x-content-type-options"], "nosniff");
  }
}
