//! Central SSR handler for Salvo.
//!
//! Renders the user's Leptos `App` component using `leptos_integration_utils`'s
//! `build_response`. Supports `<Suspense>` streaming via the in-order or
//! out-of-order pipelines and emits the hydration `<script>` and resource
//! data when `LeptosOptions::include_hydration_script` is `true`.

use std::sync::{Arc, OnceLock};

use bytes::Bytes;
use futures::StreamExt;
use futures::stream::{self, Stream};
use leptos::IntoView;
use leptos::prelude::*;
use leptos::reactive::owner::Sandboxed;
use leptos_integration_utils::{BoxedFnOnce, PinnedFuture, PinnedStream, build_response};
use leptos_meta::ServerMetaContext;
use salvo::http::StatusCode;
use salvo::prelude::*;
use tachys::view::RenderHtml;
use tokio::sync::{mpsc, oneshot};
use tokio_util::task::LocalPoolHandle;

use impulse_client_kit::ssr::{InitialTheme, LeptosResponseOptions, RequestUrlCtx};

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

    let prefix = build_html_prefix(&PrefixContext {
      opts: &self.opts,
      initial_theme: theme_value.as_deref(),
      request_path: &request_path,
    });
    let suffix = build_html_suffix(&self.opts);

    // The whole reactive lifecycle of a request — building the app, resolving its
    // `<Suspense>` resources, and finally tearing the owner down — must run on a
    // single OS thread. Leptos keeps some `!Send` values (e.g. `<Suspense>`
    // children and type-erased attributes) in the request's reactive arena behind
    // `send_wrapper::SendWrapper`, whose `Drop` aborts the process if it runs on a
    // different thread than the one that created it. On a multi-threaded Tokio
    // runtime an ordinary task is free to resume on a different worker after every
    // `.await` (and Salvo drives a streamed `ResBody` on yet another task), so the
    // render thread and the cleanup thread routinely differ. `spawn_pinned` keeps
    // the task glued to one worker for its entire life, which is the only thing
    // that makes the `SendWrapper` drop sound here. Rendered HTML is forwarded to
    // Salvo over a channel so streaming (including `<Suspense>`) is preserved.
    let (head_tx, head_rx) = oneshot::channel::<ResponseHead>();
    let (body_tx, body_rx) = mpsc::channel::<Bytes>(32);

    leptos_render_pool().spawn_pinned(move || async move {
      let (owner, app_stream) = build_response(app_fn, additional_context, stream_builder, false);
      let app_stream = app_stream.await;

      // The synchronous render above has now populated `ResponseOptions`; snapshot
      // status/redirect/headers and hand them back so the handler can build the
      // response head before any body bytes flow.
      let head = ResponseHead {
        redirect: resp_opts.redirect(),
        status: resp_opts.status(),
        headers: resp_opts.take_headers(),
      };
      let is_redirect = head.redirect.is_some();
      let _ = head_tx.send(head);

      if !is_redirect {
        let body_chunks = app_stream.ready_chunks(32).map(|n| n.join(""));
        let prefixed = stream::once(async move { prefix }).chain(body_chunks);
        let with_suffix = prefixed.chain(stream::once(async move { suffix }));
        let injected = meta_out.inject_meta_context(Box::pin(with_suffix)).await;

        // Re-bind this request's sandboxed arena on every poll: the pinned worker
        // may interleave polls of several requests' streams, and reads of stored
        // values during streaming must hit the right arena.
        let mut injected = Box::pin(owner.with(|| Sandboxed::new(injected)));
        while let Some(chunk) = injected.next().await {
          if body_tx.send(Bytes::from(chunk)).await.is_err() {
            break; // client (or a redirecting handler) dropped the receiver
          }
        }
        drop(injected);
      }

      owner.unset_with_forced_cleanup();
    });

    let Ok(head) = head_rx.await else {
      // The render task ended before producing a response head (e.g. it panicked).
      res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
      return;
    };

    if let Some(target) = head.redirect {
      res.status_code(StatusCode::SEE_OTHER);
      res.headers_mut().insert(
        salvo::http::header::LOCATION,
        target.parse().unwrap_or_else(|_| "/".parse().unwrap()),
      );
      return;
    }

    res.status_code(StatusCode::from_u16(head.status.unwrap_or(200)).unwrap_or(StatusCode::OK));
    res.headers_mut().insert(
      salvo::http::header::CONTENT_TYPE,
      "text/html; charset=utf-8".parse().unwrap(),
    );
    res
      .headers_mut()
      .insert(salvo::http::header::VARY, "Cookie, Accept-Encoding".parse().unwrap());
    for (name, value) in head.headers {
      if let (Ok(name), Ok(value)) = (
        salvo::http::header::HeaderName::from_bytes(name.as_bytes()),
        value.parse::<salvo::http::header::HeaderValue>(),
      ) {
        res.headers_mut().insert(name, value);
      }
    }

    let body_stream = stream::unfold(body_rx, |mut body_rx| async move {
      body_rx
        .recv()
        .await
        .map(|chunk| (Ok::<_, std::io::Error>(chunk), body_rx))
    });
    res.body(salvo::http::ResBody::stream(body_stream));
  }
}

/// Response status, redirect target and headers captured from `ResponseOptions`
/// after the synchronous render, passed from the pinned render task back to the
/// Salvo handler so it can build the response head before streaming the body.
struct ResponseHead {
  redirect: Option<String>,
  status: Option<u16>,
  headers: Vec<(String, String)>,
}

/// Shared pool of single-threaded workers used to render Leptos responses.
///
/// Each request's render runs to completion — including dropping the reactive
/// owner — pinned to one worker, so the `!Send` values Leptos stores behind
/// `SendWrapper` are always dropped on the thread that created them. Sized to the
/// available parallelism and initialised on first use.
fn leptos_render_pool() -> &'static LocalPoolHandle {
  static POOL: OnceLock<LocalPoolHandle> = OnceLock::new();
  POOL.get_or_init(|| {
    let threads = std::thread::available_parallelism()
      .map(|n| n.get())
      .unwrap_or(1)
      .max(1);
    LocalPoolHandle::new(threads)
  })
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
