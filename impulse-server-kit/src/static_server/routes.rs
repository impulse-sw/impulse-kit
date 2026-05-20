//! Salvo handlers and router builders for serving static files.
//!
//! Three handler flavours are exposed:
//!
//! - [`StaticRouter`] — wildcard handler that falls back to `index.html` for
//!   unknown paths (SPA-style routing).
//! - [`NoRedirectStaticRouter`] — same as `StaticRouter` but lets unmatched
//!   paths fall through to the next handler. Suited for SSR setups where the
//!   asset router sits in front of an SSR catch-all.
//! - [`ProvidedRoutesStaticRouter`] — serves only files whose names appear in
//!   an explicit allow-list; everything else falls through.
//!
//! All handlers emit `tracing` spans/events so cache hits, disk reads and
//! 404s are observable — this is the main reason to use them over Salvo's
//! built-in `StaticDir`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::prelude::*;
use crate::salvo::FlowCtrl;
use crate::static_server::caching::{CacheMap, cache_runner, send_file, send_html};

const LOCAL_FRONTEND_DISTRIBUTABLE: &str = "dist";
const CONTAINER_FRONTEND_DISTRIBUTABLE: &str = "/usr/local/frontend-dist";

async fn common_handle(
  cacher: &Option<Arc<Mutex<CacheMap>>>,
  dist_path: &Path,
  filename: String,
  filepath: PathBuf,
  req: &mut Request,
  depot: &mut Depot,
  res: &mut Response,
) {
  use salvo::Writer;

  if filename.contains(".") {
    match filename.split('.').collect::<Vec<_>>().last() {
      Some(&"html") => {
        if let Err(e) = send_html(cacher, req, depot, res, &filename, &filepath).await {
          tracing::warn!(file = %filename, "html not found");
          e.with_public("There is no such file!")
            .with_404()
            .write(req, depot, res)
            .await;
        }
      }
      _ => {
        if let Err(e) = send_file(cacher, req, depot, res, &filename, &filepath).await {
          tracing::warn!(file = %filename, "file not found");
          e.with_public("There is no such file!")
            .with_404()
            .write(req, depot, res)
            .await;
        }
      }
    }
  } else if filepath.exists() {
    if let Err(e) = send_file(cacher, req, depot, res, &filename, &filepath).await {
      tracing::warn!(file = %filename, "file not found");
      e.with_public("There is no such file!")
        .with_404()
        .write(req, depot, res)
        .await;
    }
  } else if let Err(e) = send_html(cacher, req, depot, res, "index.html", &dist_path.join("index.html")).await {
    tracing::warn!(file = %filename, "index.html fallback not found");
    e.with_public("There is no such file!")
      .with_404()
      .write(req, depot, res)
      .await;
  }
}

/// Wildcard static-file handler that falls back to `index.html`.
pub struct StaticRouter {
  path: PathBuf,
  cacher: Option<Arc<Mutex<CacheMap>>>,
}

/// Wildcard handler that returns 404 (via fall-through) for missing files
/// instead of redirecting to `index.html`.
pub struct NoRedirectStaticRouter {
  path: PathBuf,
  cacher: Option<Arc<Mutex<CacheMap>>>,
}

/// Wildcard handler that only serves files whose names are in `possible_routes`.
pub struct ProvidedRoutesStaticRouter {
  path: PathBuf,
  possible_routes: Vec<String>,
  cacher: Option<Arc<Mutex<CacheMap>>>,
}

impl StaticRouter {
  /// Build a router rooted at `path` without in-memory caching.
  pub fn new(path: impl AsRef<Path>) -> MResult<Self> {
    if !path.as_ref().exists() {
      ServerError::from_private_str(format!("There is no such folder as {:?}!", path.as_ref()))
        .with_500()
        .bail()?;
    }

    Ok(Self {
      path: path.as_ref().to_owned(),
      cacher: None,
    })
  }

  /// Build a router rooted at `path` with in-memory caching plus a filesystem
  /// watcher that invalidates entries on change.
  pub fn new_with_cacher(path: impl AsRef<Path>) -> MResult<Self> {
    if !path.as_ref().exists() {
      ServerError::from_private_str(format!("There is no such folder as {:?}!", path.as_ref()))
        .with_500()
        .bail()?;
    }

    let cacher = CacheMap::new();
    tokio::task::spawn({
      let path = path.as_ref().to_path_buf();
      let cacher = cacher.clone();

      async move {
        if let Err(e) = cache_runner(&path, cacher).await {
          tracing::error!("{e}");
        }
      }
    });

    Ok(Self {
      path: path.as_ref().to_owned(),
      cacher: Some(cacher),
    })
  }

  /// Convert into a [`NoRedirectStaticRouter`] that falls through for missing files.
  pub fn with_no_redirect(self) -> NoRedirectStaticRouter {
    NoRedirectStaticRouter {
      path: self.path,
      cacher: self.cacher,
    }
  }

  /// Convert into a [`ProvidedRoutesStaticRouter`] serving only files in `routes`.
  pub fn with_routes_list(self, routes: Vec<String>) -> ProvidedRoutesStaticRouter {
    ProvidedRoutesStaticRouter {
      path: self.path,
      possible_routes: routes,
      cacher: self.cacher,
    }
  }
}

#[salvo::async_trait]
impl salvo::Handler for StaticRouter {
  #[tracing::instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
  async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, _: &mut FlowCtrl) {
    let mut filename = req.param::<String>("rest_path").unwrap_or(String::from("index.html"));
    if filename.is_empty() {
      filename = String::from("index.html");
    }
    let filepath = self.path.join(&filename);
    common_handle(&self.cacher, &self.path, filename, filepath, req, depot, res).await;
  }
}

#[salvo::async_trait]
impl salvo::Handler for NoRedirectStaticRouter {
  #[tracing::instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
  async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, flow: &mut salvo::FlowCtrl) {
    let mut filename = req.param::<String>("rest_path").unwrap_or(String::from("index.html"));
    if filename.is_empty() {
      filename = String::from("index.html");
    }
    let filepath = self.path.join(&filename);

    if !filepath.exists() {
      tracing::debug!(file = %filename, "static asset missing, falling through");
      while flow.has_next() {
        flow.call_next(req, depot, res).await;
      }
      return;
    }
    common_handle(&self.cacher, &self.path, filename, filepath, req, depot, res).await;
  }
}

#[salvo::async_trait]
impl salvo::Handler for ProvidedRoutesStaticRouter {
  #[tracing::instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
  async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, flow: &mut salvo::FlowCtrl) {
    let mut filename = req.param::<String>("rest_path").unwrap_or(String::from("index.html"));
    if filename.is_empty() {
      filename = String::from("index.html");
    }
    let filepath = self.path.join(&filename);

    if !filepath.exists() || !self.possible_routes.iter().any(|pr| pr.as_str().eq(filename.as_str())) {
      tracing::debug!(file = %filename, "static asset not in allow-list, falling through");
      while flow.has_next() {
        flow.call_next(req, depot, res).await;
      }
      return;
    }
    common_handle(&self.cacher, &self.path, filename, filepath, req, depot, res).await;
  }
}

/// Router serving files from `dist`, with `index.html` fallback for unknown paths.
///
/// `dist` must contain an `index.html`.
pub fn frontend_router_from_given_dist(dist: &Path) -> MResult<Router> {
  Ok(Router::with_path("{**rest_path}").get(StaticRouter::new_with_cacher(dist)?))
}

/// Router that serves files from `./dist` (development) or `/usr/local/frontend-dist` (container).
///
/// Returns an error if neither directory exists.
pub fn frontend_router() -> MResult<Router> {
  let dist = StaticRouter::new_with_cacher(LOCAL_FRONTEND_DISTRIBUTABLE)
    .or(StaticRouter::new_with_cacher(CONTAINER_FRONTEND_DISTRIBUTABLE))?;

  Ok(Router::with_path("{**rest_path}").get(dist))
}

/// Wildcard asset router that serves files from `dist` without falling back to
/// `index.html`. Intended for SSR setups where unhandled paths must reach the
/// SSR handler rather than yield the SPA shell.
pub fn assets_only_router_from(dist: &Path) -> MResult<Router> {
  Ok(Router::with_path("{**rest_path}").get(StaticRouter::new_with_cacher(dist)?.with_no_redirect()))
}
