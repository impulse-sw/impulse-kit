use cc_server_kit::cc_utils;
use cc_server_kit::cc_utils::prelude::*;
use cc_server_kit::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::caching::{CacheMap, cache_runner};

const LOCAL_FRONTEND_DISTRIBUTABLE: &str = "dist";
const CONTAINER_FRONTEND_DISTRIBUTABLE: &str = "/usr/local/frontend-dist";

/// Custom static router.
pub struct CustomStaticRouter {
  path: PathBuf,
  cacher: Option<Arc<CacheMap>>,
}

impl CustomStaticRouter {
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
  pub fn new_with_cacher(path: &Path) -> MResult<Self> {
    if !path.exists() {
      ServerError::from_private_str(format!("There is no such folder as {path:?}!"))
        .with_500()
        .bail()?;
    }

    let cacher = CacheMap::new();
    tokio::task::spawn_local({
      let path = path.to_path_buf();
      let cacher = cacher.clone();

      async move {
        if let Err(e) = cache_runner(&path, cacher).await {
          tracing::error!("{e}");
        }
      }
    });

    Ok(Self {
      path: path.to_owned(),
      cacher: Some(cacher),
    })
  }
}

#[salvo::async_trait]
impl salvo::Handler for CustomStaticRouter {
  #[tracing::instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
  async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, _: &mut salvo::FlowCtrl) {
    use salvo::Writer;

    let mut filename = req.param::<String>("rest_path").unwrap_or(String::from("index.html"));
    if filename.is_empty() {
      filename = String::from("index.html");
    }
    let filepath = self.path.join(&filename);
    if filepath.exists() && filename.contains(".") {
      match filename.split('.').collect::<Vec<_>>().last() {
        Some(&"html") => {
          if let Some(cacher) = self.cacher.as_ref()
            && let Ok(Some(data)) = cacher.fetch(&filepath)
          {
            let site = String::from_utf8_lossy_owned(data);
            html!(site).unwrap().write(req, depot, res).await;
          } else if let Ok(site) = tokio::fs::read_to_string(&filepath).await {
            if let Some(cacher) = self.cacher.as_ref() {
              let _ = cacher.upsert(&filepath, site.as_bytes().to_vec());
            }
            html!(site).unwrap().write(req, depot, res).await;
          } else {
            ServerError::from_public("There is no such file!")
              .with_404()
              .write(req, depot, res)
              .await;
          }
        }
        _ => file_upload!(filepath, filename).unwrap().write(req, depot, res).await,
      }
    } else if !filename.contains(".")
      && let Ok(site) = tokio::fs::read_to_string(&self.path.join("index.html")).await
    {
      html!(site).unwrap().write(req, depot, res).await;
    } else {
      ServerError::from_public("There is no such file!")
        .with_404()
        .write(req, depot, res)
        .await;
    }
  }
}

/// Static router with given `dist` path.
///
/// Returns error if dist folder doesn't exist.
///
/// Note that your `dist` folder must contains `index.html` file.
#[allow(unused)]
pub fn frontend_router_from_given_dist(dist: &Path) -> MResult<Router> {
  Ok(Router::with_path("{**rest_path}").get(CustomStaticRouter::new(dist)?))
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
  let dist = CustomStaticRouter::new(LOCAL_FRONTEND_DISTRIBUTABLE)
    .or(CustomStaticRouter::new(CONTAINER_FRONTEND_DISTRIBUTABLE))?;

  Ok(Router::with_path("{**rest_path}").get(dist))
}
