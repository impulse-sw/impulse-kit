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
use std::time::Instant;

use impulse_endpoint::{HttpRequest, HttpResponse, OFFLINE_HEADER};
use impulse_utils::prelude::{CResult, ServerError};

mod queue;
pub use queue::{Entry, Queue};

/// The app-lifecycle signal a shell reports a resume through, shared by the
/// socket and the reconnect loop.
#[cfg(feature = "ws")]
pub mod lifecycle;

#[cfg(feature = "ws")]
mod ws;
#[cfg(feature = "ws")]
pub use ws::{Emit, LocalReply, ReconnectPolicy, WsBackend, WsEngine, WsEntry, WsQueue, WsRemote, WsSink, WsStream};

/// The concrete socket a Tauri shell opens, with the keepalive, idle detection
/// and resume handling a mobile OS makes necessary.
#[cfg(feature = "shell")]
pub mod shell;

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
  use std::sync::{Arc, OnceLock};
  use std::time::Duration;

  use impulse_endpoint::{HttpRequest, HttpResponse, OFFLINE_HEADER};
  use impulse_utils::prelude::{CResult, ClientError};

  /// How long to spend reaching a host before treating it as unreachable —
  /// generous for a slow mobile network, short enough that a drop isn't felt as
  /// a hang. Without it an unreachable server holds a request for as long as the
  /// OS allows, and the engine can't fall back to the local copy until it ends.
  const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

  /// How long a request may go without a single byte arriving before it is
  /// abandoned.
  ///
  /// [`CONNECT_TIMEOUT`] does not cover this, and the gap is where a mobile app
  /// hangs. It bounds *opening* a connection, so it says nothing about a request
  /// sent over a pooled one that was opened earlier and has since died — and
  /// after a spell in the background, that describes every connection in the
  /// pool. The peer is gone, no RST comes back, and a request with no read
  /// deadline waits for a reply that will never arrive: forever, silently, with
  /// whatever was waiting on it (a reconnect loop, an upload drain) stopped
  /// behind it.
  ///
  /// It is a read deadline rather than a total one so a slow large transfer — an
  /// offline photo queue draining over a weak link — is not cut off for taking
  /// its time while making progress.
  const READ_TIMEOUT: Duration = Duration::from_secs(4);

  /// How long an idle pooled connection may be kept for reuse.
  ///
  /// Short on purpose. Reusing a connection is only a win while it is alive, and
  /// on a phone the interesting requests are the first ones after a spell in the
  /// background — exactly when everything pooled is stale, the network may have
  /// changed from Wi-Fi to cellular underneath, and a fresh handshake costs far
  /// less than discovering the old socket is dead by waiting on it.
  const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(5);

  /// Picks `ring` as the process-wide rustls provider. Call once, before any TLS.
  ///
  /// [`client`] never needs this — it is handed an explicit provider — but a
  /// socket library that builds its own config (tokio-tungstenite's
  /// `connect_async`, say) asks rustls for the process default, and with both
  /// `ring` and `aws-lc-rs` reachable in the graph rustls refuses to guess and
  /// panics. An app that opens sockets should call this at startup.
  pub fn install_crypto_provider() {
    // `Err` means someone already installed one; either way a provider is set.
    let _ = rustls::crypto::ring::default_provider().install_default();
  }

  /// The shared client: bundled Mozilla WebPKI roots, never the platform
  /// verifier, and a deadline on every wait it can make. Built once, on first
  /// use.
  ///
  /// Everything native an app sends should go through this — the webview's
  /// forwarded requests, a socket ticket, an upload queue draining — because the
  /// timeouts above are the only thing standing between a stale pooled
  /// connection and a request that never returns.
  ///
  /// The platform verifier is not merely a different choice here — on Android it
  /// reaches the system trust store over JNI and panics unless something has
  /// initialised it with the app `Context`. Under a release profile's
  /// `panic = "abort"` that aborts the process on the very first request, which
  /// is to say on startup. The bundled roots cover the public CAs, so
  /// verification behaves identically on desktop and mobile with no JNI setup.
  pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
      let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
      };
      // The provider is pinned explicitly rather than taken from the process
      // default, so this client works whether or not anything called
      // `install_crypto_provider`.
      let tls = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("rustls default protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
      reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        // Lets the OS notice a dead peer on a connection that is otherwise
        // sitting there, instead of leaving it to the request that reuses it.
        .tcp_keepalive(POOL_IDLE_TIMEOUT)
        .build()
        .expect("build reqwest client")
    })
  }

  /// Runs a request the webview handed over, re-based onto `server_origin`, and
  /// answers with a response rather than an error when the server can't be
  /// reached.
  ///
  /// This is the body of an app's `ik_http_request` command. Two things it does
  /// that [`execute`] does not:
  ///
  /// The URL is re-based. A webview builds absolute URLs from `window.location`,
  /// which inside a Tauri shell is the webview origin — `tauri://localhost/api/…`,
  /// a scheme no HTTP client can dial. Only path and query carry meaning.
  ///
  /// A request that never reached the server answers `503` + [`OFFLINE_HEADER`]
  /// instead of failing. That distinction is the point of an offline-first app: a
  /// real `401` off the wire means the session was rejected and the user must
  /// sign in again, while "nobody was asked" must leave the app on what it knows
  /// locally, or it bounces its owner to a login screen that cannot be completed
  /// without the very network that is missing.
  ///
  /// Auth is the caller's: whatever headers the request arrived with are
  /// forwarded untouched, and nothing is added.
  pub async fn serve_webview(server_origin: &str, req: HttpRequest) -> HttpResponse {
    let url = format!(
      "{}{}",
      server_origin.trim_end_matches('/'),
      super::path_and_query(&req.url)
    );
    let mut rb = client().request(req.method.into(), &url);
    for (name, value) in &req.headers {
      rb = rb.header(name, value);
    }
    if let Some(body) = req.body {
      rb = rb.body(body);
    }

    let resp = match rb.send().await {
      Ok(resp) => resp,
      Err(e) => return offline(format!("request failed: {e}")),
    };
    let status = resp.status().as_u16();
    let headers = resp
      .headers()
      .iter()
      .filter_map(|(name, value)| value.to_str().ok().map(|v| (name.as_str().to_owned(), v.to_owned())))
      .collect();
    match resp.bytes().await {
      Ok(body) => HttpResponse {
        status,
        headers,
        body: body.to_vec(),
      },
      Err(e) => offline(format!("reading the response failed: {e}")),
    }
  }

  /// The "nobody was asked" response, which
  /// [`HttpResponse::is_offline`](impulse_endpoint::HttpResponse::is_offline)
  /// reports and an auth gate reads as *no connection* rather than *rejected*.
  fn offline(reason: String) -> HttpResponse {
    tracing::debug!("serving offline: {reason}");
    HttpResponse {
      status: 503,
      headers: vec![(OFFLINE_HEADER.to_string(), "1".to_string())],
      body: reason.into_bytes(),
    }
  }

  /// Percent-encodes a single query-parameter value: everything outside the
  /// RFC 3986 unreserved set is escaped.
  pub fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
      match byte {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
        _ => out.push_str(&format!("%{byte:02X}")),
      }
    }
    out
  }

  /// Runs `req` with the shared client and collects a buffered [`HttpResponse`].
  pub async fn execute(req: HttpRequest) -> CResult<HttpResponse> {
    let mut rb = client().request(req.method.into(), &req.url);
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

  /// Last-chance rewrite of a request the **engine itself** is about to send —
  /// a queued write being replayed (after [`rewrite_ids`](Self::rewrite_ids)) or
  /// a [`prefetch`](Engine::prefetch). Defaults to identity.
  ///
  /// Requests that didn't come straight from the UI have no current credentials
  /// on them: a queued write carries the headers it was built with, possibly
  /// weeks ago and with a long-rotated access token, and a prefetch was never
  /// built by the UI at all. Override this to stamp the current credentials.
  fn prepare_outgoing(&self, req: HttpRequest) -> HttpRequest {
    req
  }

  /// Requests worth running while online purely to fill the local store, so the
  /// data is there when the network isn't. Defaults to none.
  ///
  /// Caching only what the user happened to open leaves an app half-usable
  /// offline — you find out which documents you *didn't* read at exactly the
  /// wrong moment. Returning "everything I know about but haven't fully cached"
  /// here lets [`Engine::prefetch`] close that gap in the background. Responses
  /// go through [`cache_read`](Self::cache_read) like any other read.
  async fn prefetch_requests(&self) -> Vec<HttpRequest> {
    Vec::new()
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

  /// Empties the local store, on sign-out. Defaults to a no-op.
  ///
  /// The offline copy is one user's data sitting on the device, and nothing in
  /// the engine knows whose — that is exactly what a [`LocalBackend`] keeps
  /// track of. Left in place across a sign-out it does not merely linger: the
  /// next person to sign in on this device is served that copy while their own
  /// data is still on its way, so they are shown a stranger's boards, documents
  /// or messages as if they were their own. Implement this to drop the mirror
  /// and the remembered identity; [`Engine::clear_local_data`] calls it and
  /// clears the replay queue alongside.
  async fn clear_local(&self) {}
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
      let started = Instant::now();
      match self.remote.send(remote).await {
        Ok(resp) => {
          let network = started.elapsed();
          let cached = Instant::now();
          if resp.is_success() {
            if req.method.is_read() {
              self.backend.cache_read(&req, &resp).await;
            } else {
              self.backend.observe_write(&req, &resp).await;
            }
          }
          // The local store is written while the UI waits for this response, so a
          // slow backend is indistinguishable from a slow server from the user's
          // seat. Name both, and say so out loud when it's bad enough to feel.
          report_timing(&req, network, cached.elapsed());
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
      let mut req = self
        .backend
        .prepare_outgoing(self.backend.rewrite_ids(&entry.req, &id_map));
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
        Ok(resp) => {
          return Err(format!("server rejected a queued write: HTTP {}", resp.status()));
        }
        Err(e) => return Err(format!("sync failed, will retry: {e}")),
      }
    }
    Ok(())
  }

  /// Forgets everything this device holds for the session that just ended: the
  /// backend's local store ([`LocalBackend::clear_local`]) and every write still
  /// waiting to be replayed.
  ///
  /// Call it on sign-out, from the same place the credentials are dropped. The
  /// queue goes with the store because the two only make sense together — a
  /// replay carries the *current* credentials ([`LocalBackend::prepare_outgoing`]),
  /// so writes kept across a sign-out would land on the server as whoever signs
  /// in next.
  pub async fn clear_local_data(&self) {
    self.queue.clear();
    self.backend.clear_local().await;
  }

  /// Fills the local store ahead of time, so what the user hasn't opened yet is
  /// still there when the network goes away. Runs the backend's
  /// [`prefetch_requests`](LocalBackend::prefetch_requests) against the server
  /// and caches each success; returns how many landed.
  ///
  /// Best-effort by design: a request that fails is skipped, and losing the
  /// connection mid-pass ends it (flipping the engine offline) rather than
  /// grinding through the rest. Call it periodically while online, and after a
  /// sync — nothing here is on the user's critical path.
  pub async fn prefetch(&self) -> usize {
    if !self.is_online() {
      return 0;
    }
    let started = Instant::now();
    let mut cached = 0;
    for req in self.backend.prefetch_requests().await {
      let mut outgoing = self.backend.prepare_outgoing(req.clone());
      outgoing.url = self.remote_url(&outgoing.url);
      match self.remote.send(outgoing).await {
        Ok(resp) if resp.is_success() => {
          self.backend.cache_read(&req, &resp).await;
          cached += 1;
        }
        Ok(resp) => tracing::debug!("prefetch skipped {}: HTTP {}", req.url, resp.status()),
        Err(e) => {
          tracing::warn!("prefetch stopped, connection lost: {e}");
          self.set_online(false);
          break;
        }
      }
    }
    // Requests run one after another, so a first pass over a large backlog is
    // expected to take a while; the log makes the cost visible instead of leaving
    // it to be guessed at from a spinner.
    tracing::debug!("prefetch cached {cached} response(s) in {:?}", started.elapsed());
    cached
  }

  fn remote_url(&self, url: &str) -> String {
    format!("{}{}", self.remote_base.trim_end_matches('/'), path_and_query(url))
  }
}

/// A request the user is waiting on shouldn't take longer than this. Past it, the
/// breakdown is logged as a warning rather than at debug level — the point being
/// that "the app feels slow" should never require a rebuild to investigate.
const SLOW_REQUEST: std::time::Duration = std::time::Duration::from_millis(750);

/// Logs where an online request's time went: on the wire, or in the local store.
fn report_timing(req: &HttpRequest, network: std::time::Duration, local: std::time::Duration) {
  let total = network + local;
  if total >= SLOW_REQUEST {
    tracing::warn!(
      "slow request: {} {} took {:?} ({:?} network, {:?} local store)",
      req.method.as_str(),
      path_and_query(&req.url),
      total,
      network,
      local,
    );
  } else {
    tracing::debug!(
      "{} {}: {:?} ({:?} network, {:?} local store)",
      req.method.as_str(),
      path_and_query(&req.url),
      total,
      network,
      local,
    );
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
