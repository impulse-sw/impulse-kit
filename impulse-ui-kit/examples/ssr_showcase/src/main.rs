//! SSR showcase server binary.

use impulse_server_kit::prelude::*;
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
  let setup = load_generic_config::<Setup>("server-example").await.unwrap();
  let state = load_generic_state(&setup, true).await.unwrap();
  let opts = LeptosOptions::from_generic_values(setup.generic_values());

  let router = get_root_router_autoinject(&state, setup.clone()).push(leptos_router(opts, || ssr_showcase::App));

  let (server, _handle) = start(state, &setup, router).await.unwrap();
  server.await
}
