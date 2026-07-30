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
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use impulse_utils::prelude::ServerError;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AsyncMutex, Notify};

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
const RECONCILE_WAIT: Duration = Duration::from_secs(5);

/// The offline-capable WebSocket engine backing a Tauri app.
///
/// Construct it once, hand [`send`](Self::send) to the `ik_ws_send` command, and
/// run [`connect_and_run`](Self::connect_and_run) from a background task that the
/// shell restarts whenever the socket drops.
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
    })
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

  /// Handles one frame from the UI (`ik_ws_send`). Online, it writes to the server
  /// socket; offline — or if the write fails — it serves from the backend, emits
  /// any optimistic frames, and queues the frame for replay.
  pub async fn send(&self, frame: String) {
    if self.is_online() {
      let mut guard = self.sink.lock().await;
      if let Some(sink) = guard.as_mut() {
        match sink.send(frame.clone()).await {
          Ok(()) => return,
          Err(e) => {
            tracing::warn!("ws send failed, serving offline: {e}");
            *guard = None;
            drop(guard);
            self.go_offline();
          }
        }
      }
    }
    self.serve_offline(&frame).await;
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
  /// queue. Returns when the socket drops; the shell should call it again to
  /// reconnect.
  pub async fn connect_and_run(&self) -> Result<(), String> {
    let (sink, mut stream) = self.remote.connect(&self.url).await?;
    *self.sink.lock().await = Some(sink);
    self.online.store(true, Ordering::Relaxed);

    // The receive loop and the replay run concurrently, on purpose: a replayed
    // create is reconciled by the server's broadcast, which only arrives on the
    // receive loop — so `sync`'s wait for a real id would deadlock (until timeout)
    // if the loop weren't already draining the socket alongside it.
    let receive = async {
      while let Some(item) = stream.recv().await {
        match item {
          Ok(frame) => self.receive(frame).await,
          Err(e) => {
            tracing::warn!("ws stream error: {e}");
            break;
          }
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
    Ok(())
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
      {
        let mut guard = self.sink.lock().await;
        let Some(sink) = guard.as_mut() else {
          return Err("socket not connected".to_string());
        };
        if let Err(e) = sink.send(frame).await {
          *guard = None;
          drop(guard);
          self.go_offline();
          return Err(format!("sync stopped, socket lost: {e}"));
        }
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
