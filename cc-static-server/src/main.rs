#![deny(warnings, clippy::todo, clippy::unimplemented)]

use cc_server_kit::prelude::*;
use cc_static_server::frontend_router;
use serde::Deserialize;

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

#[tokio::main]
async fn main() {
  let setup = load_generic_config::<Setup>("static-server").await.unwrap();
  let state = load_generic_state(&setup, true).await.unwrap();

  tracing::info!("Static Server (v{})", env!("CARGO_PKG_VERSION"));

  let router = get_root_router(&state).push(frontend_router().unwrap());
  let (server, _handler) = start(state, &setup, router).await.unwrap();
  server.await
}
