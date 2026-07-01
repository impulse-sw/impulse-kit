//! End-to-end test for **graceful broker-restart recovery** of a Ring listener.
//!
//! Mirrors `restart_e2e`, but stops `impulsed` gracefully (SIGTERM, like Ctrl+C)
//! and waits out a real gap before restarting it — the way an operator restarts
//! the broker. Graceful shutdown unlinks the control segment and the data arenas
//! (a SIGKILL leaves them behind), so this exercises a different teardown. The
//! idle server listener, which never issues a call itself, must still re-register
//! on the fresh broker purely via the connector's background watcher.
//!
//! Lives in its own test binary (not alongside `restart_e2e`) because both manage
//! the single global broker; cargo runs test binaries sequentially, but tests
//! inside one binary run in parallel and would fight over the control segment.
//!
//! The test spawns the broker binary (`IMPULSED_BIN` or the default
//! `../impulse-ring/target/debug/impulsed`) and skips if it is absent.
#![cfg(feature = "async")]

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use impulse_client_ring::ImpulseRingClient;
use impulse_server_kit::prelude::*;
use impulse_server_kit::setup::ProtocolConfig;
use serde::Deserialize;

const APP: &str = "restart-e2e-graceful";

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
  let child = Command::new(bin)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .ok()?;
  Some(Broker(child))
}

/// Gracefully stop the broker (SIGTERM, like Ctrl+C — which unlinks the control
/// segment and arenas), wait out a real gap, then start a fresh broker.
fn graceful_restart_broker(mut old: Broker) -> Broker {
  let pid = old.0.id();
  let _ = Command::new("kill").arg("-TERM").arg(pid.to_string()).status();
  let _ = old.0.wait();
  std::mem::forget(old);
  std::thread::sleep(Duration::from_secs(2));
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

#[handler]
async fn hello() -> &'static str {
  "hi"
}

async fn get_hello(client: &ImpulseRingClient) -> Option<String> {
  match client.get("/hello").send().await {
    Ok(resp) if resp.status().as_u16() == 200 => resp.text().ok(),
    _ => None,
  }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn server_relistens_after_graceful_broker_restart() {
  let Some(broker) = start_broker() else {
    eprintln!("skipping: `impulsed` broker binary not found (set IMPULSED_BIN)");
    return;
  };
  tokio::time::sleep(Duration::from_millis(300)).await;

  let mut setup = Setup::default();
  setup.generic_values.app_name = APP.to_string();
  setup.generic_values.protocols = vec![ProtocolConfig::ImpulseRing {
    app_name: APP.to_string(),
    access_key: None,
    arena_size_kib: None,
  }];
  let state = load_generic_state(&setup, false).await.unwrap();
  let router = get_root_router_autoinject(&state, setup.clone()).push(Router::with_path("hello").get(hello));
  let (server, _handle) = start(state, &setup, router).await.unwrap();
  let _server_task = tokio::spawn(server);

  let mut client = None;
  for _ in 0..100 {
    if let Ok(c) = ImpulseRingClient::connect(APP) {
      client = Some(c);
      break;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
  }
  let client = client.expect("could not connect to broker");

  let mut ok = false;
  for _ in 0..100 {
    if get_hello(&client).await.as_deref() == Some("hi") {
      ok = true;
      break;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
  }
  assert!(ok, "listener never exposed its function before the restart");

  // ---- graceful stop, gap, restart ----
  let _broker = graceful_restart_broker(broker);

  // The idle listener must re-register on the fresh broker (watcher-driven) and
  // start answering again — without being rebuilt.
  let mut recovered = false;
  for _ in 0..150 {
    if get_hello(&client).await.as_deref() == Some("hi") {
      recovered = true;
      break;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
  }
  assert!(
    recovered,
    "listener did not re-register after a graceful broker restart"
  );
}
