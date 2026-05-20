//! Static-asset serving for SSR.
//!
//! The SSR router serves a single `dist` directory: every file under it is
//! served at the URL that mirrors its on-disk path, and unknown paths fall
//! through to the SSR renderer.
//!
//! Built on top of the [`static_server`](crate::static_server) handlers so
//! cache hits, disk reads and 404s are logged via `tracing` — unlike
//! `salvo::serve_static::StaticDir`, which is silent.

use std::path::Path;

use salvo::Router;

use crate::static_server::{NoRedirectStaticRouter, StaticRouter};

/// Build a Salvo handler that serves files from `site_root` without
/// `index.html` fallback. Returns `None` if `site_root` does not exist —
/// callers should treat that as a soft failure (typically: the bundle hasn't
/// been built yet) and proceed with the SSR-only setup.
pub(super) fn build_assets_handler(site_root: &Path) -> Option<NoRedirectStaticRouter> {
  match StaticRouter::new_with_cacher(site_root) {
    Ok(handler) => Some(handler.with_no_redirect()),
    Err(e) => {
      tracing::warn!(path = ?site_root, error = %e, "SSR dist directory missing; static assets will not be served");
      None
    }
  }
}

/// Build a Salvo router that serves the entire dist directory at any URL.
///
/// Returns an empty router when `site_root` does not exist so callers can
/// keep composing without a runtime error.
///
/// Prefer [`leptos_router`](super::handler::leptos_router) when wiring SSR,
/// since it already mounts asset serving alongside the SSR renderer.
pub fn assets_only_router(site_root: &Path) -> Router {
  match build_assets_handler(site_root) {
    Some(handler) => Router::with_path("{**rest_path}").get(handler),
    None => Router::new(),
  }
}
