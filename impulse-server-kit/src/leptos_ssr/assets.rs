//! Static-asset router for SSR.
//!
//! Serves the wasm bundle, CSS, fonts, favicon and other static files from
//! `site_root` under predictable URL prefixes. Unlike `frontend_router` from
//! `impulse-static-server`, this router does NOT fall back to `index.html` —
//! unmatched paths must be handled by the SSR handler.

use std::path::Path;

use salvo::Router;
use salvo::serve_static::StaticDir;

/// Build a Salvo router that serves static assets from `site_root`.
///
/// `site_pkg_dir` is the sub-directory under `site_root` where the wasm/JS/CSS
/// bundle lives (typically `pkg`). The router exposes:
///
/// - `/<site_pkg_dir>/{**rest}` — wasm/JS/CSS bundle.
/// - `/assets/{**rest}` — generic static assets (images, fonts, ...).
/// - `/favicon.ico`, `/robots.txt`, `/sitemap.xml` — top-level assets.
pub fn assets_only_router(site_root: &Path, site_pkg_dir: &str) -> Router {
  let pkg_path = site_root.join(site_pkg_dir);
  let assets_path = site_root.join("assets");

  let pkg_route = format!("/{}/{{**path}}", site_pkg_dir.trim_matches('/'));

  Router::new()
    .push(Router::with_path(&pkg_route).get(StaticDir::new([pkg_path]).auto_list(false)))
    .push(Router::with_path("/assets/{**path}").get(StaticDir::new([assets_path]).auto_list(false)))
    .push(Router::with_path("/favicon.ico").get(StaticDir::new([site_root.to_path_buf()]).auto_list(false)))
    .push(Router::with_path("/robots.txt").get(StaticDir::new([site_root.to_path_buf()]).auto_list(false)))
    .push(Router::with_path("/sitemap.xml").get(StaticDir::new([site_root.to_path_buf()]).auto_list(false)))
}
