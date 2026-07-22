//! Mount a transport-agnostic [`impulse_endpoint::Router`] into salvo.
//!
//! Request-handling logic is written once as `impulse-endpoint` handlers over an
//! app state `S`; [`endpoint_router`] wraps that router in a single catch-all
//! salvo handler that matches the incoming path against it. The same
//! `impulse_endpoint::Router` can be handed to `impulse-tauri-engine` to serve
//! the identical routes offline inside a Tauri app — so the logic lives in one
//! place and runs on either host.
//!
//! ```rust,ignore
//! // app state (e.g. a DatabaseConnection) is injected into the depot elsewhere.
//! let routes = impulse_endpoint::Router::<Db>::new()
//!   .route(Method::Get, "/api/v1/items/{id}", GetItem)
//!   .route(Method::Post, "/api/v1/items", CreateItem);
//! let router = Router::with_path("api/v1/{**rest}")
//!   .push(endpoint_router(routes, MyIdentityResolver));
//! ```

use std::sync::Arc;

use impulse_endpoint::{EndpointResponse, Method as EMethod, Router as EndpointRouter};

use crate::salvo::http::{HeaderName, HeaderValue, Method as HttpMethod, StatusCode};
use crate::salvo::{self, Depot, FlowCtrl, Handler, Request, Response, Router};

/// Resolves the request identity (from an auth cookie / middleware / header) for
/// the endpoint context. Return `None` for anonymous requests. Implemented by the
/// host app; a handler then calls [`EndpointCtx::require_identity`] as needed.
///
/// [`EndpointCtx::require_identity`]: impulse_endpoint::EndpointCtx::require_identity
#[salvo::async_trait]
pub trait IdentityResolver: Send + Sync + 'static {
  /// Resolve the authenticated identity, or `None` for anonymous.
  async fn identity(&self, req: &mut Request, depot: &mut Depot) -> Option<String>;
}

/// An [`IdentityResolver`] that treats every request as anonymous. Useful for a
/// router of purely public endpoints.
pub struct Anonymous;

#[salvo::async_trait]
impl IdentityResolver for Anonymous {
  async fn identity(&self, _req: &mut Request, _depot: &mut Depot) -> Option<String> {
    None
  }
}

/// Builds a salvo catch-all handler that dispatches every request under its mount
/// to `routes`, pulling the app state `S` from the depot (`depot.obtain::<S>()`,
/// so inject it with `affix_state`) and the identity from `resolver`.
pub fn endpoint_router<S, I>(routes: EndpointRouter<S>, resolver: I) -> Router
where
  S: Send + Sync + 'static,
  I: IdentityResolver,
{
  let handler = EndpointHandler {
    routes: Arc::new(routes),
    resolver: Arc::new(resolver),
  };
  Router::with_path("{**endpoint_rest}").goal(handler)
}

struct EndpointHandler<S, I> {
  routes: Arc<EndpointRouter<S>>,
  resolver: Arc<I>,
}

fn to_endpoint_method(m: &HttpMethod) -> Option<EMethod> {
  Some(match *m {
    HttpMethod::GET => EMethod::Get,
    HttpMethod::POST => EMethod::Post,
    HttpMethod::PUT => EMethod::Put,
    HttpMethod::PATCH => EMethod::Patch,
    HttpMethod::DELETE => EMethod::Delete,
    HttpMethod::HEAD => EMethod::Head,
    _ => return None,
  })
}

#[salvo::async_trait]
impl<S, I> Handler for EndpointHandler<S, I>
where
  S: Send + Sync + 'static,
  I: IdentityResolver,
{
  async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let Some(method) = to_endpoint_method(req.method()) else {
      res.status_code(StatusCode::METHOD_NOT_ALLOWED);
      return;
    };
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let headers: Vec<(String, String)> = req
      .headers()
      .iter()
      .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or_default().to_string()))
      .collect();
    let body = req
      .payload_with_max_size(usize::MAX)
      .await
      .map(|b| b.to_vec())
      .unwrap_or_default();

    // Resolve identity (may read the depot/req) before borrowing the state.
    let identity = self.resolver.identity(req, depot).await;

    let Ok(state) = depot.get_typed::<S>() else {
      write_response(&EndpointResponse::from_error(&missing_state_error()), res);
      return;
    };

    match self
      .routes
      .dispatch(state, identity.as_deref(), method, &path, &query, &headers, &body)
      .await
    {
      Some(Ok(response)) => write_response(&response, res),
      Some(Err(err)) => write_response(&EndpointResponse::from_error(&err), res),
      None => {
        res.status_code(StatusCode::NOT_FOUND);
      }
    }
  }
}

fn missing_state_error() -> impulse_utils::prelude::ServerError {
  impulse_utils::prelude::ServerError::from_public("endpoint adapter: app state not found in depot")
}

fn write_response(response: &EndpointResponse, res: &mut Response) {
  res.status_code(StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
  for (name, value) in &response.headers {
    if let (Ok(name), Ok(value)) = (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
      res.headers_mut().insert(name, value);
    }
  }
  let _ = res.write_body(response.body.clone());
}
