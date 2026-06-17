//! SSR + hydration showcase server binary.
//!
//! Wires `impulse-server-kit` with the Leptos SSR adapter, mounts the
//! `#[server]` function router under `LeptosOptions::server_fn_prefix`, and
//! serves both the streaming HTML and the wasm bundle that hydrates the page
//! on the client.

#![cfg(feature = "ssr")]

use impulse_server_kit::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

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

/// Example telemetry sink: logs every event received from the client.
///
/// A real application would persist events (anonymous or identified) to a
/// database or forward them to a message broker; here we simply trace them so
/// the receiving side is visible in the server logs.
struct ShowcaseTelemetrySink;

#[salvo::async_trait]
impl TelemetrySink for ShowcaseTelemetrySink {
  async fn record(&self, event: &TelemetryEvent, ctx: &TelemetryRequestCtx) {
    tracing::info!(
      kind = ?event.kind,
      message = event.message.as_deref().unwrap_or_default(),
      session = event.session_id.as_deref().unwrap_or_default(),
      user = event.user_id.as_deref().unwrap_or_default(),
      path = event.path.as_deref().unwrap_or_default(),
      remote = ctx.remote_addr.as_deref().unwrap_or_default(),
      "received client telemetry"
    );
  }
}

#[tokio::main]
async fn main() {
  let setup = load_generic_config::<Setup>("server-example").await.unwrap();
  let state = load_generic_state(&setup, true).await.unwrap();

  let mut opts = LeptosOptions::from_generic_values(setup.generic_values());
  opts.include_hydration_script = true;
  opts.stream_mode = SsrStreamMode::InOrder;

  let server_fn_prefix = opts.server_fn_prefix.clone();

  let router = get_root_router_autoinject(&state, setup.clone())
    .push(server_fn_router(server_fn_prefix))
    // Receives telemetry posted by the client (see the monitors in `App`).
    .push(telemetry_router("api/telemetry", Arc::new(ShowcaseTelemetrySink)))
    .push(leptos_router(opts, || ssr_showcase::App));

  let (server, _handle) = start(state, &setup, router).await.unwrap();
  server.await
}
