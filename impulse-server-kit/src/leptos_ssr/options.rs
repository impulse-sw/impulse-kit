//! Configuration for the SSR adapter.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// SEO defaults consumed by the SSR HTML prefix builder.
///
/// These defaults are written into the rendered `<head>` only when the
/// rendered tree does not already supply a corresponding `leptos_meta`
/// element. The fields map to standard SEO/social metadata.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SeoDefaults {
  /// Title template with `{}` substituted by the page-specific title.
  /// Example: `"{} | Impulse"`.
  #[serde(default)]
  pub title_template: Option<String>,
  /// Default `<title>` used when the rendered tree does not set one.
  #[serde(default)]
  pub default_title: Option<String>,
  /// Default `<meta name="description">` content.
  #[serde(default)]
  pub description: Option<String>,
  /// Default `og:image` URL.
  #[serde(default)]
  pub og_image: Option<String>,
  /// Canonical base URL; the request path is appended to form
  /// `<link rel="canonical">`.
  #[serde(default)]
  pub canonical_base: Option<String>,
  /// Default Twitter handle for `twitter:site`.
  #[serde(default)]
  pub twitter_handle: Option<String>,
  /// Default `robots` directive (e.g. `"index,follow"`).
  #[serde(default)]
  pub robots: Option<String>,
  /// Default `<html lang>` value (e.g. `"en"`).
  #[serde(default)]
  pub locale: Option<String>,
}

/// What to do when the SSR handler cannot produce a useful response.
#[derive(Clone, Default)]
pub enum FallbackStrategy {
  /// Render a minimal, JS-free 404/500 page from `impulse-error-pages`.
  #[default]
  ErrorPage,
  /// User-supplied fallback HTML.
  Custom(Arc<dyn Fn() -> String + Send + Sync>),
}

impl std::fmt::Debug for FallbackStrategy {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      FallbackStrategy::ErrorPage => f.write_str("ErrorPage"),
      FallbackStrategy::Custom(_) => f.write_str("Custom(<fn>)"),
    }
  }
}

/// SSR adapter options.
///
/// Build via [`LeptosOptions::from_generic_values`] when the configuration
/// already lives in YAML, or construct manually for tests.
#[derive(Clone, Debug)]
pub struct LeptosOptions {
  /// Bundle name. Used to compute URLs for the wasm/JS/CSS bundle, e.g.
  /// `/<site_pkg_dir>/<output_name>.js`.
  pub output_name: String,
  /// Root directory of the front-end build. The asset router serves files
  /// from this directory (e.g. `target/site`).
  pub site_root: PathBuf,
  /// Sub-directory under `site_root` that contains the wasm bundle.
  /// Defaults to `pkg`.
  pub site_pkg_dir: String,
  /// URL prefix under which `#[server]` functions are mounted. Used by
  /// [`crate::leptos_ssr::server_fn_router`] when building the Salvo router.
  pub server_fn_prefix: String,
  /// SEO defaults injected into the rendered HTML prefix.
  pub seo_defaults: SeoDefaults,
  /// Fallback when rendering yields no content.
  pub fallback: FallbackStrategy,
  /// When `true`, the rendered HTML embeds the hydration bootstrap
  /// `<script>` and links the wasm bundle. Set to `true` when shipping a
  /// matching `hydrate`-mode wasm bundle.
  pub include_hydration_script: bool,
  /// Streaming mode used by the SSR handler.
  pub stream_mode: super::handler::SsrStreamMode,
}

impl Default for LeptosOptions {
  fn default() -> Self {
    Self {
      output_name: String::new(),
      site_root: PathBuf::from("target/site"),
      site_pkg_dir: "pkg".to_string(),
      server_fn_prefix: "/api".to_string(),
      seo_defaults: SeoDefaults::default(),
      fallback: FallbackStrategy::ErrorPage,
      include_hydration_script: false,
      stream_mode: super::handler::SsrStreamMode::InOrder,
    }
  }
}

impl LeptosOptions {
  /// Build options from the generic YAML configuration. Missing fields fall
  /// back to sensible defaults.
  pub fn from_generic_values(g: &crate::setup::GenericValues) -> Self {
    let mut opts = LeptosOptions::default();
    if let Some(p) = &g.frontend_dist_path {
      opts.site_root = p.clone();
    }
    if let Some(name) = &g.leptos_output_name {
      opts.output_name = name.clone();
    } else {
      opts.output_name = g.app_name.clone();
    }
    if let Some(pkg) = &g.leptos_site_pkg_dir {
      opts.site_pkg_dir = pkg.clone();
    }
    if let Some(prefix) = &g.leptos_server_fn_prefix {
      opts.server_fn_prefix = prefix.clone();
    }
    if let Some(seo) = &g.leptos_seo {
      opts.seo_defaults = seo.clone();
    }
    opts
  }
}
