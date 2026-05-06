use impulse_server_kit::impulse_utils::prelude::*;
use impulse_server_kit::prelude::*;
use impulse_server_kit::salvo::FlowCtrl;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::caching::{CacheMap, cache_runner, send_file, send_html};

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
          e.with_public("There is no such file!")
            .with_404()
            .write(req, depot, res)
            .await;
        }
      }
      _ => {
        if let Err(e) = send_file(cacher, req, depot, res, &filename, &filepath).await {
          e.with_public("There is no such file!")
            .with_404()
            .write(req, depot, res)
            .await;
        }
      }
    }
  } else if filepath.exists() {
    if let Err(e) = send_file(cacher, req, depot, res, &filename, &filepath).await {
      e.with_public("There is no such file!")
        .with_404()
        .write(req, depot, res)
        .await;
    }
  } else if let Err(e) = send_html(cacher, req, depot, res, "index.html", &dist_path.join("index.html")).await {
    e.with_public("There is no such file!")
      .with_404()
      .write(req, depot, res)
      .await;
  }
}

/// Static router.
pub struct StaticRouter {
  path: PathBuf,
  cacher: Option<Arc<Mutex<CacheMap>>>,
}

/// Static router with no redirections allowed.
///
/// It serves all files from given path, but if server requested non-existing file, it will not redirect it to `index.html`.
pub struct NoRedirectStaticRouter {
  path: PathBuf,
  cacher: Option<Arc<Mutex<CacheMap>>>,
}

/// Static router with only provided routes.
pub struct ProvidedRoutesStaticRouter {
  path: PathBuf,
  possible_routes: Vec<String>,
  cacher: Option<Arc<Mutex<CacheMap>>>,
}

impl StaticRouter {
  /// Create static router from given path.
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

  /// Create static router from given path with in-memory cacher.
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

  /// Disable redirect to `index.html` when no requested file found.
  pub fn with_no_redirect(self) -> NoRedirectStaticRouter {
    NoRedirectStaticRouter {
      path: self.path,
      cacher: self.cacher,
    }
  }

  /// Disable redirect and provide routes' list.
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
  async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, flow: &mut salvo::FlowCtrl) {
    let mut filename = req.param::<String>("rest_path").unwrap_or(String::from("index.html"));
    if filename.is_empty() {
      filename = String::from("index.html");
    }
    let filepath = self.path.join(&filename);

    if !filepath.exists() {
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
  async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, flow: &mut salvo::FlowCtrl) {
    let mut filename = req.param::<String>("rest_path").unwrap_or(String::from("index.html"));
    if filename.is_empty() {
      filename = String::from("index.html");
    }
    let filepath = self.path.join(&filename);

    if !filepath.exists() || !self.possible_routes.iter().any(|pr| pr.as_str().eq(filename.as_str())) {
      while flow.has_next() {
        flow.call_next(req, depot, res).await;
      }
      return;
    }
    common_handle(&self.cacher, &self.path, filename, filepath, req, depot, res).await;
  }
}

/// Static router with given `dist` path.
///
/// Returns error if dist folder doesn't exist.
///
/// Note that your `dist` folder must contains `index.html` file.
pub fn frontend_router_from_given_dist(dist: &Path) -> MResult<Router> {
  Ok(Router::with_path("{**rest_path}").get(StaticRouter::new_with_cacher(dist)?))
}

/// Static router.
///
/// All that you need to include your app internals inside your application.
///
/// Note that `cc-static-server` serves only files from `dist` or `/usr/local/frontend-dist` folders.
/// Make sure that `dist` folder is located in one folder with your application, not in current working
/// directory provided by `$ pwd`.
///
/// Returns error if neither `dist` nor `/usr/local/frontend-dist` folder exists.
pub fn frontend_router() -> MResult<Router> {
  let dist = StaticRouter::new_with_cacher(LOCAL_FRONTEND_DISTRIBUTABLE)
    .or(StaticRouter::new_with_cacher(CONTAINER_FRONTEND_DISTRIBUTABLE))?;

  Ok(Router::with_path("{**rest_path}").get(dist))
}

/// Asset-only router that serves files from `dist` without redirecting unknown
/// paths to `index.html`. Intended for SSR setups where unhandled paths must
/// fall through to the SSR handler instead of yielding the SPA shell.
///
/// Built on top of [`NoRedirectStaticRouter`] with in-memory caching enabled.
pub fn assets_only_router_from(dist: &Path) -> MResult<Router> {
  Ok(Router::with_path("{**rest_path}").get(StaticRouter::new_with_cacher(dist)?.with_no_redirect()))
}
