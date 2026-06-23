//! A minimal Impulse Server Kit application that listens **only** over the Ring
//! shared-memory bus (no TCP), exposed under the application name `hello-ring`.
//!
//! Run the broker first, then this server, then the `ring-cli` example:
//!
//! ```sh
//! # 1) the Ring broker (from the impulse-ring repository)
//! cargo run -p impulsed
//!
//! # 2) this server
//! cargo run -p impulse-client-ring --example ring-server
//!
//! # 3) call it (see examples/cli.rs)
//! cargo run -p impulse-client-ring --example ring-cli -- get /hello
//! cargo run -p impulse-client-ring --example ring-cli -- post /echo --body 'hi there'
//! ```
//!
//! The configuration is built in-code (rather than read from YAML) to keep the
//! example self-contained.

use impulse_server_kit::prelude::*;
use impulse_server_kit::salvo::prelude::Json;
use impulse_server_kit::setup::ProtocolConfig;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize)]
struct Echo {
  said: String,
}

#[handler]
async fn hello() -> &'static str {
  "Hello from a shared-memory HTTP server!"
}

#[handler]
async fn echo(req: &mut Request, res: &mut Response) {
  let body = req.payload().await.map(|b| b.to_vec()).unwrap_or_default();
  let said = String::from_utf8_lossy(&body).into_owned();
  res.render(Json(Echo { said }));
}

#[tokio::main]
async fn main() {
  // Configure a single `impulse-ring` protocol under the name `hello-ring`.
  // The equivalent YAML is:
  //   protocols:
  //     - type: impulse-ring
  //       app_name: hello-ring
  let mut setup = Setup::default();
  setup.generic_values.app_name = "hello-ring".to_string();
  setup.generic_values.protocols = vec![ProtocolConfig::ImpulseRing {
    app_name: "hello-ring".to_string(),
    access_key: None,
    arena_size_kib: None,
  }];
  setup.generic_values.tracing_options.enable_io_logs = Some(true);
  setup.generic_values.tracing_options.io_log_level = Some("info".to_string());

  let state = load_generic_state(&setup, true).await.unwrap();

  let router = get_root_router_autoinject(&state, setup.clone())
    .push(Router::with_path("hello").get(hello))
    .push(Router::with_path("echo").post(echo));

  tracing::info!("Serving 'hello-ring' over the Ring bus. Ctrl+C to stop.");

  let (server, _handle) = start(state, &setup, router).await.unwrap();
  server.await;
}
