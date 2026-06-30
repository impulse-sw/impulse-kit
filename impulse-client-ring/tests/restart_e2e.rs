//! End-to-end test for **broker-restart recovery** of the Ring HTTP client.
//!
//! A server (an `impulse-server-kit` Ring listener) and a client connect through
//! `impulsed`; the broker is then killed and a fresh one started (a new
//! shared-memory generation). Both the server's bus connection and the client
//! transparently reconnect, so an HTTP request issued after the restart succeeds
//! without rebuilding the client.
//!
//! The test spawns the broker binary. Point `IMPULSED_BIN` at it, or rely on the
//! default `../impulse-ring/target/debug/impulsed`. If the broker cannot be
//! started the test skips (prints a notice) rather than failing.
#![cfg(feature = "async")]

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use impulse_client_ring::ImpulseRingClient;
use impulse_server_kit::prelude::*;
use impulse_server_kit::setup::ProtocolConfig;
use serde::Deserialize;

const APP: &str = "restart-e2e";

/// Kills the broker child on drop so segments are released between runs.
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

/// Remove stale Ring shared-memory segments; otherwise a fresh broker can
/// collide on the fixed control segment name.
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

/// Kill the current broker (no graceful unlink), reap it, GC its segments, and
/// start a fresh broker — a new shared-memory generation with a new epoch.
fn restart_broker(mut old: Broker) -> Broker {
  let _ = old.0.kill();
  let _ = old.0.wait();
  std::mem::forget(old); // already reaped; skip the guard's Drop (which would re-wait)
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
async fn client_and_server_recover_after_broker_restart() {
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
  let router = get_root_router_autoinject(&state, setup.clone()).push(Router::with_path("hello").get(hello));
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
  assert!(client.auto_reconnect(), "auto-reconnect is on by default");

  // The server may not have exposed its function yet; retry the first call.
  let mut ok = false;
  for _ in 0..100 {
    if get_hello(&client).await.as_deref() == Some("hi") {
      ok = true;
      break;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
  }
  assert!(ok, "listener never exposed its function before the restart");

  // ---- restart the broker ----
  let _broker = restart_broker(broker);

  // Both the server's bus connection and the client reconnect; the server
  // re-exposes its HTTP function and the client retries transparently. Allow a
  // generous window for the watcher-driven reconnect + re-expose to settle.
  let mut recovered = false;
  for _ in 0..150 {
    if get_hello(&client).await.as_deref() == Some("hi") {
      recovered = true;
      break;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
  }
  assert!(recovered, "client did not recover after the broker restart");
}
