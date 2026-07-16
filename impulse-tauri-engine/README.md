# impulse-tauri-engine

A reusable **offline-first engine** for Tauri apps. Native (non-wasm) only.

A Tauri webview can't reach the network from wasm, so the UI forwards every
request over IPC (`invoke("ik_http_request")`) to the native side, where this
engine handles it:

* **online** — forward to the real server (via a `Remote`) and let the app cache
  successful reads locally;
* **offline** — serve the request from a local store and, for writes, enqueue it
  for replay;
* **on reconnect** — `Engine::sync()` replays the queued writes oldest-first,
  reconciling any locally-minted provisional ids with the server's real ids.

The transport, the online/offline switch and the crash-safe write queue are
written once here; everything app-specific lives behind the `LocalBackend` trait.
Wire types come from `impulse-endpoint`, so the engine does **not** depend on the
leptos UI kit.

## Usage

Implement `LocalBackend` over your local store (e.g. a SQLite handle + the
signed-in identity), then build an `Engine`:

```rust
use impulse_tauri_engine::{Engine, LocalBackend};
use impulse_endpoint::{HttpRequest, HttpResponse};

struct MyBackend { /* db, identity, … */ }

impl LocalBackend for MyBackend {
    async fn serve_local(
        &self,
        req: &HttpRequest,
        provisional: &dyn Fn() -> i64,
    ) -> Result<(HttpResponse, Option<i64>), impulse_utils::prelude::ServerError> {
        // Match req.method + path against your local store. For an offline
        // create, mint a temporary id with `provisional()` and return it as the
        // second tuple element so the engine can reconcile it on sync.
        todo!()
    }
    // Optional hooks (all have defaults):
    //   cache_read, created_id, reconcile_id, rewrite_ids
}

// In the Tauri shell:
let engine = Engine::with_executor(MyBackend { /* … */ }, remote_base, queue_path)?;

// Wire it to the IPC command:
//   #[tauri::command]
//   async fn ik_http_request(engine: State<'_, App>, req: HttpRequest)
//       -> Result<HttpResponse, ()> { Ok(engine.handle(req).await) }

// From a connectivity probe, on a false→true transition:
engine.set_online(true);
engine.sync().await.ok();
```

### Reusing server routes offline

`LocalBackend::serve_local` can be backed by an `impulse_endpoint::Router` — the
**same** router the server mounts via `impulse-server-kit`'s `endpoint_router`.
That way a route's logic is written once and runs both on the server and offline
in the app.

## The pieces

* `Remote` — the online transport (injected, so tests use a fake server).
* `ExecutorRemote` — the production `Remote`: runs requests natively via reqwest
  (behind the `executor` feature). Also exposed as `executor::execute` for a
  standalone connectivity probe.
* `Queue` — the persistent, crash-safe FIFO journal of offline writes.
* `Engine<R, L>` — the orchestration: `handle`, `sync`, the online flag.
* `LocalBackend` — the app's offline behaviour (serve / cache / reconcile).

## Features

* `executor` *(on by default)* — provides `ExecutorRemote` and `executor::execute`
  (pulls `reqwest`). Turn it off for headless tests that inject a fake `Remote`.
