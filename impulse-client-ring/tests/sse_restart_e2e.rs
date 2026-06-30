//! End-to-end test for **SSE-stream recovery across a broker restart**.
//!
//! An `impulse-server-kit` Ring listener serves a never-ending SSE stream; a
//! client opens it and reads events. The `impulsed` broker is then killed and a
//! fresh one started (a new shared-memory generation). The *same* open
//! [`RingEventStream`](impulse_client_ring::streaming::RingEventStream) must
//! transparently re-handshake against the new broker and keep delivering events,
//! without the caller rebuilding anything.
//!
//! Like `restart_e2e`, this spawns the broker binary (`IMPULSED_BIN` or the
//! default `../impulse-ring/target/debug/impulsed`) and skips if it is absent.
#![cfg(feature = "async")]

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use futures_util::StreamExt;
use impulse_client_ring::ImpulseRingClient;
use impulse_server_kit::prelude::*;
use impulse_server_kit::setup::ProtocolConfig;
use salvo::sse::SseEvent;
use serde::Deserialize;

const APP: &str = "sse-restart-e2e";

struct Broker(Child);
impl Drop for Broker {
  fn drop(&mut self) {
    let _ = self.0.kill();
    let _ = self.0.wait();
  }
}

fn broker_binary() -> Option<PathBuf> {
  if let Ok(p) = std::env::var("IMPULSED_BIN") {
    let p = PathBuf::from(p);
    if p.exists() {
      return Some(p);
    }
  }
  let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  for rel in [
    "../../impulse-ring/target/debug/impulsed",
    "../impulse-ring/target/debug/impulsed",
  ] {
    let cand = manifest.join(rel);
    if cand.exists() {
      return Some(cand);
    }
  }
  None
}

/// Remove stale Ring shared-memory segments so a fresh broker does not collide on
/// the fixed control segment name.
fn cleanup_shm() {
  if let Ok(entries) = std::fs::read_dir("/dev/shm") {
    for entry in entries.flatten() {
      if entry.file_name().to_string_lossy().starts_with("impulse-ring.") {
        let _ = std::fs::remove_file(entry.path());
      }
    }
  }
}

fn start_broker() -> Option<Broker> {
  let bin = broker_binary()?;
  cleanup_shm();
  Command::new(bin)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .ok()
    .map(Broker)
}

fn restart_broker(mut old: Broker) -> Broker {
  let _ = old.0.kill();
  let _ = old.0.wait();
  std::mem::forget(old); // already reaped; skip the guard's Drop
  start_broker().expect("restart broker")
}

#[derive(Deserialize, Default, Clone)]
struct Setup {
  #[serde(flatten)]
  generic_values: GenericValues,
}
impl GenericSetup for Setup {
  fn generic_values(&self) -> &GenericValues {
    &self.generic_values
  }
  fn generic_values_mut(&mut self) -> &mut GenericValues {
    &mut self.generic_values
  }
}

// A never-ending SSE stream emitting a tick every 200ms.
#[handler]
async fn ticks(res: &mut Response) {
  let stream = futures_util::stream::unfold(0u64, |n| async move {
    tokio::time::sleep(Duration::from_millis(200)).await;
    Some((
      Ok::<_, std::convert::Infallible>(SseEvent::default().text(format!("tick {n}"))),
      n + 1,
    ))
  });
  salvo::sse::stream(res, stream);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sse_stream_recovers_after_broker_restart() {
  let Some(broker) = start_broker() else {
    eprintln!("skipping: `impulsed` broker binary not found (set IMPULSED_BIN)");
    return;
  };
  tokio::time::sleep(Duration::from_millis(300)).await;

  // ---- server ----
  let mut setup = Setup::default();
  setup.generic_values.app_name = APP.to_string();
  setup.generic_values.protocols = vec![ProtocolConfig::ImpulseRing {
    app_name: APP.to_string(),
    access_key: None,
    arena_size_kib: None,
  }];
  let state = load_generic_state(&setup, false).await.unwrap();
  let router = get_root_router_autoinject(&state, setup.clone()).push(Router::with_path("ticks").get(ticks));
  let (server, _handle) = start(state, &setup, router).await.unwrap();
  let _server_task = tokio::spawn(server);

  // ---- client ----
  let mut client = None;
  for _ in 0..100 {
    if let Ok(c) = ImpulseRingClient::connect(APP) {
      client = Some(c);
      break;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
  }
  let client = client.expect("could not connect to broker");

  // Open the SSE stream and confirm events arrive before the restart.
  let mut stream = None;
  for _ in 0..100 {
    if let Ok(s) = client.sse("/ticks").await {
      stream = Some(s);
      break;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
  }
  let mut stream = stream.expect("could not open the SSE stream");
  let first = tokio::time::timeout(Duration::from_secs(5), stream.next()).await;
  assert!(matches!(first, Ok(Some(_))), "no SSE event before the restart");

  // ---- restart the broker ----
  let _broker = restart_broker(broker);

  // The same stream must transparently re-handshake and keep delivering events.
  // Allow a generous window for the watcher reconnect + server re-expose + the
  // stream's own re-handshake to settle.
  let mut recovered = false;
  for _ in 0..150 {
    if let Ok(Some(Ok(_))) = tokio::time::timeout(Duration::from_millis(300), stream.next()).await {
      recovered = true;
      break;
    }
  }
  assert!(recovered, "the SSE stream did not recover after the broker restart");
}
