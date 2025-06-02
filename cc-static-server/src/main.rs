#![deny(warnings, clippy::todo, clippy::unimplemented)]
#![feature(if_let_guard, let_chains)]

mod static_routes;

use cc_server_kit::prelude::*;
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
  let router = get_root_router(&state).push(static_routes::frontend_router());
  let (server, _handler) = start(state, &setup, router).await.unwrap();
  server.await
}
