//! Central SSR handler for Salvo.
//!
//! Renders the user's Leptos `App` component using `leptos_integration_utils`'s
//! `build_response`. Supports `<Suspense>` streaming via the in-order or
//! out-of-order pipelines and emits the hydration `<script>` and resource
//! data when `LeptosOptions::include_hydration_script` is `true`.

use std::sync::Arc;

use bytes::Bytes;
use futures::StreamExt;
use futures::stream::{self, Stream};
use leptos::IntoView;
use leptos::prelude::*;
use leptos_integration_utils::{BoxedFnOnce, PinnedFuture, PinnedStream, build_response};
use leptos_meta::ServerMetaContext;
use salvo::http::StatusCode;
use salvo::prelude::*;
use tachys::view::RenderHtml;

use impulse_ui_kit::ssr::{InitialTheme, LeptosResponseOptions, RequestUrlCtx};

use crate::static_server::NoRedirectStaticRouter;

use super::options::LeptosOptions;
use super::prefix::{PrefixContext, build_html_prefix, build_html_suffix};
use super::theme::parse_theme_cookie;

/// Streaming mode used by [`LeptosSsrHandler`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SsrStreamMode {
  /// Stream chunks in the order they appear in the document. `<Suspense>`
  /// boundaries block until their resources resolve, then continue. This is
  /// the default — simpler, predictable, no JavaScript-driven re-ordering on
  /// the client.
  #[default]
  InOrder,
  /// Out-of-order streaming. Suspense placeholders are flushed immediately
  /// and replaced via injected `<template>`/`<script>` chunks as resources
  /// resolve. Faster first-meaningful-paint but requires the hydration
  /// runtime to swap fragments.
  OutOfOrder,
}

/// Salvo handler that renders a Leptos application to streaming HTML.
///
/// When [`LeptosSsrHandler::with_assets`] is used, the handler first delegates
/// to a [`NoRedirectStaticRouter`] rooted at `opts.site_root`. Requests that
/// resolve to a real file are served as static assets (with caching and
/// `tracing` logs); everything else falls through to the SSR renderer.
///
/// Construct via [`leptos_router`] rather than instantiating directly.
pub struct LeptosSsrHandler<F, IV>
where
  F: Fn() -> IV + Clone + Send + Sync + 'static,
  IV: IntoView + 'static,
{
  opts: Arc<LeptosOptions>,
  app_fn: F,
  mode: SsrStreamMode,
  assets: Option<Arc<NoRedirectStaticRouter>>,
}

impl<F, IV> LeptosSsrHandler<F, IV>
where
  F: Fn() -> IV + Clone + Send + Sync + 'static,
  IV: IntoView + 'static,
{
  /// Create a new handler with the given options and root component factory.
  pub fn new(opts: LeptosOptions, app_fn: F) -> Self {
    let mode = opts.stream_mode;
    Self {
      opts: Arc::new(opts),
      app_fn,
      mode,
      assets: None,
    }
  }

  /// Attach a static-asset handler that runs ahead of SSR rendering.
  pub fn with_assets(mut self, assets: NoRedirectStaticRouter) -> Self {
    self.assets = Some(Arc::new(assets));
    self
  }
}

#[salvo::async_trait]
impl<F, IV> Handler for LeptosSsrHandler<F, IV>
where
  F: Fn() -> IV + Clone + Send + Sync + 'static,
  IV: IntoView + 'static,
{
  async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    if let Some(assets) = &self.assets {
      assets.handle(req, depot, res, ctrl).await;
      if res.status_code.is_some() || !res.body.is_none() {
        return;
      }
    }
    if is_index_alias(req.uri().path()) {
      res.status_code(StatusCode::MOVED_PERMANENTLY);
      res.headers_mut().insert(
        salvo::http::header::LOCATION,
        salvo::http::header::HeaderValue::from_static("/"),
      );
      return;
    }
    let url = build_request_url(req);
    let theme_value = parse_theme_cookie(req.headers());
    let request_path = url.path.clone();
    let resp_opts = LeptosResponseOptions::default();

    let app_fn = self.app_fn.clone();
    let opts = self.opts.clone();
    let mode = self.mode;

    let (meta_ctx, meta_out) = ServerMetaContext::new();

    let resp_opts_for_render = resp_opts.clone();
    let meta_ctx_for_render = meta_ctx.clone();
    let url_for_render = url.clone();
    let theme_for_render = theme_value.clone();
    let opts_for_render = opts.clone();

    let additional_context = move || {
      provide_context(url_for_render);
      provide_context(InitialTheme(theme_for_render));
      provide_context(resp_opts_for_render);
      provide_context(opts_for_render.seo_defaults.clone());
      provide_context(meta_ctx_for_render);
    };

    let stream_builder: fn(IV, BoxedFnOnce<PinnedStream<String>>, bool) -> PinnedFuture<PinnedStream<String>> =
      match mode {
        SsrStreamMode::InOrder => stream_in_order::<IV>,
        SsrStreamMode::OutOfOrder => stream_out_of_order::<IV>,
      };

    let (owner, app_stream) = build_response(app_fn, additional_context, stream_builder, false);
    let app_stream = app_stream.await;

    if let Some(target) = resp_opts.redirect() {
      res.status_code(StatusCode::SEE_OTHER);
      res.headers_mut().insert(
        salvo::http::header::LOCATION,
        target.parse().unwrap_or_else(|_| "/".parse().unwrap()),
      );
      return;
    }

    let prefix = build_html_prefix(&PrefixContext {
      opts: &self.opts,
      initial_theme: theme_value.as_deref(),
      request_path: &request_path,
    });
    let suffix = build_html_suffix(&self.opts);

    let body_chunks = app_stream.ready_chunks(32).map(|n| n.join(""));
    let prefixed = stream::once(async move { prefix }).chain(body_chunks);
    let with_suffix = prefixed.chain(stream::once(async move { suffix }));

    let injected = meta_out.inject_meta_context(Box::pin(with_suffix)).await;

    let cleanup = stream::once(async move {
      owner.unset_with_forced_cleanup();
      String::new()
    });
    let final_stream = injected
      .chain(cleanup)
      .map(|chunk| Ok::<_, std::io::Error>(Bytes::from(chunk)));

    let status = resp_opts.status().unwrap_or(200);
    res.status_code(StatusCode::from_u16(status).unwrap_or(StatusCode::OK));
    res.headers_mut().insert(
      salvo::http::header::CONTENT_TYPE,
      "text/html; charset=utf-8".parse().unwrap(),
    );
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
    res.body(salvo::http::ResBody::stream(final_stream));
  }
}

fn stream_in_order<IV>(
  app: IV,
  chunks: BoxedFnOnce<PinnedStream<String>>,
  _supports_ooo: bool,
) -> PinnedFuture<PinnedStream<String>>
where
  IV: IntoView + 'static,
{
  Box::pin(async move {
    let app_stream = app.into_view().to_html_stream_in_order();
    let combined: PinnedStream<String> = Box::pin(app_stream.chain(chunks()));
    combined
  })
}

fn stream_out_of_order<IV>(
  app: IV,
  chunks: BoxedFnOnce<PinnedStream<String>>,
  _supports_ooo: bool,
) -> PinnedFuture<PinnedStream<String>>
where
  IV: IntoView + 'static,
{
  Box::pin(async move {
    let app_stream = app.into_view().to_html_stream_out_of_order();
    let combined: PinnedStream<String> = Box::pin(app_stream.chain(chunks()));
    combined
  })
}

/// Build a Salvo router that serves the Leptos application via SSR.
///
/// Files under `opts.site_root` (the `dist/` directory) are served at the URL
/// that mirrors their on-disk path; any path that does not match a real file
/// falls through to the SSR renderer.
///
/// Initialises the global Leptos task executor (tokio variant) on first call;
/// subsequent calls are no-ops. Must be invoked from within a tokio runtime.
pub fn leptos_router<F, IV>(opts: LeptosOptions, app_fn: F) -> Router
where
  F: Fn() -> IV + Clone + Send + Sync + 'static,
  IV: IntoView + 'static,
{
  let _ = any_spawner::Executor::init_tokio();
  let assets = super::assets::build_assets_handler(&opts.site_root);
  let mut handler = LeptosSsrHandler::new(opts, app_fn);
  if let Some(assets) = assets {
    handler = handler.with_assets(assets);
  }
  Router::with_path("{**rest_path}").get(handler)
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

fn _assert_stream_send<S: Stream + Send>(_: &S) {}

/// Whether `path` is an alias for the site root that should be 301-redirected
/// to `/`. Matches `/index.html`, `/index.htm` and `/index.php` so SEO
/// crawlers don't index the canonical landing page under two URLs.
///
/// The asset router runs ahead of this check: a real `dist/index.html`
/// (rare in SSR setups) is still served as a file.
fn is_index_alias(path: &str) -> bool {
  matches!(path, "/index.html" | "/index.htm" | "/index.php")
}

#[cfg(test)]
mod tests {
  use super::is_index_alias;

  #[test]
  fn matches_known_index_aliases() {
    assert!(is_index_alias("/index.html"));
    assert!(is_index_alias("/index.htm"));
    assert!(is_index_alias("/index.php"));
  }

  #[test]
  fn does_not_match_root_or_nested() {
    assert!(!is_index_alias("/"));
    assert!(!is_index_alias("/blog/index.html"));
    assert!(!is_index_alias("/index"));
    assert!(!is_index_alias("/index.html/extra"));
    assert!(!is_index_alias("/INDEX.HTML"));
  }
}
