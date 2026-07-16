//! A tiny transport-agnostic endpoint/router abstraction.
//!
//! An [`Endpoint`] is request-handling logic written **once**, against a neutral
//! [`EndpointCtx`] (method, path params, query, body, an app-supplied identity
//! and app state) returning an [`EndpointResponse`]. The same [`Router`] can then
//! be mounted on either host:
//!
//! * on the server, a salvo adapter wraps each route as a salvo handler;
//! * in a Tauri app, the engine matches an incoming [`crate::HttpRequest`]
//!   against the router and runs the handler locally (offline), or forwards it to
//!   the real server (online).
//!
//! Handlers are object-safe (`Box<dyn Endpoint<S>>`) via a boxed future, so a
//! router can hold heterogeneous handlers over one shared state `S`.

use std::future::Future;
use std::pin::Pin;

use impulse_utils::prelude::{MResult, ServerError};
use serde::Serialize;

use crate::wire::Method;

/// One segment of a [`PathPattern`].
#[derive(Clone, Debug)]
enum Segment {
  /// A literal segment that must match exactly.
  Static(String),
  /// A `{name}` capture bound into [`PathParams`].
  Param(String),
}

/// A path pattern like `/api/v1/documents/{id}`, matched against a concrete path.
/// Segment count must match exactly; `{name}` segments capture into [`PathParams`].
#[derive(Clone, Debug)]
pub struct PathPattern {
  segments: Vec<Segment>,
}

impl PathPattern {
  /// Parses a pattern. `{name}` marks a captured parameter; everything else is a
  /// literal segment. Leading/trailing slashes and empty segments are ignored.
  pub fn new(pattern: &str) -> Self {
    let segments = pattern
      .trim_matches('/')
      .split('/')
      .filter(|s| !s.is_empty())
      .map(|s| match s.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        Some(name) => Segment::Param(name.to_string()),
        None => Segment::Static(s.to_string()),
      })
      .collect();
    Self { segments }
  }

  /// Matches a concrete `path`, returning the captured parameters, or `None` when
  /// the path doesn't fit this pattern.
  pub fn matches(&self, path: &str) -> Option<PathParams> {
    let parts: Vec<&str> = path.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() != self.segments.len() {
      return None;
    }
    let mut params = Vec::new();
    for (seg, part) in self.segments.iter().zip(parts.iter()) {
      match seg {
        Segment::Static(s) if s == part => {}
        Segment::Static(_) => return None,
        Segment::Param(name) => params.push((name.clone(), (*part).to_string())),
      }
    }
    Some(PathParams(params))
  }
}

/// Captured `{name}` path parameters.
#[derive(Clone, Debug, Default)]
pub struct PathParams(Vec<(String, String)>);

impl PathParams {
  /// Builds params directly (used by the salvo adapter, which already extracted
  /// them). Prefer [`PathPattern::matches`] elsewhere.
  pub fn from_pairs(pairs: Vec<(String, String)>) -> Self {
    Self(pairs)
  }

  /// The raw value of a captured parameter.
  pub fn get(&self, name: &str) -> Option<&str> {
    self.0.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
  }

  /// Parses a captured parameter, returning a `400` [`ServerError`] when it is
  /// missing or unparseable.
  pub fn parse<T: std::str::FromStr>(&self, name: &str) -> MResult<T> {
    self
      .get(name)
      .ok_or_else(|| ServerError::from_public(format!("Missing path parameter `{name}`")).with_400())?
      .parse::<T>()
      .map_err(|_| ServerError::from_public(format!("Invalid path parameter `{name}`")).with_400())
  }
}

/// The neutral request context handed to an [`Endpoint`]. Borrows everything, so
/// it is cheap to build on either host.
pub struct EndpointCtx<'a, S> {
  /// App state (e.g. a database handle), supplied by the host.
  pub state: &'a S,
  /// The authenticated identity (e.g. an email), or `None` for anonymous
  /// requests. The host resolves this — auth middleware on the server, the
  /// signed-in user on the engine.
  pub identity: Option<&'a str>,
  /// The request method.
  pub method: Method,
  /// Captured path parameters.
  pub params: &'a PathParams,
  /// The raw query string (without the leading `?`).
  pub query: &'a str,
  /// Request headers.
  pub headers: &'a [(String, String)],
  /// Raw request body.
  pub body: &'a [u8],
}

impl<S> EndpointCtx<'_, S> {
  /// The authenticated identity, or a `401` [`ServerError`] when anonymous.
  pub fn require_identity(&self) -> MResult<&str> {
    self
      .identity
      .ok_or_else(|| ServerError::from_public("Not signed in").with_401())
  }

  /// Decodes the JSON request body, mapping a parse failure to a `400`.
  pub fn json_body<T: serde::de::DeserializeOwned>(&self) -> MResult<T> {
    serde_json::from_slice(self.body).map_err(|e| ServerError::from_public(format!("Malformed body: {e}")).with_400())
  }

  /// The first value of a `key=value` pair in the query string, percent-decoding
  /// left to the caller.
  pub fn query_param(&self, key: &str) -> Option<&str> {
    self.query.split('&').find_map(|pair| {
      let (k, v) = pair.split_once('=')?;
      (k == key).then_some(v)
    })
  }
}

/// A response produced by an [`Endpoint`]. Structurally identical to
/// [`crate::HttpResponse`] so the engine can hand it straight back over IPC.
pub struct EndpointResponse {
  /// HTTP status code.
  pub status: u16,
  /// Response headers.
  pub headers: Vec<(String, String)>,
  /// Raw response body.
  pub body: Vec<u8>,
}

impl EndpointResponse {
  /// A `200 application/json` response encoding `value`.
  pub fn json<T: Serialize>(value: &T) -> MResult<Self> {
    let body = serde_json::to_vec(value).map_err(|e| ServerError::from_private(e).with_500())?;
    Ok(Self {
      status: 200,
      headers: vec![("content-type".into(), "application/json".into())],
      body,
    })
  }

  /// An empty `200` response.
  pub fn empty() -> Self {
    Self {
      status: 200,
      headers: Vec::new(),
      body: Vec::new(),
    }
  }

  /// Overrides the status code (builder-style).
  pub fn with_status(mut self, status: u16) -> Self {
    self.status = status;
    self
  }
}

/// The boxed future an [`Endpoint`] returns. Boxing keeps the trait object-safe
/// so a [`Router`] can hold heterogeneous handlers.
pub type EndpointFuture<'a> = Pin<Box<dyn Future<Output = MResult<EndpointResponse>> + Send + 'a>>;

/// Request-handling logic over app state `S`. Implement it on a unit struct per
/// route; the body is a `Box::pin(async move { … })` over the [`EndpointCtx`].
pub trait Endpoint<S>: Send + Sync {
  /// Handles one request.
  fn call<'a>(&'a self, ctx: EndpointCtx<'a, S>) -> EndpointFuture<'a>;
}

/// A single mounted route: method + path pattern + handler.
pub struct Route<S> {
  /// The method this route answers.
  pub method: Method,
  /// The path pattern this route matches.
  pub pattern: PathPattern,
  /// The handler.
  pub handler: Box<dyn Endpoint<S>>,
}

/// An ordered collection of [`Route`]s over shared app state `S`.
pub struct Router<S> {
  routes: Vec<Route<S>>,
}

impl<S> Default for Router<S> {
  fn default() -> Self {
    Self::new()
  }
}

impl<S> Router<S> {
  /// An empty router.
  pub fn new() -> Self {
    Self { routes: Vec::new() }
  }

  /// Adds a route (builder-style).
  pub fn route(mut self, method: Method, pattern: &str, handler: impl Endpoint<S> + 'static) -> Self {
    self.routes.push(Route {
      method,
      pattern: PathPattern::new(pattern),
      handler: Box::new(handler),
    });
    self
  }

  /// The mounted routes (for a host adapter that registers them, e.g. salvo).
  pub fn routes(&self) -> &[Route<S>] {
    &self.routes
  }

  /// Finds the first route matching `method` + `path`, returning the handler and
  /// captured params.
  pub fn match_route(&self, method: Method, path: &str) -> Option<(&dyn Endpoint<S>, PathParams)> {
    self.routes.iter().find_map(|r| {
      if r.method == method {
        r.pattern.matches(path).map(|params| (r.handler.as_ref(), params))
      } else {
        None
      }
    })
  }

  /// Matches and runs a request against the router. Returns `None` when no route
  /// matched (so the host can 404 or fall through), otherwise the handler result.
  #[allow(clippy::too_many_arguments)]
  pub async fn dispatch(
    &self,
    state: &S,
    identity: Option<&str>,
    method: Method,
    path: &str,
    query: &str,
    headers: &[(String, String)],
    body: &[u8],
  ) -> Option<MResult<EndpointResponse>> {
    let (handler, params) = self.match_route(method, path)?;
    let ctx = EndpointCtx {
      state,
      identity,
      method,
      params: &params,
      query,
      headers,
      body,
    };
    Some(handler.call(ctx).await)
  }
}
