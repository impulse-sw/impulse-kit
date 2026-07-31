//! Tests for the streaming offline engine, driven by an in-memory fake socket.
//!
//! The fake server holds items by id, assigns positive ids on create, and echoes
//! the client-ref (`tmp`) back in the `created` broadcast so id reconciliation can
//! be exercised without a real network.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use impulse_utils::prelude::ServerError;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::ws::{Emit, LocalReply, WsBackend, WsEngine, WsRemote, WsSink, WsStream};

// ─── fake transport ────────────────────────────────────────────────────────

/// Shared server state behind a connection's sink + stream.
#[derive(Clone)]
struct FakeServer {
  items: Arc<Mutex<HashMap<i64, String>>>,
  next_id: Arc<AtomicI64>,
  reachable: Arc<AtomicBool>,
  /// The channel feeding the currently-connected stream.
  out: Arc<Mutex<Option<mpsc::UnboundedSender<String>>>>,
}

impl FakeServer {
  fn new() -> Self {
    Self {
      items: Arc::new(Mutex::new(HashMap::new())),
      next_id: Arc::new(AtomicI64::new(100)),
      reachable: Arc::new(AtomicBool::new(true)),
      out: Arc::new(Mutex::new(None)),
    }
  }

  fn push(&self, frame: Value) {
    if let Some(tx) = self.out.lock().unwrap().as_ref() {
      let _ = tx.send(frame.to_string());
    }
  }

  fn handle(&self, frame: &str) {
    let msg: Value = serde_json::from_str(frame).unwrap();
    match msg["type"].as_str() {
      Some("hello") => {
        let items = self.items.lock().unwrap();
        let list: Vec<Value> = items.iter().map(|(id, c)| json!({ "id": id, "content": c })).collect();
        self.push(json!({ "type": "snapshot", "items": list }));
      }
      Some("create") => {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let content = msg["content"].as_str().unwrap_or_default().to_string();
        self.items.lock().unwrap().insert(id, content.clone());
        // Echo the client-ref `tmp` so the origin can reconcile its provisional id.
        self.push(json!({ "type": "created", "tmp": msg["tmp"], "id": id, "content": content }));
      }
      Some("edit") => {
        let id = msg["id"].as_i64().unwrap();
        let content = msg["content"].as_str().unwrap_or_default().to_string();
        self.items.lock().unwrap().insert(id, content);
      }
      _ => {}
    }
  }
}

struct FakeRemote {
  server: FakeServer,
}

struct FakeSink {
  server: FakeServer,
}

struct FakeStream {
  rx: mpsc::UnboundedReceiver<String>,
}

impl WsSink for FakeSink {
  async fn send(&mut self, frame: String) -> Result<(), String> {
    if !self.server.reachable.load(Ordering::Relaxed) {
      return Err("socket down".to_string());
    }
    self.server.handle(&frame);
    Ok(())
  }
}

impl WsStream for FakeStream {
  async fn recv(&mut self) -> Option<Result<String, String>> {
    self.rx.recv().await.map(Ok)
  }
}

impl WsRemote for FakeRemote {
  type Sink = FakeSink;
  type Stream = FakeStream;
  async fn connect(&self, _url: &str) -> Result<(Self::Sink, Self::Stream), String> {
    if !self.server.reachable.load(Ordering::Relaxed) {
      return Err("unreachable".to_string());
    }
    let (tx, rx) = mpsc::unbounded_channel();
    *self.server.out.lock().unwrap() = Some(tx);
    Ok((
      FakeSink {
        server: self.server.clone(),
      },
      FakeStream { rx },
    ))
  }
}

// ─── fake local store ────────────────────────────────────────────────────────

#[derive(Default)]
struct MemBackend {
  items: Mutex<HashMap<i64, String>>,
}

impl MemBackend {
  fn has(&self, id: i64) -> bool {
    self.items.lock().unwrap().contains_key(&id)
  }
  fn content(&self, id: i64) -> Option<String> {
    self.items.lock().unwrap().get(&id).cloned()
  }
}

impl WsBackend for MemBackend {
  async fn serve_local(&self, frame: &str, provisional: &dyn Fn() -> i64) -> Result<LocalReply, ServerError> {
    let msg: Value = serde_json::from_str(frame).map_err(|e| ServerError::from_public(e.to_string()).with_500())?;
    match msg["type"].as_str() {
      Some("hello") => {
        let items = self.items.lock().unwrap();
        let list: Vec<Value> = items.iter().map(|(id, c)| json!({ "id": id, "content": c })).collect();
        Ok(LocalReply::frames([
          json!({ "type": "snapshot", "items": list }).to_string()
        ]))
      }
      Some("create") => {
        let id = provisional();
        let content = msg["content"].as_str().unwrap_or_default().to_string();
        self.items.lock().unwrap().insert(id, content.clone());
        let event = json!({ "type": "created", "tmp": id, "id": id, "content": content });
        // The queued frame carries the provisional id as `tmp`, which the server
        // echoes back on replay so this client can reconcile it.
        let queued = json!({ "type": "create", "tmp": id, "content": content });
        Ok(LocalReply::created([event.to_string()], id, queued.to_string()))
      }
      Some("edit") => {
        let id = msg["id"].as_i64().unwrap_or_default();
        let content = msg["content"].as_str().unwrap_or_default().to_string();
        self.items.lock().unwrap().insert(id, content);
        Ok(LocalReply::frames([]))
      }
      _ => Err(ServerError::from_public("unknown frame").with_500()),
    }
  }

  async fn apply_event(&self, frame: &str) {
    let msg: Value = match serde_json::from_str(frame) {
      Ok(v) => v,
      Err(_) => return,
    };
    match msg["type"].as_str() {
      Some("snapshot") => {
        let mut items = self.items.lock().unwrap();
        for it in msg["items"].as_array().into_iter().flatten() {
          if let (Some(id), Some(c)) = (it["id"].as_i64(), it["content"].as_str()) {
            items.insert(id, c.to_string());
          }
        }
      }
      Some("created") => {
        if let (Some(id), Some(c)) = (msg["id"].as_i64(), msg["content"].as_str()) {
          self.items.lock().unwrap().insert(id, c.to_string());
        }
      }
      _ => {}
    }
  }

  fn should_queue(&self, frame: &str) -> bool {
    !frame.contains("\"hello\"")
  }

  fn created_id(&self, frame: &str) -> Option<(i64, i64)> {
    let msg: Value = serde_json::from_str(frame).ok()?;
    if msg["type"].as_str() == Some("created") {
      let tmp = msg["tmp"].as_i64()?;
      let real = msg["id"].as_i64()?;
      if tmp < 0 && tmp != real {
        return Some((tmp, real));
      }
    }
    None
  }

  async fn reconcile_id(&self, provisional: i64, real: i64) {
    let mut items = self.items.lock().unwrap();
    if let Some(content) = items.remove(&provisional) {
      items.entry(real).or_insert(content);
    }
  }

  fn rewrite_ids(&self, frame: &str, id_map: &HashMap<i64, i64>) -> String {
    let mut msg: Value = match serde_json::from_str(frame) {
      Ok(v) => v,
      Err(_) => return frame.to_string(),
    };
    if let Some(id) = msg["id"].as_i64()
      && let Some(real) = id_map.get(&id)
    {
      msg["id"] = json!(real);
    }
    msg.to_string()
  }
}

// ─── harness ────────────────────────────────────────────────────────────────

/// Collects frames the engine emits toward the webview, for assertions.
#[derive(Clone, Default)]
struct Emitted(Arc<Mutex<Vec<String>>>);

impl Emitted {
  fn sink(&self) -> Emit {
    let store = self.0.clone();
    Box::new(move |frame| store.lock().unwrap().push(frame))
  }
  fn any(&self, needle: &str) -> bool {
    self.0.lock().unwrap().iter().any(|f| f.contains(needle))
  }
}

fn queue_path() -> std::path::PathBuf {
  let mut p = std::env::temp_dir();
  p.push(format!("ik-ws-queue-{}.json", uuid_like()));
  let _ = std::fs::remove_file(&p);
  p
}

fn uuid_like() -> u128 {
  use std::time::{SystemTime, UNIX_EPOCH};
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
}

/// Polls `cond` up to ~2s so tests don't race the background receive loop.
async fn eventually(mut cond: impl FnMut() -> bool) -> bool {
  for _ in 0..200 {
    if cond() {
      return true;
    }
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
  }
  cond()
}

// ─── tests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn offline_send_serves_locally_and_queues() {
  let server = FakeServer::new();
  let emitted = Emitted::default();
  let engine = WsEngine::new(
    MemBackend::default(),
    FakeRemote { server: server.clone() },
    "ws://x",
    queue_path(),
    emitted.sink(),
  )
  .unwrap();

  // Never connected → offline. A create is served locally and queued.
  engine
    .send(json!({ "type": "create", "tmp": 0, "content": "hi" }).to_string())
    .await;

  assert_eq!(engine.pending_sync(), 1, "the create should be queued for replay");
  assert!(
    engine.backend().has(-1),
    "the create should be stored under a provisional id"
  );
  assert!(
    emitted.any("\"created\""),
    "an optimistic event should reach the webview"
  );
  // The server never saw it.
  assert_eq!(server.items.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn hello_is_not_queued() {
  let server = FakeServer::new();
  let emitted = Emitted::default();
  let engine = WsEngine::new(
    MemBackend::default(),
    FakeRemote { server },
    "ws://x",
    queue_path(),
    emitted.sink(),
  )
  .unwrap();

  engine.send(json!({ "type": "hello" }).to_string()).await;
  assert_eq!(engine.pending_sync(), 0, "a hello/snapshot read must not be queued");
  assert!(emitted.any("\"snapshot\""), "a local snapshot should be served offline");
}

#[tokio::test]
async fn reconnect_replays_queue_and_reconciles_id() {
  let server = FakeServer::new();
  let emitted = Emitted::default();
  let engine = Arc::new(
    WsEngine::new(
      MemBackend::default(),
      FakeRemote { server: server.clone() },
      "ws://x",
      queue_path(),
      emitted.sink(),
    )
    .unwrap(),
  );

  // Offline: create, then edit that same offline-created item.
  engine
    .send(json!({ "type": "create", "tmp": 0, "content": "first" }).to_string())
    .await;
  engine
    .send(json!({ "type": "edit", "id": -1, "content": "second" }).to_string())
    .await;
  assert_eq!(engine.pending_sync(), 2);

  // Reconnect: the receive loop + replay run concurrently.
  let bg = tokio::spawn({
    let engine = engine.clone();
    async move {
      let _ = engine.connect_and_run().await;
    }
  });

  // The queue drains and the provisional id is reconciled to the server's real id.
  assert!(
    eventually(|| engine.pending_sync() == 0).await,
    "queue should drain on reconnect"
  );
  assert!(
    eventually(|| engine.backend().has(100)).await,
    "local row should move to the real id"
  );
  assert!(!engine.backend().has(-1), "the provisional id should be gone");
  // The server holds the create and the follow-up edit against the real id.
  assert!(eventually(|| server.items.lock().unwrap().get(&100).map(String::as_str) == Some("second")).await);

  bg.abort();
}

#[tokio::test]
async fn online_send_forwards_and_broadcast_is_applied() {
  let server = FakeServer::new();
  server.items.lock().unwrap().insert(100, "existing".to_string());
  let emitted = Emitted::default();
  let engine = Arc::new(
    WsEngine::new(
      MemBackend::default(),
      FakeRemote { server: server.clone() },
      "ws://x",
      queue_path(),
      emitted.sink(),
    )
    .unwrap(),
  );

  let bg = tokio::spawn({
    let engine = engine.clone();
    async move {
      let _ = engine.connect_and_run().await;
    }
  });
  assert!(
    eventually(|| engine.is_online()).await,
    "engine should come online after connect"
  );

  // Online hello → forwarded to the server → snapshot broadcast → applied locally.
  engine.send(json!({ "type": "hello" }).to_string()).await;
  assert!(eventually(|| engine.backend().content(100).as_deref() == Some("existing")).await);
  assert!(
    emitted.any("\"snapshot\""),
    "the server snapshot should reach the webview"
  );
  assert_eq!(engine.pending_sync(), 0, "online sends are not queued");

  bg.abort();
}
