//! End-to-end tests of the engine orchestration against a fake remote and an
//! in-memory local backend:
//! * an online read forwards to the server and is cached locally;
//! * a write made offline is served locally and queued;
//! * on reconnect the queued writes replay, and an offline-created item's
//!   temporary id is reconciled with the server's real id (including a follow-up
//!   edit that referenced the temporary id).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use impulse_endpoint::{HttpRequest, HttpResponse, Method};
use impulse_utils::prelude::{CResult, ClientError, ServerError};
use serde::{Deserialize, Serialize};

use crate::{Engine, LocalBackend, Remote, path_and_query};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
struct Item {
  id: i64,
  content: String,
}

fn json_resp(status: u16, value: &impl Serialize) -> HttpResponse {
  HttpResponse {
    status,
    headers: vec![("content-type".into(), "application/json".into())],
    body: serde_json::to_vec(value).unwrap(),
  }
}

/// `/items` or `/items/{id}` → the trailing id, if present.
fn item_id(url: &str) -> Option<i64> {
  let pq = path_and_query(url);
  let path = pq.split('?').next().unwrap_or("");
  let segs: Vec<&str> = path.trim_matches('/').split('/').collect();
  match segs.as_slice() {
    ["items", id] => id.parse().ok(),
    _ => None,
  }
}

/// A stand-in server holding items by id, assigning positive ids on create, and
/// switchable "unreachable" to simulate the network being down.
#[derive(Clone)]
struct FakeRemote {
  reachable: Arc<AtomicBool>,
  items: Arc<Mutex<HashMap<i64, Item>>>,
  next_id: Arc<AtomicI64>,
}

impl FakeRemote {
  fn new() -> Self {
    Self {
      reachable: Arc::new(AtomicBool::new(true)),
      items: Arc::new(Mutex::new(HashMap::new())),
      next_id: Arc::new(AtomicI64::new(100)),
    }
  }
  fn set_reachable(&self, v: bool) {
    self.reachable.store(v, Ordering::Relaxed);
  }
  fn content_of(&self, id: i64) -> Option<String> {
    self.items.lock().unwrap().get(&id).map(|i| i.content.clone())
  }
  fn count(&self) -> usize {
    self.items.lock().unwrap().len()
  }
}

impl Remote for FakeRemote {
  async fn send(&self, req: HttpRequest) -> CResult<HttpResponse> {
    if !self.reachable.load(Ordering::Relaxed) {
      return Err(ClientError::from_str("network down"));
    }
    let body: Option<Item> = req.body.as_ref().and_then(|b| serde_json::from_slice(b).ok());
    let mut items = self.items.lock().unwrap();
    let resp = match (req.method, item_id(&req.url)) {
      (Method::Post, _) => {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let item = Item {
          id,
          content: body.map(|b| b.content).unwrap_or_default(),
        };
        items.insert(id, item.clone());
        json_resp(200, &item)
      }
      (Method::Put, Some(id)) => {
        let item = Item {
          id,
          content: body.map(|b| b.content).unwrap_or_default(),
        };
        items.insert(id, item.clone());
        json_resp(200, &item)
      }
      (Method::Get, Some(id)) => match items.get(&id) {
        Some(item) => json_resp(200, item),
        None => json_resp(404, &serde_json::json!({ "err": "not found" })),
      },
      _ => json_resp(400, &serde_json::json!({ "err": "bad request" })),
    };
    Ok(resp)
  }
}

/// An in-memory local store standing in for an app's offline SQLite.
#[derive(Default)]
struct MemBackend {
  items: Mutex<HashMap<i64, Item>>,
  cached_reads: Mutex<usize>,
}

impl LocalBackend for MemBackend {
  async fn serve_local(
    &self,
    req: &HttpRequest,
    provisional: &dyn Fn() -> i64,
  ) -> Result<(HttpResponse, Option<i64>), ServerError> {
    let body: Option<Item> = req.body.as_ref().and_then(|b| serde_json::from_slice(b).ok());
    let mut items = self.items.lock().unwrap();
    match (req.method, item_id(&req.url)) {
      (Method::Post, _) => {
        let id = provisional();
        let item = Item {
          id,
          content: body.map(|b| b.content).unwrap_or_default(),
        };
        items.insert(id, item.clone());
        Ok((json_resp(200, &item), Some(id)))
      }
      (Method::Put, Some(id)) => {
        let item = Item {
          id,
          content: body.map(|b| b.content).unwrap_or_default(),
        };
        items.insert(id, item.clone());
        Ok((json_resp(200, &item), None))
      }
      (Method::Get, Some(id)) => match items.get(&id) {
        Some(item) => Ok((json_resp(200, item), None)),
        None => Err(ServerError::from_public("not found").with_code(impulse_utils::prelude::StatusCode::NOT_FOUND)),
      },
      _ => Err(ServerError::from_public("This action isn't available offline")
        .with_code(impulse_utils::prelude::StatusCode::SERVICE_UNAVAILABLE)),
    }
  }

  async fn cache_read(&self, req: &HttpRequest, resp: &HttpResponse) {
    if let Ok(item) = serde_json::from_slice::<Item>(&resp.body) {
      self.items.lock().unwrap().insert(item.id, item);
      *self.cached_reads.lock().unwrap() += 1;
    }
    let _ = req;
  }

  fn created_id(&self, resp: &HttpResponse) -> Option<i64> {
    serde_json::from_slice::<Item>(&resp.body).ok().map(|i| i.id)
  }

  async fn reconcile_id(&self, provisional: i64, real: i64) {
    let mut items = self.items.lock().unwrap();
    if let Some(mut item) = items.remove(&provisional) {
      item.id = real;
      items.insert(real, item);
    }
  }

  fn rewrite_ids(&self, req: &HttpRequest, id_map: &HashMap<i64, i64>) -> HttpRequest {
    let mut out = req.clone();
    if let Some(id) = item_id(&req.url)
      && let Some(&real) = id_map.get(&id)
    {
      out.url = format!("/items/{real}");
    }
    out
  }
}

fn post(content: &str) -> HttpRequest {
  HttpRequest {
    method: Method::Post,
    url: "/items".into(),
    headers: vec![],
    body: Some(serde_json::to_vec(&serde_json::json!({ "id": 0, "content": content })).unwrap()),
    credentials: true,
  }
}

fn put(id: i64, content: &str) -> HttpRequest {
  HttpRequest {
    method: Method::Put,
    url: format!("/items/{id}"),
    headers: vec![],
    body: Some(serde_json::to_vec(&serde_json::json!({ "id": id, "content": content })).unwrap()),
    credentials: true,
  }
}

fn get(id: i64) -> HttpRequest {
  HttpRequest {
    method: Method::Get,
    url: format!("/items/{id}"),
    headers: vec![],
    body: None,
    credentials: true,
  }
}

fn engine(remote: FakeRemote) -> Engine<FakeRemote, MemBackend> {
  let mut path = std::env::temp_dir();
  path.push(format!(
    "ite-queue-{}-{}.json",
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ));
  Engine::new(MemBackend::default(), remote, "https://example.test", path).unwrap()
}

#[tokio::test]
async fn online_read_forwards_and_caches() {
  let remote = FakeRemote::new();
  remote.items.lock().unwrap().insert(7, Item { id: 7, content: "hi".into() });
  let eng = engine(remote.clone());

  let resp = eng.handle(get(7)).await;
  assert_eq!(resp.status, 200);
  assert_eq!(serde_json::from_slice::<Item>(&resp.body).unwrap().content, "hi");
  // The read was cached locally, so it's available offline.
  assert_eq!(*eng.backend().cached_reads.lock().unwrap(), 1);
  remote.set_reachable(false);
  let offline = eng.handle(get(7)).await;
  assert_eq!(offline.status, 200);
}

#[tokio::test]
async fn offline_write_is_queued_then_replayed() {
  let remote = FakeRemote::new();
  remote.items.lock().unwrap().insert(7, Item { id: 7, content: "old".into() });
  let eng = engine(remote.clone());

  // Go offline, edit — served locally and queued, server unchanged.
  remote.set_reachable(false);
  eng.set_online(false);
  let resp = eng.handle(put(7, "new")).await;
  assert_eq!(resp.status, 200);
  assert_eq!(eng.pending_sync(), 1);
  assert_eq!(remote.content_of(7).as_deref(), Some("old"));

  // Reconnect + sync: the edit lands on the server.
  remote.set_reachable(true);
  eng.set_online(true);
  eng.sync().await.unwrap();
  assert_eq!(eng.pending_sync(), 0);
  assert_eq!(remote.content_of(7).as_deref(), Some("new"));
}

#[tokio::test]
async fn offline_create_then_edit_reconciles_id() {
  let remote = FakeRemote::new();
  let eng = engine(remote.clone());

  remote.set_reachable(false);
  eng.set_online(false);

  // Create offline → temporary negative id.
  let created = eng.handle(post("draft")).await;
  let temp = serde_json::from_slice::<Item>(&created.body).unwrap().id;
  assert!(temp < 0, "offline create should mint a negative id, got {temp}");

  // Edit that offline-created item, still offline (references the temp id).
  eng.handle(put(temp, "draft v2")).await;
  assert_eq!(eng.pending_sync(), 2);

  // Reconnect + sync: create replays (server assigns a real id), and the queued
  // edit is rewritten onto the real id.
  remote.set_reachable(true);
  eng.set_online(true);
  eng.sync().await.unwrap();

  assert_eq!(eng.pending_sync(), 0);
  assert_eq!(remote.count(), 1);
  // The single server item carries the latest content under a real (positive) id.
  let (real_id, content) = {
    let items = remote.items.lock().unwrap();
    let (id, item) = items.iter().next().unwrap();
    (*id, item.content.clone())
  };
  assert!(real_id > 0);
  assert_eq!(content, "draft v2");
  // Local store was reconciled onto the real id.
  assert!(eng.backend().items.lock().unwrap().contains_key(&real_id));
}
