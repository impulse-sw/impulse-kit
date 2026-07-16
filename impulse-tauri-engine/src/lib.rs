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
//! Native (non-wasm) only.

#![deny(warnings, clippy::todo, clippy::unimplemented, missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use impulse_endpoint::{HttpRequest, HttpResponse};
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

/// The production [`Remote`]: the kit's native reqwest executor
/// (`impulse-tauri-client`). Available with the `executor` feature.
#[cfg(feature = "executor")]
pub struct ExecutorRemote;

#[cfg(feature = "executor")]
impl Remote for ExecutorRemote {
  async fn send(&self, req: HttpRequest) -> CResult<HttpResponse> {
    impulse_tauri_client::executor::execute(req).await
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
  /// the server and lets the backend cache reads locally; on a network failure —
  /// or when already offline — it serves from the backend and queues writes.
  pub async fn handle(&self, req: HttpRequest) -> HttpResponse {
    if self.is_online() {
      let mut remote = req.clone();
      remote.url = self.remote_url(&req.url);
      match self.remote.send(remote).await {
        Ok(resp) => {
          if req.method.is_read() && resp.is_success() {
            self.backend.cache_read(&req, &resp).await;
          }
          return resp;
        }
        Err(e) => tracing::warn!("remote request failed, serving offline: {e}"),
      }
    }

    let mint = || self.queue.next_provisional_id();
    match self.backend.serve_local(&req, &mint).await {
      Ok((resp, provisional)) => {
        if !req.method.is_read() {
          self.queue.enqueue(&req, provisional);
        }
        resp
      }
      Err(err) => error_response(err),
    }
  }

  /// Replays queued offline writes against the server, oldest first, dropping
  /// each on success. Stops at the first failure, leaving it and the rest queued.
  /// Call on a false→true connectivity transition.
  ///
  /// Reconciles ids: a queued create carries the temporary id it minted offline;
  /// when its replay returns the server's real id, later queued requests that
  /// referenced the temporary id are rewritten to the real one, and the local
  /// store is reconciled via [`LocalBackend::reconcile_id`].
  pub async fn sync(&self) -> Result<(), String> {
    let mut id_map: HashMap<i64, i64> = HashMap::new();
    for entry in self.queue.pending() {
      let mut req = self.backend.rewrite_ids(&entry.req, &id_map);
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
