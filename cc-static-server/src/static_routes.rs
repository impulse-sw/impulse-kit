use cc_server_kit::cc_utils;
use cc_server_kit::cc_utils::prelude::*;
use cc_server_kit::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::caching::{CacheMap, CachedFile, cache_runner};

const LOCAL_FRONTEND_DISTRIBUTABLE: &str = "dist";
const CONTAINER_FRONTEND_DISTRIBUTABLE: &str = "/usr/local/frontend-dist";

/// Custom static router.
pub struct CustomStaticRouter {
  path: PathBuf,
  cacher: Option<Arc<Mutex<CacheMap>>>,
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

  /// Request given file either from in-memory cacher or from disk.
  pub async fn send_file(
    &self,
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    filename: &str,
    path: &Path,
  ) -> MResult<()> {
    use salvo::Writer;

    if let Some(cacher) = self.cacher.as_ref() {
      {
        let guard = cacher.lock().await;
        if let Ok(Some(cached)) = guard.fetch(path) {
          cached.send(req.headers(), res).await;
          return Ok(());
        }
      }
      let length = tokio::fs::metadata(path)
        .await
        .map_err(|e| ServerError::from_private(e).with_404())?
        .len();
      if length > 16 * 1024 * 1024 {
        file_upload!(path.to_path_buf(), filename.to_string())
          .write(req, depot, res)
          .await;
      } else {
        let cached = CachedFile::construct_from(filename, path, length).await?;
        {
          let mut guard = cacher.lock().await;
          guard.upsert(path, cached.clone())?;
        }
        cached.send(req.headers(), res).await;
      }
    } else {
      file_upload!(path.to_path_buf(), filename.to_string())
        .write(req, depot, res)
        .await;
    }
    Ok(())
  }

  /// Request given HTML page either from in-memory cacher or from disk.
  pub async fn send_html(
    &self,
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    filename: &str,
    path: &Path,
  ) -> MResult<()> {
    use salvo::Writer;

    if let Some(cacher) = self.cacher.as_ref() {
      let cached = {
        let guard = cacher.lock().await;
        guard.fetch(path)
      };
      if let Ok(Some(cached)) = cached {
        let site = String::from_utf8_lossy_owned(cached.bytes);
        html!(site).unwrap().write(req, depot, res).await;
        return Ok(());
      }
      let length = tokio::fs::metadata(path)
        .await
        .map_err(|e| ServerError::from_private(e).with_404())?
        .len();
      if length > 16 * 1024 * 1024 {
        let site = tokio::fs::read_to_string(path)
          .await
          .map_err(|e| ServerError::from_private(e).with_404())?;
        html!(site).unwrap().write(req, depot, res).await;
      } else {
        let cached = CachedFile::construct_from(filename, path, length).await?;
        {
          let mut guard = cacher.lock().await;
          guard.upsert(path, cached.clone())?;
        }
        let site = String::from_utf8_lossy_owned(cached.bytes);
        html!(site).unwrap().write(req, depot, res).await;
      }
    } else {
      let site = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| ServerError::from_private(e).with_404())?;
      html!(site).unwrap().write(req, depot, res).await;
    }
    Ok(())
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
    if filename.contains(".") {
      match filename.split('.').collect::<Vec<_>>().last() {
        Some(&"html") => {
          if let Err(e) = self.send_html(req, depot, res, &filename, &filepath).await {
            e.with_public("There is no such file!")
              .with_404()
              .write(req, depot, res)
              .await;
          }
        }
        _ => {
          if let Err(e) = self.send_file(req, depot, res, &filename, &filepath).await {
            e.with_public("There is no such file!")
              .with_404()
              .write(req, depot, res)
              .await;
          }
        }
      }
    } else if filepath.exists() {
      if let Err(e) = self.send_file(req, depot, res, &filename, &filepath).await {
        e.with_public("There is no such file!")
          .with_404()
          .write(req, depot, res)
          .await;
      }
    } else if let Err(e) = self
      .send_html(req, depot, res, "index.html", &self.path.join("index.html"))
      .await
    {
      e.with_public("There is no such file!")
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
  Ok(Router::with_path("{**rest_path}").get(CustomStaticRouter::new_with_cacher(dist)?))
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
  let dist = CustomStaticRouter::new_with_cacher(LOCAL_FRONTEND_DISTRIBUTABLE)
    .or(CustomStaticRouter::new_with_cacher(CONTAINER_FRONTEND_DISTRIBUTABLE))?;

  Ok(Router::with_path("{**rest_path}").get(dist))
}
