//! Server-side rendering contexts.
//!
//! These types are populated by `impulse-server-kit::leptos_ssr` for each
//! incoming request and provided into the Leptos `Owner`/context tree before
//! rendering. UI Kit components and helpers consume them via
//! [`leptos::context::use_context`].

use std::sync::{Arc, Mutex};

/// Per-request URL information available during SSR.
///
/// Populated by the SSR handler from the incoming Salvo request.
#[derive(Clone, Debug, Default)]
pub struct RequestUrlCtx {
  /// URL scheme without the trailing colon, e.g. `"http"` or `"https"`.
  pub scheme: String,
  /// Host with optional port, e.g. `"example.com"` or `"127.0.0.1:8801"`.
  pub host: String,
  /// Request path, e.g. `"/about"`.
  pub path: String,
  /// Raw query string without the leading `?`.
  pub query: Option<String>,
}

/// Initial theme resolved on the server from the `impulse_theme` cookie.
///
/// `ThemeProvider` consumes this to render `<html class="dark">` (or `light`)
/// without flicker on the first paint.
#[derive(Clone, Debug, Default)]
pub struct InitialTheme(pub Option<String>);

/// Mutable response options that the rendered tree can set via context.
///
/// The SSR handler inspects this after rendering finishes and short-circuits
/// the response if a redirect is requested or a non-default status code is set.
#[derive(Clone, Default)]
pub struct LeptosResponseOptions(Arc<Mutex<RespOptsInner>>);

#[derive(Default)]
struct RespOptsInner {
  status: Option<u16>,
  redirect: Option<String>,
  headers: Vec<(String, String)>,
}

impl LeptosResponseOptions {
  /// Set the HTTP status code that the SSR handler should send.
  pub fn set_status(&self, code: u16) {
    if let Ok(mut inner) = self.0.lock() {
      inner.status = Some(code);
    }
  }

  /// Request that the SSR handler issues an HTTP redirect to `url`.
  pub fn set_redirect(&self, url: impl Into<String>) {
    if let Ok(mut inner) = self.0.lock() {
      inner.redirect = Some(url.into());
    }
  }

  /// Append a custom response header.
  pub fn insert_header(&self, name: impl Into<String>, value: impl Into<String>) {
    if let Ok(mut inner) = self.0.lock() {
      inner.headers.push((name.into(), value.into()));
    }
  }

  /// Read the configured status code, if any.
  pub fn status(&self) -> Option<u16> {
    self.0.lock().ok().and_then(|i| i.status)
  }

  /// Read the configured redirect target, if any.
  pub fn redirect(&self) -> Option<String> {
    self.0.lock().ok().and_then(|i| i.redirect.clone())
  }

  /// Drain the configured custom headers.
  pub fn take_headers(&self) -> Vec<(String, String)> {
    self.0.lock().map(|mut i| std::mem::take(&mut i.headers)).unwrap_or_default()
  }
}

