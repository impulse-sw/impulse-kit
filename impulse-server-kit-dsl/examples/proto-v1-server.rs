#![allow(warnings)]

use impulse_server_kit::prelude::*;
use serde::Deserialize;

pub mod api;

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
  let setup = load_generic_config::<Setup>("proto-v1-server").await.unwrap();
  let state = load_generic_state(&setup, true).await.unwrap();
  let router = get_root_router_autoinject(&state, setup.clone())
    .push(api::v1::users::users_router())
    .push(api::v1::test::test_router())
    .push(api::v1::chat::chat_router());
  let (server, _handler) = start(state, &setup, router).await.unwrap();
  server.await
}
