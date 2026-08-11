//! Offline-first engine for a **WebSocket/WebTransport** app surface — the
//! streaming sibling of the request/response [`Engine`](crate::Engine).
//!
//! A Tauri webview can't hold a socket to an arbitrary host, so the socket lives
//! natively and the UI talks to it over IPC: outgoing frames arrive via
//! `invoke("ik_ws_send", { text })` and inbound frames are pushed back as the
//! Tauri event `ik_ws_message` (this is the contract the client side,
//! `impulse_client_kit::ws_ipc`, already implements). The native half — holding
//! the real socket, serving frames locally while offline, and replaying queued
//! frames on reconnect — is this module.
//!
//! ```text
//!   webview ──ik_ws_send──▶ WsEngine::send ──▶ server socket        (online)
//!                                          └──▶ WsBackend::serve_local + queue (offline)
//!   server socket ──▶ WsBackend::apply_event ──▶ emit ik_ws_message ──▶ webview
//! ```
//!
//! ## Online vs offline
//!
//! * **online** — a frame is written straight to the server socket; the server's
//!   broadcasts come back on the receive loop, are folded into the local store
//!   ([`WsBackend::apply_event`]) and pushed to the webview.
//! * **offline** — the frame is answered from the local store
//!   ([`WsBackend::serve_local`], which may emit an optimistic event) and, if it
//!   is a write, appended to the persistent [`WsQueue`] for replay.
//! * **on reconnect** — [`WsEngine::sync`] replays the queued frames oldest-first
//!   over the socket, then the server's authoritative broadcasts refresh the UI.
//!
//! ## Staying connected
//!
//! [`WsEngine::run_reconnecting`] is the loop that keeps a socket up for the
//! life of the app, and it lives here rather than in each shell because getting
//! it right is not app-specific. Every wait it can make — the connect attempt,
//! every write, the backoff between attempts — is bounded by
//! [`ReconnectPolicy`] and cut short by [`lifecycle::wake`].
//!
//! That is the whole point of the type. A phone coming back from the background
//! holds connections that are dead without having been closed: pooled HTTP
//! sockets a token fetch will reuse, a TCP stream whose peer stopped listening
//! hours ago. An `await` on any of them never completes and never fails, so a
//! reconnect loop that reaches one simply stops — no pings, no timeouts, no
//! retry, nothing to see in a log. The socket's own keepalive cannot save it,
//! because the loop wedged *before* there was a socket to keep alive. Timeouts
//! on every step are what make that impossible, so the invariant is worth
//! stating plainly: **no await in the reconnect path may be unbounded.**
//!
//! ## Ids without a response
//!
//! A socket frame has no reply to read a server-assigned id from, so a WS create
//! is reconciled by **echo**: the app mints a provisional (negative) id offline,
//! embeds it in the frame it queues (as a client-ref the server echoes), and the
//! server includes both that ref and the real id in the resulting broadcast. The
//! receive loop hands every inbound frame to [`WsBackend::created_id`]; when it
//! returns `(provisional, real)` the engine reconciles the local row
//! ([`WsBackend::reconcile_id`]) and rewrites any still-queued follow-ups
//! ([`WsBackend::rewrite_ids`]). This works the same online, on replay, and for
//! frames a peer's action produced. Native (non-wasm) only.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use impulse_utils::prelude::ServerError;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AsyncMutex, Notify};

use crate::lifecycle;

/// The write half of a live server socket: sends one text frame.
///
/// Used only through generics (never as a `dyn` object), so `async fn` is fine.
#[allow(async_fn_in_trait)]
pub trait WsSink: Send {
  /// Writes one text frame to the server, or errors if the socket is broken.
  async fn send(&mut self, frame: String) -> Result<(), String>;
}

/// The read half of a live server socket: yields inbound text frames until the
/// socket closes (`None`).
#[allow(async_fn_in_trait)]
pub trait WsStream: Send {
  /// Awaits the next inbound frame. `None` means the socket has closed; an `Err`
  /// is a transport error that also ends the stream.
  async fn recv(&mut self) -> Option<Result<String, String>>;
}

/// Opens a live socket to the server, returning its split write/read halves.
///
/// The concrete transport (a signed, TLS-pinned WebSocket, a WebTransport
/// session, …) is supplied by the app shell — mirroring how the request/response
/// side leaves the production [`Remote`](crate::Remote) to the app. Injected so
/// the engine can be exercised against an in-memory fake in tests.
#[allow(async_fn_in_trait)]
pub trait WsRemote: Send + Sync {
  /// The write half returned by [`connect`](Self::connect).
  type Sink: WsSink;
  /// The read half returned by [`connect`](Self::connect).
  type Stream: WsStream;
  /// Connects to `url`, returning the socket's write and read halves.
  async fn connect(&self, url: &str) -> Result<(Self::Sink, Self::Stream), String>;
}

/// The result of serving one frame locally while offline: the frames to emit
/// back to the webview (e.g. an optimistic event mirroring what the server would
/// have broadcast), the provisional id minted for a create, and — for a create —
/// the frame to actually queue for replay.
///
/// The queued frame matters because the provisional (client-ref) id is minted
/// *here*, inside serving, and the UI's original frame can't carry it. So a
/// create returns a `queued` frame with that id embedded, which the server echoes
/// back in its broadcast so [`WsBackend::created_id`] can reconcile it.
pub struct LocalReply {
  /// Frames to push to the webview immediately (may be empty).
  pub emit: Vec<String>,
  /// The provisional (negative) id minted for a create; tags the queued frame for
  /// later reconciliation. `None` for non-creates.
  pub provisional: Option<i64>,
  /// The frame to enqueue for replay instead of the UI's original — used to embed
  /// the minted provisional id. `None` queues the original frame unchanged.
  pub queued: Option<String>,
}

impl LocalReply {
  /// A reply that emits `frames`, mints no id, and queues the original frame.
  pub fn frames(emit: impl IntoIterator<Item = String>) -> Self {
    Self {
      emit: emit.into_iter().collect(),
      provisional: None,
      queued: None,
    }
  }

  /// A create reply: emits `frames`, reports the `provisional` id it minted, and
  /// queues `queued` — the frame carrying that id as the client-ref the server
  /// will echo back for reconciliation.
  pub fn created(emit: impl IntoIterator<Item = String>, provisional: i64, queued: impl Into<String>) -> Self {
    Self {
      emit: emit.into_iter().collect(),
      provisional: Some(provisional),
      queued: Some(queued.into()),
    }
  }
}

/// The app-specific local behaviour behind the WS engine: how to serve frames
/// offline, fold server broadcasts into the local store, and reconcile
/// provisional ids. An app implements this over its own state (a SQLite handle,
/// the signed-in identity, …).
///
/// Used only through generics, so `async fn` is fine.
#[allow(async_fn_in_trait)]
pub trait WsBackend: Send + Sync {
  /// Serves a frame from the local store while offline. For a create, mint a
  /// provisional id with `provisional` (it draws from the queue, staying
  /// negative) and report it via [`LocalReply::created`]. Return an `Err` for a
  /// frame that simply can't be served offline.
  async fn serve_local(&self, frame: &str, provisional: &dyn Fn() -> i64) -> Result<LocalReply, ServerError>;

  /// Folds a server broadcast into the local store so it is available offline
  /// later (the streaming analogue of `LocalBackend::cache_read`). Defaults to a
  /// no-op.
  async fn apply_event(&self, _frame: &str) {}

  /// Whether a frame just served offline should be queued for replay. Defaults to
  /// "queue everything" — override for frames that are reads/subscriptions and
  /// change nothing on the server (e.g. a `Hello`/snapshot request), so answering
  /// them offline schedules no pointless replay.
  fn should_queue(&self, _frame: &str) -> bool {
    true
  }

  /// Last-chance rewrite of a frame the engine itself is about to replay, after
  /// [`rewrite_ids`](Self::rewrite_ids). Use it to stamp fresh credentials or a
  /// resend marker. Defaults to identity.
  fn prepare_outgoing(&self, frame: String) -> String {
    frame
  }

  /// The `(provisional, real)` id pair carried by a broadcast that resulted from
  /// a create — the app reads the client-ref it embedded and the server's real
  /// id out of `frame`. Returning `Some` drives reconciliation. Defaults to
  /// `None` (no id reconciliation).
  fn created_id(&self, _frame: &str) -> Option<(i64, i64)> {
    None
  }

  /// Reconciles a provisional id → real id in the local store once a create's
  /// broadcast is seen. Defaults to a no-op.
  async fn reconcile_id(&self, _provisional: i64, _real: i64) {}

  /// Rewrites a queued frame's provisional ids using the temp→real map learned so
  /// far, so a follow-up edit of an offline-created item targets the real id on
  /// replay. Defaults to identity.
  fn rewrite_ids(&self, frame: &str, _id_map: &HashMap<i64, i64>) -> String {
    frame.to_string()
  }

  /// Empties the local store, on sign-out. Defaults to a no-op.
  ///
  /// The mirror is one user's data sitting on the device, and nothing in the
  /// engine knows whose. Left in place across a sign-out it is what the *next*
  /// person to sign in on this device is served while their own data is still on
  /// its way — a stranger's boards shown as their own. Implement it to drop the
  /// mirror and the remembered identity; [`WsEngine::clear_local_data`] calls it
  /// and clears the replay queue alongside.
  async fn clear_local(&self) {}
}

/// Pushes a frame to the webview. The app shell wires this to
/// `app.emit("ik_ws_message", frame)`; the engine crate stays Tauri-free.
pub type Emit = Box<dyn Fn(String) + Send + Sync>;

/// One queued offline frame awaiting replay.
#[derive(Clone, Serialize, Deserialize)]
pub struct WsEntry {
  /// Monotonic queue id (for acking).
  pub id: u64,
  /// The frame to replay.
  pub frame: String,
  /// For a create, the provisional id it minted, reconciled after replay.
  pub provisional_id: Option<i64>,
}

#[derive(Serialize, Deserialize)]
struct WsState {
  next_id: u64,
  next_provisional: i64,
  entries: Vec<WsEntry>,
}

impl Default for WsState {
  fn default() -> Self {
    Self {
      next_id: 0,
      next_provisional: -1,
      entries: Vec::new(),
    }
  }
}

/// File-backed FIFO queue of pending offline frames — the streaming analogue of
/// [`Queue`](crate::Queue). Rewritten to a temp file and renamed on every change
/// so a crash mid-flush never truncates it; order is preserved so frames replay
/// in the sequence they were sent.
pub struct WsQueue {
  path: PathBuf,
  state: Mutex<WsState>,
}

impl WsQueue {
  /// Opens (or creates) the queue at `path`, loading any persisted frames.
  pub fn open(path: PathBuf) -> std::io::Result<Self> {
    let state = match std::fs::read(&path) {
      Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => WsState::default(),
      Err(e) => return Err(e),
    };
    Ok(Self {
      path,
      state: Mutex::new(state),
    })
  }

  /// Hands out the next provisional (negative) id for an offline create.
  pub fn next_provisional_id(&self) -> i64 {
    let mut state = self.state.lock().expect("ws queue mutex");
    let id = state.next_provisional;
    state.next_provisional -= 1;
    self.persist(&state);
    id
  }

  /// Appends a frame to replay later, tagging a create with the id it minted.
  pub fn enqueue(&self, frame: &str, provisional_id: Option<i64>) {
    let mut state = self.state.lock().expect("ws queue mutex");
    let id = state.next_id;
    state.next_id += 1;
    state.entries.push(WsEntry {
      id,
      frame: frame.to_string(),
      provisional_id,
    });
    self.persist(&state);
  }

  /// Pending frames, oldest first.
  pub fn pending(&self) -> Vec<WsEntry> {
    self.state.lock().expect("ws queue mutex").entries.clone()
  }

  /// Drops a replayed frame.
  pub fn ack(&self, id: u64) {
    let mut state = self.state.lock().expect("ws queue mutex");
    state.entries.retain(|e| e.id != id);
    self.persist(&state);
  }

  /// Drops every pending frame without replaying it.
  ///
  /// For sign-out, and only for sign-out: a queued frame belongs to the session
  /// that made it, and the socket it would replay over authenticates as whoever
  /// is signed in *now*. The counters are deliberately not reset, so ids stay
  /// monotonic across the wipe.
  pub fn clear(&self) {
    let mut state = self.state.lock().expect("ws queue mutex");
    state.entries.clear();
    self.persist(&state);
  }

  /// Number of pending frames.
  pub fn len(&self) -> usize {
    self.state.lock().expect("ws queue mutex").entries.len()
  }

  /// Whether the queue is empty.
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  fn persist(&self, state: &WsState) {
    if let Some(parent) = self.path.parent() {
      let _ = std::fs::create_dir_all(parent);
    }
    let Ok(bytes) = serde_json::to_vec(state) else {
      return;
    };
    let tmp = self.path.with_extension("ws.tmp");
    if std::fs::write(&tmp, &bytes).is_ok() {
      let _ = std::fs::rename(&tmp, &self.path);
    }
  }
}

/// How long [`WsEngine::sync`] waits for a create's broadcast (and thus its real
/// id) before replaying frames that may depend on it. Best-effort: on timeout the
/// dependent frame replays with whatever ids are known, and the server's echo
/// still reconciles the local store afterwards.
const RECONCILE_WAIT: Duration = Duration::from_secs(3);

/// The bounds [`WsEngine::run_reconnecting`] keeps the connection under: how long
/// a single attempt may take, how long a write may take, and how long to wait
/// between attempts.
///
/// Every field exists because the operation it covers can otherwise wait forever
/// on a phone (see the module docs), and every default is a small number of
/// seconds: nothing here may keep the app disconnected for longer than about
/// five, whatever goes wrong. A reconnect is cheap and an app that has silently
/// stopped updating is not, so where the two trade off, this trades towards
/// noticing sooner — a connect abandoned at five seconds is retried at once with
/// a fresh ticket, which is a far better outcome than waiting to find out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReconnectPolicy {
  /// How long one connect attempt — everything [`WsRemote::connect`] does, which
  /// typically includes minting a ticket over HTTP as well as the socket
  /// handshake — may take before it is abandoned and retried.
  pub connect_timeout: Duration,
  /// How long a single frame may take to reach the socket. A write that outlives
  /// this is treated as a lost connection: the socket is dropped (a half-written
  /// frame cannot be reused) and the engine goes offline, so the frame is served
  /// and queued locally instead.
  pub write_timeout: Duration,
  /// Wait before the first retry after a connection ends.
  pub initial_delay: Duration,
  /// Upper bound on the backed-off wait between attempts.
  pub max_delay: Duration,
  /// Multiplier applied to the wait after each failed attempt. `1` keeps it
  /// constant.
  pub backoff_factor: u32,
}

impl Default for ReconnectPolicy {
  fn default() -> Self {
    Self {
      connect_timeout: Duration::from_secs(5),
      write_timeout: Duration::from_secs(3),
      initial_delay: Duration::from_millis(500),
      max_delay: Duration::from_secs(3),
      backoff_factor: 2,
    }
  }
}

impl ReconnectPolicy {
  /// Set how long one connect attempt may take.
  pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
    self.connect_timeout = timeout;
    self
  }

  /// Set how long a single frame may take to reach the socket.
  pub fn with_write_timeout(mut self, timeout: Duration) -> Self {
    self.write_timeout = timeout;
    self
  }

  /// Set the wait before the first retry.
  pub fn with_initial_delay(mut self, delay: Duration) -> Self {
    self.initial_delay = delay;
    self
  }

  /// Set the upper bound on the wait between attempts.
  pub fn with_max_delay(mut self, delay: Duration) -> Self {
    self.max_delay = delay;
    self
  }

  /// Set the backoff multiplier applied after each failed attempt.
  pub fn with_backoff_factor(mut self, factor: u32) -> Self {
    self.backoff_factor = factor;
    self
  }

  fn next_delay(&self, delay: Duration) -> Duration {
    delay.saturating_mul(self.backoff_factor.max(1)).min(self.max_delay)
  }
}

/// The offline-capable WebSocket engine backing a Tauri app.
///
/// Construct it once, hand [`send`](Self::send) to the `ik_ws_send` command, and
/// spawn [`run_reconnecting`](Self::run_reconnecting) as a background task — that
/// is the whole shell-side lifecycle, and it is deliberately not something an app
/// writes for itself.
pub struct WsEngine<R: WsRemote, B: WsBackend> {
  remote: R,
  backend: B,
  url: String,
  emit: Emit,
  online: AtomicBool,
  sink: AsyncMutex<Option<R::Sink>>,
  queue: WsQueue,
  id_map: Mutex<HashMap<i64, i64>>,
  waiters: Mutex<HashMap<i64, Arc<Notify>>>,
  /// Raised to end the current connection from outside the receive loop — see
  /// [`WsEngine::drop_socket`].
  reset: Notify,
  /// Bumped by [`WsEngine::drop_socket`]; a connection that no longer matches it
  /// stops applying what it receives.
  epoch: std::sync::atomic::AtomicU64,
  policy: ReconnectPolicy,
}

impl<R: WsRemote, B: WsBackend> WsEngine<R, B> {
  /// Builds an engine that connects to `url` through `remote`, serves offline
  /// through `backend`, persists queued frames at `queue_path`, and pushes inbound
  /// frames to the webview through `emit`.
  pub fn new(
    backend: B,
    remote: R,
    url: impl Into<String>,
    queue_path: impl Into<PathBuf>,
    emit: Emit,
  ) -> std::io::Result<Self> {
    Ok(Self {
      remote,
      backend,
      url: url.into(),
      emit,
      online: AtomicBool::new(false),
      sink: AsyncMutex::new(None),
      queue: WsQueue::open(queue_path.into())?,
      id_map: Mutex::new(HashMap::new()),
      waiters: Mutex::new(HashMap::new()),
      reset: Notify::new(),
      epoch: std::sync::atomic::AtomicU64::new(0),
      policy: ReconnectPolicy::default(),
    })
  }

  /// Overrides the [`ReconnectPolicy`]. The default is what an app should
  /// normally want; this is for a shell with an unusual transport (a connect
  /// that is legitimately slower than five seconds, say).
  pub fn with_reconnect_policy(mut self, policy: ReconnectPolicy) -> Self {
    self.policy = policy;
    self
  }

  /// The app's local backend (e.g. to set the signed-in identity on it).
  pub fn backend(&self) -> &B {
    &self.backend
  }

  /// Whether the engine currently holds a live server socket.
  pub fn is_online(&self) -> bool {
    self.online.load(Ordering::Relaxed)
  }

  /// Number of frames waiting to be replayed to the server.
  pub fn pending_sync(&self) -> usize {
    self.queue.len()
  }

  /// Ends the current connection, so the next one is opened under whatever
  /// credentials are current. Frames still in flight on the old socket are
  /// discarded rather than applied.
  ///
  /// The socket is the one thing in a Tauri app that outlives the page: signing
  /// out reloads the webview, but the shell's connection — opened with a ticket
  /// minted for the session that just ended — carries on receiving that user's
  /// broadcasts. [`run_reconnecting`](Self::run_reconnecting) picks it up from
  /// here and reconnects, which succeeds again once somebody has signed in.
  pub fn drop_socket(&self) {
    self.epoch.fetch_add(1, Ordering::Relaxed);
    self.go_offline();
    self.reset.notify_waiters();
  }

  /// Forgets everything this device holds for the session that just ended: the
  /// backend's mirror ([`WsBackend::clear_local`]), every frame still waiting to
  /// be replayed, the id reconciliation learned along the way, and the socket
  /// that was carrying it all.
  ///
  /// Call it on sign-out, from the same place the credentials are dropped. All
  /// four go together: a mirror kept across a sign-out is what the next person
  /// on this device is shown while their own data loads, and a queued frame
  /// kept alongside it would replay over the *next* session's socket — one
  /// person's work sent as another.
  pub async fn clear_local_data(&self) {
    self.drop_socket();
    self.queue.clear();
    self.id_map.lock().expect("id map").clear();
    self.waiters.lock().expect("waiters").clear();
    self.backend.clear_local().await;
  }

  /// Handles one frame from the UI (`ik_ws_send`). Online, it writes to the server
  /// socket; offline — or if the write fails — it serves from the backend, emits
  /// any optimistic frames, and queues the frame for replay.
  pub async fn send(&self, frame: String) {
    if self.is_online() {
      match self.write_frame(frame.clone()).await {
        Ok(()) => return,
        Err(e) => tracing::warn!("ws send failed, serving offline: {e}"),
      }
    }
    self.serve_offline(&frame).await;
  }

  /// Writes one frame to the live socket, bounded by
  /// [`ReconnectPolicy::write_timeout`]. Any failure — including running out of
  /// time — drops the socket and flips the engine offline, so the caller can fall
  /// back to the local store and the reconnect loop can take over.
  ///
  /// The timeout is not decoration. A write to a peer that vanished without
  /// closing sits in the kernel's send buffer and, once that fills, never
  /// completes; because it holds the sink while it waits, an unbounded one takes
  /// the *next* connect attempt down with it, which is a stall no keepalive can
  /// break.
  async fn write_frame(&self, frame: String) -> Result<(), String> {
    let write = async {
      let mut guard = self.sink.lock().await;
      let Some(sink) = guard.as_mut() else {
        return Err("socket not connected".to_string());
      };
      let result = sink.send(frame).await;
      if result.is_err() {
        *guard = None;
      }
      result
    };
    match tokio::time::timeout(self.policy.write_timeout, write).await {
      Ok(Ok(())) => Ok(()),
      Ok(Err(e)) => {
        self.go_offline();
        Err(e)
      }
      Err(_) => {
        // Cancelling mid-write may have left a partial frame in the sink, so the
        // socket is unusable even if it later recovers: drop it.
        self.go_offline();
        self.clear_sink().await;
        Err(format!("write timed out after {:?}", self.policy.write_timeout))
      }
    }
  }

  /// Drops the current socket, bounded so a wedged writer can't hold the loop
  /// here. Best-effort: failing to take the lock only means the socket is still
  /// being written to, and the next successful connect replaces it anyway.
  async fn clear_sink(&self) {
    if let Ok(mut guard) = tokio::time::timeout(self.policy.write_timeout, self.sink.lock()).await {
      *guard = None;
    }
  }

  async fn serve_offline(&self, frame: &str) {
    let mint = || self.queue.next_provisional_id();
    match self.backend.serve_local(frame, &mint).await {
      Ok(reply) => {
        if self.backend.should_queue(frame) {
          let queued = reply.queued.as_deref().unwrap_or(frame);
          self.queue.enqueue(queued, reply.provisional);
        }
        for out in reply.emit {
          (self.emit)(out);
        }
      }
      Err(err) => {
        tracing::debug!("frame not serveable offline: {:?}", err.public_msg);
      }
    }
  }

  /// Connects the socket and runs the receive loop until it closes, folding every
  /// inbound frame into the local store, reconciling create ids, and pushing the
  /// frame to the webview. On a successful connect it flips online and replays the
  /// queue. Returns when the socket drops.
  ///
  /// Prefer [`run_reconnecting`](Self::run_reconnecting), which calls this in the
  /// loop it belongs in; this is public for a shell that needs one attempt, and
  /// for tests.
  ///
  /// The attempt is bounded by [`ReconnectPolicy::connect_timeout`] and abandoned
  /// on [`lifecycle::wake`] — a connect that was in flight while the app was in
  /// the background is reaching for a network the app may no longer be on, and
  /// waiting for it to notice is the stall this exists to prevent.
  pub async fn connect_and_run(&self) -> Result<(), String> {
    let (sink, mut stream) = tokio::select! {
      biased;
      _ = lifecycle::resumed() => return Err("app resumed mid-connect; retrying on a fresh socket".to_string()),
      connected = tokio::time::timeout(self.policy.connect_timeout, self.remote.connect(&self.url)) => match connected {
        Ok(halves) => halves?,
        Err(_) => return Err(format!("connect timed out after {:?}", self.policy.connect_timeout)),
      },
    };
    // Bounded like every other wait here: a previous write still wedged on the
    // dead socket holds this lock, and blocking on it would sink the very attempt
    // meant to replace that socket.
    match tokio::time::timeout(self.policy.write_timeout, self.sink.lock()).await {
      Ok(mut guard) => *guard = Some(sink),
      Err(_) => return Err("previous socket is still being written to; retrying".to_string()),
    }
    self.online.store(true, Ordering::Relaxed);

    // Which session this socket belongs to. A sign-out bumps it, and every frame
    // is checked against it before being applied: a broadcast already in flight
    // when the local store was wiped must not be folded back into it, or the
    // wipe is undone by the very data it was meant to forget.
    let epoch = self.epoch.load(Ordering::Relaxed);

    // The receive loop and the replay run concurrently, on purpose: a replayed
    // create is reconciled by the server's broadcast, which only arrives on the
    // receive loop — so `sync`'s wait for a real id would deadlock (until timeout)
    // if the loop weren't already draining the socket alongside it.
    let receive = async {
      // Registered before the first frame is awaited, so a `drop_socket` landing
      // mid-frame still wakes this loop rather than being announced to nobody.
      let reset = self.reset.notified();
      tokio::pin!(reset);
      loop {
        let item = tokio::select! {
          biased;
          _ = &mut reset => break,
          item = stream.recv() => item,
        };
        if self.epoch.load(Ordering::Relaxed) != epoch {
          break;
        }
        match item {
          Some(Ok(frame)) => self.receive(frame).await,
          Some(Err(e)) => {
            tracing::warn!("ws stream error: {e}");
            break;
          }
          None => break,
        }
      }
    };
    let replay = async {
      if let Err(e) = self.sync().await {
        tracing::warn!("post-reconnect sync did not finish: {e}");
      }
    };
    tokio::join!(receive, replay);

    self.go_offline();
    // Drop the dead socket rather than leave it for the keepalive to ping and for
    // the next attempt to wait on.
    self.clear_sink().await;
    Ok(())
  }

  /// Keeps the socket up for as long as the app runs: connect, serve until it
  /// drops, wait, connect again. Never returns — spawn it once at startup.
  ///
  /// There is no connectivity pre-check: a bounded connect attempt *is* the
  /// probe, and one that fails costs a backoff rather than a wasted request. A
  /// [`lifecycle::wake`] collapses the wait, so returning to the foreground
  /// reconnects at once instead of serving out a backoff measured against a
  /// network the app may no longer be on.
  pub async fn run_reconnecting(&self) {
    self.run_reconnecting_with(|| async {}).await
  }

  /// [`run_reconnecting`](Self::run_reconnecting) with `after_cycle` run once
  /// after every connection ends — for a shell with its own queue to drain
  /// alongside the engine's (uploads that a socket frame can't carry, say).
  ///
  /// It runs inline, so it holds up the next attempt: whatever it awaits must be
  /// bounded. Anything going through the crate's shared `executor::client`
  /// already is.
  pub async fn run_reconnecting_with<F, Fut>(&self, after_cycle: F)
  where
    F: Fn() -> Fut,
    Fut: Future<Output = ()>,
  {
    let mut delay = self.policy.initial_delay;
    loop {
      match self.connect_and_run().await {
        Ok(()) => delay = self.policy.initial_delay,
        Err(e) => tracing::debug!("ws connection attempt ended: {e}"),
      }
      after_cycle().await;
      let resumed = tokio::select! {
        _ = tokio::time::sleep(delay) => false,
        _ = lifecycle::resumed() => true,
      };
      delay = if resumed {
        self.policy.initial_delay
      } else {
        self.policy.next_delay(delay)
      };
    }
  }

  async fn receive(&self, frame: String) {
    self.backend.apply_event(&frame).await;
    if let Some((provisional, real)) = self.backend.created_id(&frame) {
      self.id_map.lock().expect("id map").insert(provisional, real);
      self.backend.reconcile_id(provisional, real).await;
      if let Some(n) = self.waiters.lock().expect("waiters").remove(&provisional) {
        n.notify_waiters();
      }
    }
    (self.emit)(frame);
  }

  fn go_offline(&self) {
    self.online.store(false, Ordering::Relaxed);
  }

  /// Replays queued frames against the server oldest-first, dropping each once
  /// written. For a create it then waits briefly for the server's broadcast so a
  /// dependent follow-up replays against the reconciled id. Stops at the first
  /// send failure, leaving the rest queued (and flipping offline).
  pub async fn sync(&self) -> Result<(), String> {
    for entry in self.queue.pending() {
      let rewritten = {
        let map = self.id_map.lock().expect("id map");
        self.backend.rewrite_ids(&entry.frame, &map)
      };
      let frame = self.backend.prepare_outgoing(rewritten);
      if let Err(e) = self.write_frame(frame).await {
        return Err(format!("sync stopped, socket lost: {e}"));
      }
      self.queue.ack(entry.id);
      if let Some(provisional) = entry.provisional_id {
        self.await_reconcile(provisional).await;
      }
    }
    Ok(())
  }

  /// Waits (bounded) for `provisional` to be reconciled by an inbound broadcast,
  /// so dependent frames replay against the real id. Returns immediately if it is
  /// already known.
  async fn await_reconcile(&self, provisional: i64) {
    let notify = {
      if self.id_map.lock().expect("id map").contains_key(&provisional) {
        return;
      }
      self
        .waiters
        .lock()
        .expect("waiters")
        .entry(provisional)
        .or_insert_with(|| Arc::new(Notify::new()))
        .clone()
    };
    let _ = tokio::time::timeout(RECONCILE_WAIT, notify.notified()).await;
  }
}

#[cfg(test)]
#[path = "ws_tests.rs"]
mod ws_tests;
