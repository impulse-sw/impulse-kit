//! Unified client-side request execution with a single API across two
//! compile-time backends, plus a typed ergonomics layer shared by every app.
//!
//! * **default** (browser wasm, or native): a request goes out directly —
//!   `reqwest` with the browser fetch backend on wasm, native TLS off-wasm.
//! * **`cfg(tauri)`** (the wasm frontend bundled into a Tauri webview): the
//!   webview can't reach arbitrary hosts, so every request is serialised and
//!   forwarded over Tauri IPC (`invoke("ik_http_request")`) to the native engine,
//!   which runs the real transport via [`executor`] and hands back a
//!   [`HttpResponse`]. The switch is a Cargo feature — recompile and it just works.
//!
//! The same [`RequestBuilder`] / [`HttpResponse`] surface is used in both modes,
//! so call sites (an app's `requests.rs`) never mention which transport is live.
//!
//! ## Typed ergonomics
//!
//! Instead of every app re-writing "send with credentials, check the status,
//! decode JSON or surface the server's `{ "err": … }` message", use
//! [`RequestBuilder::recv`] / [`RequestBuilder::recv_ok`]:
//!
//! ```rust,ignore
//! use impulse_tauri_client as client;
//! let doc: MyDto = client::get("/api/v1/documents/1").credentials().recv().await?;
//! client::delete("/api/v1/documents/1").credentials().recv_ok().await?;
//! ```
//!
//! ## Auth is layered by the app, not baked in
//!
//! This crate is deliberately auth-agnostic to avoid a dependency cycle with
//! `authnz`. Install a [request interceptor][set_request_interceptor] once at
//! startup to attach credentials (a bearer token, cookies, …). In `cfg(tauri)`
//! mode the interceptor lives on the **engine** side (the executor), where the
//! token is held; in the default mode it runs in the browser.

#![deny(warnings, clippy::todo, clippy::unimplemented, missing_docs)]

use impulse_utils::prelude::{CResult, ClientError};
use serde::{Deserialize, Serialize};

// The HTTP wire types (`Method`, `HttpRequest`, `HttpResponse`) live in the
// neutral `impulse-endpoint` crate, shared with the server adapter and the Tauri
// engine. Re-exported here (and, in turn, from `impulse_client_kit::client`).
pub use impulse_endpoint::{HttpRequest, HttpResponse, Method};

/// Fluent builder mirroring `reqwest`'s ergonomics. Body-encoding errors are
/// captured and surfaced from [`send`](RequestBuilder::send).
pub struct RequestBuilder {
  req: HttpRequest,
  err: Option<ClientError>,
}

impl RequestBuilder {
  fn new(method: Method, url: impl Into<String>) -> Self {
    Self {
      req: HttpRequest {
        method,
        url: url.into(),
        headers: Vec::new(),
        body: None,
        credentials: false,
      },
      err: None,
    }
  }

  /// Adds a header.
  pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
    self.req.headers.push((name.into(), value.into()));
    self
  }

  /// Requests that ambient credentials (cookies / stored token) ride along.
  pub fn credentials(mut self) -> Self {
    self.req.credentials = true;
    self
  }

  /// Sets a raw body.
  pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
    self.req.body = Some(body.into());
    self
  }

  /// Serialises `value` as JSON and sets `Content-Type: application/json`.
  pub fn json<T: Serialize + ?Sized>(mut self, value: &T) -> Self {
    match serde_json::to_vec(value) {
      Ok(bytes) => {
        self
          .req
          .headers
          .push(("Content-Type".into(), "application/json".into()));
        self.req.body = Some(bytes);
      }
      Err(e) => self.err = Some(ClientError::from(e)),
    }
    self
  }

  /// Serialises `value` as MessagePack and sets `Content-Type: application/msgpack`.
  pub fn msgpack<T: Serialize + ?Sized>(mut self, value: &T) -> Self {
    match rmp_serde::to_vec(value) {
      Ok(bytes) => {
        self
          .req
          .headers
          .push(("Content-Type".into(), "application/msgpack".into()));
        self.req.body = Some(bytes);
      }
      Err(e) => self.err = Some(ClientError::from(e)),
    }
    self
  }

  /// Runs the request through the active backend, applying the installed
  /// interceptor first.
  pub async fn send(self) -> CResult<HttpResponse> {
    if let Some(e) = self.err {
      return Err(e);
    }
    let mut req = self.req;
    apply_interceptor(&mut req);
    dispatch(req).await
  }

  /// Sends the request and decodes a 2xx JSON body into `T`. On a non-2xx status
  /// the server's public `{ "err": … }` message is surfaced as the error (falling
  /// back to `HTTP <status>`). This is the shared replacement for every app's
  /// hand-written "send, check status, decode-or-extract-error" helper.
  pub async fn recv<T: serde::de::DeserializeOwned>(self) -> CResult<T> {
    let resp = self.send().await?;
    if !resp.is_success() {
      return Err(server_error(&resp));
    }
    resp.json::<T>()
  }

  /// Like [`recv`](RequestBuilder::recv) for an endpoint that answers an empty
  /// 2xx: succeeds with `()` or surfaces the server's error message.
  pub async fn recv_ok(self) -> CResult<()> {
    let resp = self.send().await?;
    if !resp.is_success() {
      return Err(server_error(&resp));
    }
    Ok(())
  }
}

/// Extracts the server's public error message from a non-2xx response body
/// (`{ "err": … }`), falling back to a status-based message.
fn server_error(resp: &HttpResponse) -> ClientError {
  #[derive(Deserialize)]
  struct ErrBody {
    err: String,
  }
  match resp.json::<ErrBody>() {
    Ok(body) => ClientError::from_str(body.err),
    Err(_) => ClientError::from_str(format!("Request failed (HTTP {})", resp.status())),
  }
}

/// Starts a request with an explicit verb.
pub fn request(method: Method, url: impl Into<String>) -> RequestBuilder {
  RequestBuilder::new(method, url)
}

/// `GET` request builder.
pub fn get(url: impl Into<String>) -> RequestBuilder {
  RequestBuilder::new(Method::Get, url)
}
/// `POST` request builder.
pub fn post(url: impl Into<String>) -> RequestBuilder {
  RequestBuilder::new(Method::Post, url)
}
/// `PUT` request builder.
pub fn put(url: impl Into<String>) -> RequestBuilder {
  RequestBuilder::new(Method::Put, url)
}
/// `PATCH` request builder.
pub fn patch(url: impl Into<String>) -> RequestBuilder {
  RequestBuilder::new(Method::Patch, url)
}
/// `DELETE` request builder.
pub fn delete(url: impl Into<String>) -> RequestBuilder {
  RequestBuilder::new(Method::Delete, url)
}
/// `HEAD` request builder.
pub fn head(url: impl Into<String>) -> RequestBuilder {
  RequestBuilder::new(Method::Head, url)
}

// ---------- Request interceptor (auth is layered here) ----------

type Interceptor = Box<dyn Fn(&mut HttpRequest) + Send + Sync + 'static>;
static INTERCEPTOR: std::sync::OnceLock<Interceptor> = std::sync::OnceLock::new();

/// Installs a global hook run against every outgoing [`HttpRequest`] just before
/// dispatch — the place to attach an `Authorization` header, flip credentials, or
/// add tracing headers. Install-once; a second call is ignored.
pub fn set_request_interceptor<F>(f: F) -> Result<(), &'static str>
where
  F: Fn(&mut HttpRequest) + Send + Sync + 'static,
{
  INTERCEPTOR
    .set(Box::new(f))
    .map_err(|_| "impulse-tauri-client: request interceptor already installed")
}

fn apply_interceptor(req: &mut HttpRequest) {
  if let Some(f) = INTERCEPTOR.get() {
    f(req);
  }
}

// ---------- Backends ----------

/// Default backend: run the request directly with `reqwest` (browser fetch on
/// wasm, native TLS off-wasm).
#[cfg(not(tauri))]
async fn dispatch(req: HttpRequest) -> CResult<HttpResponse> {
  let client = reqwest::Client::new();
  let mut rb = client.request(req.method.into(), &req.url);
  for (k, v) in &req.headers {
    rb = rb.header(k, v);
  }
  if let Some(body) = req.body {
    rb = rb.body(body);
  }
  // On wasm, send cookies with cross-fetch when asked. Off-wasm reqwest carries
  // its own cookie store / the engine attaches auth, so this is a no-op there.
  #[cfg(target_arch = "wasm32")]
  if req.credentials {
    rb = rb.fetch_credentials_include();
  }
  let resp = rb.send().await.map_err(ClientError::from)?;
  collect(resp).await
}

#[cfg(not(tauri))]
async fn collect(resp: reqwest::Response) -> CResult<HttpResponse> {
  let status = resp.status().as_u16();
  let headers = resp
    .headers()
    .iter()
    .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or_default().to_string()))
    .collect();
  let body = resp.bytes().await.map_err(ClientError::from)?.to_vec();
  Ok(HttpResponse { status, headers, body })
}

/// Tauri backend: forward the request to the native engine over IPC.
#[cfg(tauri)]
async fn dispatch(req: HttpRequest) -> CResult<HttpResponse> {
  ipc::request(req).await
}

/// The wasm ⇄ Tauri IPC bridge. Calls the engine's `ik_http_request` command via
/// the global `window.__TAURI__.core.invoke` (requires `withGlobalTauri` in the
/// app's `tauri.conf.json`).
#[cfg(tauri)]
mod ipc {
  use super::{ClientError, HttpRequest, HttpResponse};
  use impulse_utils::prelude::CResult;
  use serde::Serialize;
  use wasm_bindgen::JsValue;
  use wasm_bindgen::prelude::wasm_bindgen;

  #[wasm_bindgen]
  extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
  }

  #[derive(Serialize)]
  struct Args {
    req: HttpRequest,
  }

  pub(super) async fn request(req: HttpRequest) -> CResult<HttpResponse> {
    let args = serde_wasm_bindgen::to_value(&Args { req })
      .map_err(|e| ClientError::from_str(format!("IPC encode failed: {e:?}")))?;
    let value = invoke("ik_http_request", args)
      .await
      .map_err(|e| ClientError::from_str(format!("IPC request failed: {e:?}")))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| ClientError::from_str(format!("IPC decode failed: {e:?}")))
  }
}

// ---------- Native engine executor ----------

/// The native transport the Tauri engine registers as an IPC command handler.
///
/// The engine wraps [`execute`] in a Tauri command named `ik_http_request` and
/// installs a [request interceptor][super::set_request_interceptor] that attaches
/// the stored authnz token.
#[cfg(all(feature = "tauri-executor", not(any(target_arch = "wasm32", target_arch = "wasm64"))))]
pub mod executor {
  use super::{HttpRequest, HttpResponse, apply_interceptor, collect};
  use impulse_utils::prelude::{CResult, ClientError};

  /// Executes a request natively (reqwest) after applying the installed
  /// interceptor, returning a serialisable [`HttpResponse`] to hand back over IPC.
  pub async fn execute(mut req: HttpRequest) -> CResult<HttpResponse> {
    apply_interceptor(&mut req);
    let client = reqwest::Client::new();
    let mut rb = client.request(req.method.into(), &req.url);
    for (k, v) in &req.headers {
      rb = rb.header(k, v);
    }
    if let Some(body) = req.body {
      rb = rb.body(body);
    }
    let resp = rb.send().await.map_err(ClientError::from)?;
    collect(resp).await
  }
}
