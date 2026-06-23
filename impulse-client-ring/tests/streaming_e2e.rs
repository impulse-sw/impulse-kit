//! End-to-end tests for the Ring HTTP client against a real `impulsed` broker
//! and an `impulse-server-kit` listener: plain HTTP, MsgPack bodies and SSE.
//!
//! The test spawns the broker binary. Point `IMPULSED_BIN` at it, or rely on the
//! default `../impulse-ring/target/debug/impulsed`. If the broker cannot be
//! started the test skips (prints a notice) rather than failing.
#![cfg(feature = "async")]

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use impulse_client_ring::ImpulseRingClient;
use impulse_server_kit::prelude::*;
use impulse_server_kit::salvo;
use impulse_server_kit::salvo::http::header::{CONTENT_TYPE, HeaderValue};
use impulse_server_kit::salvo::sse::{self, SseEvent};
use impulse_server_kit::salvo::websocket::WebSocketUpgrade;
use impulse_server_kit::setup::ProtocolConfig;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::Message as TMsg;

const APP: &str = "stream-e2e";

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

/// Remove stale Ring shared-memory segments left by a previously killed broker;
/// otherwise a fresh broker can collide on the fixed control segment name.
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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Msg {
  text: String,
  n: u32,
}

#[handler]
async fn hello() -> &'static str {
  "hi"
}

#[handler]
async fn mp_echo(req: &mut Request, res: &mut Response) {
  // Echo the raw request body straight back. Use a generous size limit so the
  // large-body (streamed-request) case is not capped by salvo's 64 KiB default.
  let body = req
    .payload_with_max_size(8 * 1024 * 1024)
    .await
    .map(|b| b.to_vec())
    .unwrap_or_default();
  res
    .headers_mut()
    .insert(CONTENT_TYPE, HeaderValue::from_static("application/msgpack"));
  res.write_body(body).ok();
}

#[handler]
async fn ws_echo(req: &mut Request, res: &mut Response) -> Result<(), salvo::http::StatusError> {
  WebSocketUpgrade::new()
    .upgrade(req, res, |mut ws| async move {
      while let Some(Ok(msg)) = ws.recv().await {
        if (msg.is_text() || msg.is_binary()) && ws.send(msg).await.is_err() {
          break;
        }
      }
    })
    .await
}

#[handler]
async fn sse_handler(res: &mut Response) {
  let events = futures_util::stream::iter(vec![
    Ok::<_, std::convert::Infallible>(SseEvent::default().text("one")),
    Ok(SseEvent::default().text("two")),
    Ok(SseEvent::default().text("three")),
  ]);
  sse::stream(res, events);
}

async fn connect_with_retry(app: &str) -> ImpulseRingClient {
  for _ in 0..100 {
    if let Ok(c) = ImpulseRingClient::connect(app) {
      return c;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
  }
  panic!("could not connect to '{app}' (broker/server not ready)");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn plain_msgpack_sse_websocket_and_webtransport_over_ring() {
  let Some(_broker) = start_broker() else {
    eprintln!("skipping: `impulsed` broker binary not found (set IMPULSED_BIN)");
    return;
  };
  // Give the broker a moment to create its control segment.
  tokio::time::sleep(Duration::from_millis(300)).await;

  // ---- server ----
  let mut setup = Setup::default();
  setup.generic_values.app_name = APP.to_string();
  setup.generic_values.protocols = vec![ProtocolConfig::ImpulseRing {
    app_name: APP.to_string(),
    access_key: None,
    // Exercise the per-service arena knob (1 MiB request arena).
    arena_size_kib: Some(1024),
  }];
  let state = load_generic_state(&setup, false).await.unwrap();
  let router = get_root_router_autoinject(&state, setup.clone())
    .push(Router::with_path("hello").get(hello))
    .push(Router::with_path("mp").post(mp_echo))
    .push(Router::with_path("sse").get(sse_handler))
    .push(Router::with_path("ws").goal(ws_echo));
  let (server, _handle) = start(state, &setup, router).await.unwrap();
  let server_task = tokio::spawn(server);

  // ---- client ----
  let client = connect_with_retry(APP).await;

  // Connecting only reaches the broker; the server may not have exposed its
  // function yet. Retry the first call until the listener has registered.
  let mut r = None;
  for _ in 0..100 {
    match client.get("/hello").send().await {
      Ok(resp) => {
        r = Some(resp);
        break;
      }
      Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
    }
  }

  // plain HTTP
  let r = r.expect("listener never exposed its function");
  assert_eq!(r.status().as_u16(), 200);
  assert_eq!(r.text().unwrap(), "hi");

  // MsgPack round-trip
  let msg = Msg {
    text: "ring".into(),
    n: 42,
  };
  let resp = client.post("/mp").msgpack(&msg).unwrap().send().await.unwrap();
  let got: Msg = resp.msgpack().unwrap();
  assert_eq!(got, msg);

  // Large request body: bigger than `MAX_INLINE_REQUEST_BODY` (192 KiB), so the
  // client streams it over a Ring channel instead of shipping it inline through
  // the function request ring. `/mp` echoes the raw body straight back.
  let big: Vec<u8> = (0..300 * 1024).map(|i| (i % 251) as u8).collect();
  let resp = client.post("/mp").body(big.clone()).send().await.unwrap();
  assert_eq!(resp.status().as_u16(), 200);
  assert_eq!(resp.bytes(), big, "large streamed request body did not round-trip");

  // SSE: collect three events
  let mut stream = client.sse("/sse").await.unwrap();
  let mut collected = String::new();
  for _ in 0..3 {
    let chunk = tokio::time::timeout(Duration::from_secs(5), stream.recv())
      .await
      .expect("sse recv timed out")
      .expect("sse stream ended early")
      .expect("sse stream error");
    collected.push_str(&String::from_utf8_lossy(&chunk));
  }
  assert!(collected.contains("one"), "missing 'one' in {collected:?}");
  assert!(collected.contains("two"), "missing 'two' in {collected:?}");
  assert!(collected.contains("three"), "missing 'three' in {collected:?}");

  // WebSocket: echo round-trip over a Ring virtual connection. A standard
  // tungstenite client speaks WebSocket straight over the `RingDuplex`; salvo
  // terminates the upgrade on the server side.
  let duplex = client.websocket("/ws").await.unwrap();
  let (mut wss, _resp) = tokio_tungstenite::client_async("ws://stream-e2e/ws", duplex)
    .await
    .expect("ws handshake failed");
  wss.send(TMsg::text("ping")).await.unwrap();
  let echoed = tokio::time::timeout(Duration::from_secs(5), wss.next())
    .await
    .expect("ws recv timed out")
    .expect("ws stream ended")
    .expect("ws error");
  assert_eq!(echoed.into_text().unwrap().as_str(), "ping");

  // ---- WebTransport over the same broker (separate app, direct listener) ----
  use std::sync::Arc;

  use impulse_server_kit::impulse_ring::{ImpulseRingListener, RingWebTransportHandler, serve_impulse_ring};

  const WT_APP: &str = "wt-e2e";

  // Echo handler: bounce one datagram, then echo a bidirectional stream.
  let handler: RingWebTransportHandler = Arc::new(|mut wt| {
    Box::pin(async move {
      if let Some(dg) = wt.recv_datagram().await {
        let _ = wt.send_datagram(dg.to_vec());
      }
      if let Some(mut stream) = wt.accept_bi().await {
        while let Some(chunk) = stream.recv().await {
          if stream.send(chunk.to_vec()).is_err() {
            break;
          }
        }
      }
    })
  });

  let service = salvo::Service::new(Router::new());
  let listener = ImpulseRingListener::new(WT_APP).on_webtransport(handler);
  let wt_server_task = tokio::spawn(async move {
    let _ = serve_impulse_ring(listener, service, std::future::pending::<()>()).await;
  });

  let wt_client = connect_with_retry(WT_APP).await;
  let mut wt = wt_client.webtransport("/wt").await.unwrap();

  // Datagram echo.
  wt.send_datagram(b"hello-dgram".to_vec()).unwrap();
  let dg = tokio::time::timeout(Duration::from_secs(5), wt.recv_datagram())
    .await
    .expect("datagram recv timed out")
    .expect("datagram stream ended");
  assert_eq!(&dg[..], b"hello-dgram");

  // Bidirectional stream echo.
  let mut stream = wt.open_bi().unwrap();
  stream.send(b"streamed".to_vec()).unwrap();
  let got = tokio::time::timeout(Duration::from_secs(5), stream.recv())
    .await
    .expect("stream recv timed out")
    .expect("stream ended early");
  assert_eq!(&got[..], b"streamed");

  // ---- teardown ----
  server_task.abort();
  wt_server_task.abort();
}
