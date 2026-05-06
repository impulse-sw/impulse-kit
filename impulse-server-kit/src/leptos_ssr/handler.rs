//! Central SSR handler for Salvo.
//!
//! Renders the user's Leptos `App` component to a single HTML string, splices
//! `leptos_meta`'s collected `<head>` content into the prefix, applies any
//! response options the rendered tree set (status / redirect / headers), and
//! writes the response.
//!
//! This is the simple, non-streaming variant. Streaming and `<Suspense>`
//! support are reserved for the next iteration.

use std::sync::Arc;

use futures::StreamExt;
use leptos::IntoView;
use leptos::prelude::*;
use leptos::reactive::owner::Owner;
use leptos_meta::ServerMetaContext;
use salvo::http::StatusCode;
use salvo::prelude::*;
use tachys::view::RenderHtml;

use impulse_ui_kit::ssr::{InitialTheme, LeptosResponseOptions, RequestUrlCtx};

use super::options::LeptosOptions;
use super::prefix::{PrefixContext, build_html_prefix, build_html_suffix};
use super::theme::parse_theme_cookie;

/// Salvo handler that renders a Leptos application to HTML on every request.
///
/// Construct via [`leptos_router`] rather than instantiating directly.
pub struct LeptosSsrHandler<F, IV>
where
  F: Fn() -> IV + Clone + Send + Sync + 'static,
  IV: IntoView + 'static,
{
  opts: Arc<LeptosOptions>,
  app_fn: F,
}

impl<F, IV> LeptosSsrHandler<F, IV>
where
  F: Fn() -> IV + Clone + Send + Sync + 'static,
  IV: IntoView + 'static,
{
  /// Create a new handler with the given options and root component factory.
  pub fn new(opts: LeptosOptions, app_fn: F) -> Self {
    Self {
      opts: Arc::new(opts),
      app_fn,
    }
  }
}

#[salvo::async_trait]
impl<F, IV> Handler for LeptosSsrHandler<F, IV>
where
  F: Fn() -> IV + Clone + Send + Sync + 'static,
  IV: IntoView + 'static,
{
  async fn handle(&self, req: &mut Request, _depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let url = build_request_url(req);
    let theme_value = parse_theme_cookie(req.headers());
    let request_path = url.path.clone();
    let resp_opts = LeptosResponseOptions::default();
    let initial_theme = InitialTheme(theme_value.clone());

    let opts = self.opts.clone();
    let app_fn = self.app_fn.clone();

    let owner = Owner::new();
    let resp_opts_for_render = resp_opts.clone();

    let (meta_ctx, meta_out) = ServerMetaContext::new();
    let meta_ctx_for_owner = meta_ctx.clone();

    let body_html: String = owner.with(move || {
      provide_context(url);
      provide_context(initial_theme);
      provide_context(resp_opts_for_render);
      provide_context(opts.seo_defaults.clone());
      provide_context(meta_ctx_for_owner);
      let view = app_fn();
      view.into_view().to_html()
    });

    if let Some(target) = resp_opts.redirect() {
      res.status_code(StatusCode::SEE_OTHER);
      res
        .headers_mut()
        .insert(salvo::http::header::LOCATION, target.parse().unwrap_or_else(|_| "/".parse().unwrap()));
      return;
    }

    let theme_class = theme_value.as_deref();
    let prefix = build_html_prefix(&PrefixContext {
      opts: &self.opts,
      initial_theme: theme_class,
      request_path: &request_path,
    });
    let suffix = build_html_suffix(&self.opts);

    let single_chunk = format!("{prefix}{body_html}{suffix}");
    let stream = futures::stream::iter(std::iter::once(single_chunk));
    let injected = meta_out.inject_meta_context(stream).await;
    let mut final_html = String::new();
    let mut injected = Box::pin(injected);
    while let Some(chunk) = injected.next().await {
      final_html.push_str(&chunk);
    }

    let status = resp_opts.status().unwrap_or(200);
    res.status_code(StatusCode::from_u16(status).unwrap_or(StatusCode::OK));
    res
      .headers_mut()
      .insert(salvo::http::header::CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());
    res
      .headers_mut()
      .insert(salvo::http::header::VARY, "Cookie, Accept-Encoding".parse().unwrap());
    for (name, value) in resp_opts.take_headers() {
      if let (Ok(name), Ok(value)) = (
        salvo::http::header::HeaderName::from_bytes(name.as_bytes()),
        value.parse::<salvo::http::header::HeaderValue>(),
      ) {
        res.headers_mut().insert(name, value);
      }
    }
    let _ = res.write_body(final_html);
  }
}

/// Build a Salvo router that serves the Leptos application via SSR.
///
/// The returned router also serves static assets from `opts.site_root`. The
/// SSR handler matches all unhandled GET paths.
///
/// Initialises the global Leptos task executor (tokio variant) on first call;
/// subsequent calls are no-ops. Must be invoked from within a tokio runtime.
pub fn leptos_router<F, IV>(opts: LeptosOptions, app_fn: F) -> Router
where
  F: Fn() -> IV + Clone + Send + Sync + 'static,
  IV: IntoView + 'static,
{
  let _ = any_spawner::Executor::init_tokio();
  let assets = super::assets::assets_only_router(&opts.site_root, &opts.site_pkg_dir);
  let handler = LeptosSsrHandler::new(opts, app_fn);
  Router::new().push(assets).push(Router::with_path("{**any_path}").get(handler))
}

fn build_request_url(req: &Request) -> RequestUrlCtx {
  let uri = req.uri();
  let scheme = uri
    .scheme_str()
    .map(|s| s.to_string())
    .or_else(|| {
      req
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    })
    .unwrap_or_else(|| "http".to_string());
  let host = uri
    .host()
    .map(|h| {
      if let Some(port) = uri.port_u16() {
        format!("{h}:{port}")
      } else {
        h.to_string()
      }
    })
    .or_else(|| {
      req
        .headers()
        .get(salvo::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    })
    .unwrap_or_default();
  RequestUrlCtx {
    scheme,
    host,
    path: uri.path().to_string(),
    query: uri.query().map(|q| q.to_string()),
  }
}
