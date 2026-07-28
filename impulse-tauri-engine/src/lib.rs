//! Reusable offline-first engine for Tauri apps.
//!
//! A Tauri webview can't reach the network from wasm, so the UI forwards every
//! request over IPC (`invoke("ik_http_request")`) to the native side, where this
//! engine handles it:
//!
//! * **online** — forward to the real server (via a [`Remote`]) and let the app
//!   cache successful reads locally ([`LocalBackend::cache_read`]);
//! * **offline** — serve the request from a local store
//!   ([`LocalBackend::serve_local`]) and, for writes, enqueue it for replay;
//! * **on reconnect** — [`Engine::sync`] replays the queued writes oldest-first,
//!   reconciling any locally-minted provisional ids with the server's real ids.
//!
//! Everything app-specific — how to serve/cache/reconcile locally — lives behind
//! the [`LocalBackend`] trait, so the transport, the online/offline switch and
//! the persistent write [`Queue`] are written once here. The wire types come from
//! `impulse-endpoint`, so this crate does not depend on the leptos UI kit.
//!
//! ## Offline is a state, not an error
//!
//! Every response the engine produces without reaching the server carries
//! [`OFFLINE_HEADER`](impulse_endpoint::OFFLINE_HEADER) — see
//! [`HttpResponse::is_offline`]. That distinction is what lets a frontend keep
//! working without a network: a `401` that came off the wire means the session
//! really was rejected and the user must sign in again, while an engine-produced
//! `503` means nobody was asked and the app should fall back to whatever it knows
//! locally (its stored credentials, its cached data) instead of bouncing the user
//! to a login screen it can't complete anyway.
//!
//! The engine deliberately never validates credentials itself. It only needs the
//! app's [`LocalBackend`] to remember *whose* data it is serving — captured from
//! a successful online response via [`LocalBackend::observe_write`] /
//! [`LocalBackend::cache_read`] — so the offline work can be attributed and,
//! later, synced.
//!
//! Native (non-wasm) only.

#![deny(warnings, clippy::todo, clippy::unimplemented, missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use impulse_endpoint::{HttpRequest, HttpResponse, OFFLINE_HEADER};
use impulse_utils::prelude::{CResult, ServerError};

mod queue;
pub use queue::{Entry, Queue};

/// The remote transport the engine forwards to when online. Injected so the
/// engine can be exercised against a fake server in tests.
///
/// Used only through generics (never as a `dyn` object), so `async fn` here is
/// fine.
#[allow(async_fn_in_trait)]
pub trait Remote {
  /// Sends a request to the remote server, or errors if it can't be reached.
  async fn send(&self, req: HttpRequest) -> CResult<HttpResponse>;
}

/// The production [`Remote`]: runs the request natively via reqwest. This is the
/// native side of a Tauri app forwarding the UI's requests to the real server.
/// Available with the `executor` feature.
#[cfg(feature = "executor")]
pub struct ExecutorRemote;

#[cfg(feature = "executor")]
impl Remote for ExecutorRemote {
  async fn send(&self, req: HttpRequest) -> CResult<HttpResponse> {
    executor::execute(req).await
  }
}

/// The native reqwest executor behind [`ExecutorRemote`]. Also usable directly
/// (e.g. a connectivity probe) without constructing an [`Engine`].
#[cfg(feature = "executor")]
pub mod executor {
  use impulse_endpoint::{HttpRequest, HttpResponse};
  use impulse_utils::prelude::{CResult, ClientError};

  /// Runs `req` with reqwest and collects a buffered [`HttpResponse`].
  pub async fn execute(req: HttpRequest) -> CResult<HttpResponse> {
    let client = reqwest::Client::new();
    let mut rb = client.request(req.method.into(), &req.url);
    for (k, v) in &req.headers {
      rb = rb.header(k, v);
    }
    if let Some(body) = req.body {
      rb = rb.body(body);
    }
    let resp = rb.send().await.map_err(ClientError::from)?;
    let status = resp.status().as_u16();
    let headers = resp
      .headers()
      .iter()
      .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or_default().to_string()))
      .collect();
    let body = resp.bytes().await.map_err(ClientError::from)?.to_vec();
    Ok(HttpResponse { status, headers, body })
  }
}

/// The app-specific local behaviour behind the engine: how to serve, cache and
/// reconcile requests against a local store while offline. An app implements this
/// over its own state (a database handle, the signed-in identity, …).
///
/// Used only through generics, so `async fn` is fine.
#[allow(async_fn_in_trait)]
pub trait LocalBackend {
  /// Serves a request from the local store while offline. Returns the response
  /// plus, for a create, the temporary id it minted (obtained from `provisional`,
  /// which draws from the engine's queue) so the caller can tag the queued write
  /// for later id reconciliation. Return an `Err` (e.g. a `503`) for requests
  /// that simply aren't available offline.
  async fn serve_local(
    &self,
    req: &HttpRequest,
    provisional: &dyn Fn() -> i64,
  ) -> Result<(HttpResponse, Option<i64>), ServerError>;

  /// Caches a successful online read locally so it is available offline later.
  /// Defaults to a no-op.
  async fn cache_read(&self, _req: &HttpRequest, _resp: &HttpResponse) {}

  /// Observes a successful online *write* (anything that isn't a read), which
  /// [`cache_read`](Self::cache_read) deliberately never sees. The hook exists
  /// for responses that carry state the offline side needs but that aren't
  /// `GET`s — the canonical case being an auth endpoint answering `POST` with
  /// the signed-in identity, which the backend must remember to serve requests
  /// as that user while offline. Defaults to a no-op.
  async fn observe_write(&self, _req: &HttpRequest, _resp: &HttpResponse) {}

  /// Whether a request the backend just served locally should be queued for
  /// replay to the server. Defaults to "every write is queued", which is what a
  /// data endpoint wants.
  ///
  /// Override it for endpoints that are `POST`s by protocol but change nothing
  /// on the server — an auth *check*, a probe — so answering them offline
  /// doesn't schedule a pointless (or harmful) replay on reconnect.
  fn should_queue(&self, req: &HttpRequest) -> bool {
    !req.method.is_read()
  }

  /// Last-chance rewrite of a queued request just before it is replayed,
  /// *after* [`rewrite_ids`](Self::rewrite_ids). Defaults to identity.
  ///
  /// A queued write carries the headers it was built with, possibly weeks
  /// earlier — including an `Authorization` header whose access token has long
  /// since rotated. Override this to stamp the current credentials on the
  /// request so the replay is authenticated as of *now*, not as of the moment
  /// the user went offline.
  fn prepare_replay(&self, req: HttpRequest) -> HttpRequest {
    req
  }

  /// The server-assigned id in a create's replay response, used to reconcile it
  /// with the provisional id minted offline. Defaults to `None` (no id
  /// reconciliation).
  fn created_id(&self, _resp: &HttpResponse) -> Option<i64> {
    None
  }

  /// Reconciles a provisional id → real id in the local store after a queued
  /// create replays. Defaults to a no-op.
  async fn reconcile_id(&self, _provisional: i64, _real: i64) {}

  /// Rewrites a queued request's provisional ids using the temp→real map built
  /// during [`Engine::sync`], so a follow-up edit of an offline-created item
  /// targets the real id once the create has replayed. Defaults to identity.
  fn rewrite_ids(&self, req: &HttpRequest, _id_map: &HashMap<i64, i64>) -> HttpRequest {
    req.clone()
  }
}

/// The offline-capable request engine backing a Tauri app.
pub struct Engine<R: Remote, L: LocalBackend> {
  remote: R,
  backend: L,
  remote_base: String,
  online: AtomicBool,
  queue: Queue,
}

#[cfg(feature = "executor")]
impl<L: LocalBackend> Engine<ExecutorRemote, L> {
  /// Builds an engine using the real network transport.
  pub fn with_executor(
    backend: L,
    remote_base: impl Into<String>,
    queue_path: impl Into<PathBuf>,
  ) -> std::io::Result<Self> {
    Self::new(backend, ExecutorRemote, remote_base, queue_path)
  }
}

impl<R: Remote, L: LocalBackend> Engine<R, L> {
  /// Builds an engine forwarding online requests to `remote_base` through
  /// `remote`, serving offline through `backend`, persisting the write queue at
  /// `queue_path`.
  pub fn new(
    backend: L,
    remote: R,
    remote_base: impl Into<String>,
    queue_path: impl Into<PathBuf>,
  ) -> std::io::Result<Self> {
    Ok(Self {
      remote,
      backend,
      remote_base: remote_base.into(),
      online: AtomicBool::new(true),
      queue: Queue::open(queue_path.into())?,
    })
  }

  /// The app's local backend (e.g. to set the signed-in identity on it).
  pub fn backend(&self) -> &L {
    &self.backend
  }

  /// Whether the engine currently believes it can reach the server.
  pub fn is_online(&self) -> bool {
    self.online.load(Ordering::Relaxed)
  }

  /// Updates the online flag. The shell flips this from its connectivity probe;
  /// on a false→true transition it should also call [`sync`](Self::sync).
  pub fn set_online(&self, online: bool) {
    self.online.store(online, Ordering::Relaxed);
  }

  /// Number of writes waiting to be replayed to the server.
  pub fn pending_sync(&self) -> usize {
    self.queue.len()
  }

  /// The IPC entry point: handle one request from the UI. Online, it forwards to
  /// the server and lets the backend observe the response; on a network failure —
  /// or when already offline — it serves from the backend and queues writes.
  ///
  /// Every response produced *without* reaching the server is stamped with
  /// [`OFFLINE_HEADER`](impulse_endpoint::OFFLINE_HEADER), so the caller can tell
  /// "the server rejected this" from "the server was never asked" — see
  /// [`HttpResponse::is_offline`]. A failed remote attempt also flips the engine
  /// offline immediately, rather than letting the next few requests each pay for
  /// their own timeout until the shell's connectivity probe notices.
  pub async fn handle(&self, req: HttpRequest) -> HttpResponse {
    if self.is_online() {
      let mut remote = req.clone();
      remote.url = self.remote_url(&req.url);
      match self.remote.send(remote).await {
        Ok(resp) => {
          if resp.is_success() {
            if req.method.is_read() {
              self.backend.cache_read(&req, &resp).await;
            } else {
              self.backend.observe_write(&req, &resp).await;
            }
          }
          return resp;
        }
        Err(e) => {
          tracing::warn!("remote request failed, serving offline: {e}");
          self.set_online(false);
        }
      }
    }

    let mint = || self.queue.next_provisional_id();
    match self.backend.serve_local(&req, &mint).await {
      Ok((resp, provisional)) => {
        if self.backend.should_queue(&req) {
          self.queue.enqueue(&req, provisional);
        }
        mark_offline(resp)
      }
      Err(err) => mark_offline(error_response(err)),
    }
  }

  /// Replays queued offline writes against the server, oldest first, dropping
  /// each on success. Stops at the first failure, leaving it and the rest queued.
  /// Call on a false→true connectivity transition.
  ///
  /// A queued write is **only ever dropped when the server accepted it**. A
  /// rejection — including a `401` because the session expired while the user was
  /// away — stops the replay with the entry still in the queue, so offline work
  /// survives an expired session and lands after the next sign-in. Call this
  /// again once the user has re-authenticated ([`prepare_replay`] then stamps the
  /// fresh credentials on each entry).
  ///
  /// [`prepare_replay`]: LocalBackend::prepare_replay
  ///
  /// Reconciles ids: a queued create carries the temporary id it minted offline;
  /// when its replay returns the server's real id, later queued requests that
  /// referenced the temporary id are rewritten to the real one, and the local
  /// store is reconciled via [`LocalBackend::reconcile_id`].
  pub async fn sync(&self) -> Result<(), String> {
    let mut id_map: HashMap<i64, i64> = HashMap::new();
    for entry in self.queue.pending() {
      let mut req = self.backend.prepare_replay(self.backend.rewrite_ids(&entry.req, &id_map));
      req.url = self.remote_url(&req.url);
      match self.remote.send(req).await {
        Ok(resp) if resp.is_success() => {
          if let Some(temp) = entry.provisional_id
            && let Some(real) = self.backend.created_id(&resp)
          {
            id_map.insert(temp, real);
            self.backend.reconcile_id(temp, real).await;
          }
          self.queue.ack(entry.id);
        }
        Ok(resp) => return Err(format!("server rejected a queued write: HTTP {}", resp.status())),
        Err(e) => return Err(format!("sync failed, will retry: {e}")),
      }
    }
    Ok(())
  }

  fn remote_url(&self, url: &str) -> String {
    format!("{}{}", self.remote_base.trim_end_matches('/'), path_and_query(url))
  }
}

/// Stamps [`OFFLINE_HEADER`] on a response the engine produced itself, marking
/// it as "the server was never reached" for callers that need to tell that apart
/// from a real server verdict.
fn mark_offline(mut resp: HttpResponse) -> HttpResponse {
  if !resp.is_offline() {
    resp.headers.push((OFFLINE_HEADER.to_string(), "1".to_string()));
  }
  resp
}

/// Renders a [`ServerError`] as an HTTP-shaped [`HttpResponse`] (a JSON
/// `{ "err": … }` body), matching what a server would return.
pub fn error_response(err: ServerError) -> HttpResponse {
  let status = err.status_code.map(|c| c.as_u16()).unwrap_or(500);
  let msg = err.public_msg.clone().unwrap_or_else(|| "Server error".to_string());
  let body = serde_json::to_vec(&serde_json::json!({ "err": msg })).unwrap_or_default();
  HttpResponse {
    status,
    headers: vec![("content-type".into(), "application/json".into())],
    body,
  }
}

/// Extracts the `path` (with `?query`) from a possibly-absolute URL, so the
/// engine can re-base a request onto its `remote_base`.
pub fn path_and_query(url: &str) -> String {
  if let Some(rest) = url.find("://").map(|i| &url[i + 3..]) {
    match rest.find('/') {
      Some(slash) => rest[slash..].to_string(),
      None => "/".to_string(),
    }
  } else {
    url.to_string()
  }
}

#[cfg(test)]
mod tests;
