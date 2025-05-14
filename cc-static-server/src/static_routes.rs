use cc_server_kit::cc_utils;
use cc_server_kit::cc_utils::prelude::*;
use cc_server_kit::prelude::*;
use std::path::{Path, PathBuf};

const LOCAL_FRONTEND_DISTRIBUTABLE: &str = "dist";
const CONTAINER_FRONTEND_DISTRIBUTABLE: &str = "/usr/local/frontend-dist";

pub async fn get_filepath_from_dist(filename: impl Into<String>) -> MResult<String> {
  let filename = filename.into();
  tracing::debug!("Trying to get access to {}", filename);

  let filepath = PathBuf::from(CONTAINER_FRONTEND_DISTRIBUTABLE).join(&filename);
  if tokio::fs::try_exists(&filepath).await.is_ok_and(|v| v) {
    return Ok(filepath.to_string_lossy().to_string());
  } else {
    tracing::debug!("There is no such file as {:?}", filepath);
  }
  let mut filepath = std::env::current_exe().map_err(|e| ServerError::from_private(e).with_500())?;
  filepath.pop();
  let filepath = filepath.join(LOCAL_FRONTEND_DISTRIBUTABLE).join(&filename);
  if tokio::fs::try_exists(&filepath).await.is_ok_and(|v| v) {
    return Ok(filepath.to_string_lossy().to_string());
  } else {
    tracing::debug!("There is no such file as {:?}", filepath);
  }

  ServerError::from_public(format!(r#"Can't open file "{}""#, filename))
    .with_404()
    .bail()
}

pub async fn get_from_dist(filename: impl Into<String>) -> MResult<File> {
  let filename = filename.into();
  let filepath = get_filepath_from_dist(&filename).await?;
  file_upload!(filepath.into(), filename)
}

#[handler]
#[tracing::instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
pub async fn frontend(req: &Request) -> MResult<Html> {
  let filepath = get_filepath_from_dist("index.html").await?;
  let site = tokio::fs::read_to_string(&filepath)
    .await
    .map_err(|e| ServerError::from_private(e).with_500())?;
  html!(site)
}

#[handler]
#[tracing::instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
pub async fn get_uikit_app_internals(req: &Request) -> MResult<AnyOf> {
  let rest_path = req
    .param::<String>("rest_path")
    .ok_or(ServerError::from_public("Can't get the rest path.").with_400())?;
  match get_from_dist(rest_path.as_str()).await {
    Ok(file) => Ok(AnyOf::File(file)),
    Err(_) => {
      if rest_path.contains(".") {
        ServerError::from_public("There is no such file!").with_404().bail()?;
      }
      match frontend::frontend(req).await {
        Ok(html) => Ok(AnyOf::Html(html)),
        Err(e) => Err(e.with_404()),
      }
    }
  }
}

enum AnyOf {
  Html(Html),
  File(File),
}

#[salvo::async_trait]
impl salvo::Writer for AnyOf {
  async fn write(self, req: &mut Request, depot: &mut Depot, res: &mut salvo::Response) {
    match self {
      AnyOf::Html(html) => html.write(req, depot, res).await,
      AnyOf::File(file) => file.write(req, depot, res).await,
    }
  }
}

/// Static router.
///
/// All that you need to include your app internals inside your application.
///
/// Note that `cc-static-server` serves only files from `dist` or `/usr/local/frontend-dist` folders.
/// Make sure that `dist` folder is located in one folder with your application, not in current working
/// directory provided by `$ pwd`.
pub fn frontend_router() -> Router {
  Router::with_path("{**rest_path}").get(get_uikit_app_internals)
}

/// Custom static router.
struct CustomStaticRouter {
  path: PathBuf,
}

impl CustomStaticRouter {
  fn new(path: impl AsRef<Path>) -> Self {
    Self {
      path: path.as_ref().to_owned(),
    }
  }
}

#[salvo::async_trait]
impl salvo::Handler for CustomStaticRouter {
  #[tracing::instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
  async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, _: &mut salvo::FlowCtrl) {
    use salvo::Writer;

    let filename = req.param::<String>("rest_path").unwrap_or(String::from("index.html"));
    let filepath = self.path.join(&filename);
    if filepath.exists() {
      match filename.split('.').collect::<Vec<_>>().last() {
        Some(&"html") if let Ok(site) = tokio::fs::read_to_string(&filepath).await => {
          html!(site).unwrap().write(req, depot, res).await
        }
        _ => file_upload!(filepath, filename).unwrap().write(req, depot, res).await,
      }
    } else if !filename.contains(".")
      && let Ok(site) = tokio::fs::read_to_string(&filepath).await
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
/// Note that your `dist` folder must contains `index.html` file.
#[allow(unused)]
pub fn frontend_router_from_given_dist(dist: &Path) -> Router {
  Router::with_path("{**rest_path}").get(CustomStaticRouter::new(dist))
}
