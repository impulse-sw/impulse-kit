//! Salvo adapter for `#[server]` functions.
//!
//! Converts incoming Salvo requests into `axum::extract::Request`, dispatches
//! through `server_fn::axum::handle_server_fn`, and pipes the resulting body
//! back as a Salvo response. Streaming server functions are supported because
//! `axum::body::Body` is preserved end-to-end.

use bytes::Bytes;
use futures::TryStreamExt;
use salvo::http::{ResBody, StatusCode};
use salvo::prelude::*;

/// Build a Salvo router that dispatches to all server functions registered
/// (either via `#[server]`'s `inventory::submit!` or
/// [`server_fn::axum::register_explicit`]).
///
/// All paths under `prefix` are routed; the function's own `PATH` is matched
/// against the registry.
pub fn server_fn_router(prefix: impl Into<String>) -> Router {
  let prefix = prefix.into();
  let trimmed = prefix.trim_matches('/');
  let pattern = if trimmed.is_empty() {
    "{**fn_path}".to_string()
  } else {
    format!("/{trimmed}/{{**fn_path}}")
  };

  Router::with_path(&pattern)
    .get(ServerFnSalvoHandler)
    .post(ServerFnSalvoHandler)
    .put(ServerFnSalvoHandler)
    .patch(ServerFnSalvoHandler)
    .delete(ServerFnSalvoHandler)
}

/// Salvo handler that wraps `server_fn::axum::handle_server_fn`.
#[derive(Clone)]
pub struct ServerFnSalvoHandler;

#[salvo::async_trait]
impl Handler for ServerFnSalvoHandler {
  async fn handle(&self, req: &mut Request, _depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let axum_req = match build_axum_request(req).await {
      Ok(r) => r,
      Err(err) => {
        res.status_code(StatusCode::BAD_REQUEST);
        let _ = res.write_body(format!("server_fn adapter error: {err}"));
        return;
      }
    };

    let axum_resp = server_fn::axum::handle_server_fn(axum_req).await;
    write_salvo_response(axum_resp, res);
  }
}

async fn build_axum_request(req: &mut Request) -> Result<axum::extract::Request, String> {
  let method = req.method().clone();
  let uri = req.uri().clone();
  let headers = req.headers().clone();
  let version = req.version();

  // Cap the buffered body at the global secure max size (same default the rest
  // of the request-parsing code uses) rather than reading an unbounded body
  // into memory, which would let a single large request exhaust memory.
  let body_bytes = req
    .payload_with_max_size(salvo::http::request::global_secure_max_size())
    .await
    .map_err(|e| format!("read body: {e}"))?
    .clone();
  let body = axum::body::Body::from(body_bytes);

  let mut builder = http::Request::builder().method(method).uri(uri).version(version);
  if let Some(headers_mut) = builder.headers_mut() {
    *headers_mut = headers;
  }
  builder.body(body).map_err(|e| format!("build request: {e}"))
}

fn write_salvo_response(axum_resp: axum::response::Response, res: &mut Response) {
  let (parts, body) = axum_resp.into_parts();
  res.status_code(parts.status);
  for (name, value) in parts.headers.iter() {
    res.headers_mut().insert(name.clone(), value.clone());
  }
  let stream = body
    .into_data_stream()
    .map_ok(Bytes::from)
    .map_err(std::io::Error::other);
  res.body(ResBody::stream(stream));
}
