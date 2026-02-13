use impulse_utils::prelude::*;
use std::collections::HashMap;

type ComplexAliasType = HashMap<String, u32>;

pub async fn get_test() -> CResult<()> {
  reqwest::Client::new()
    .get(endpoint("/test"))
    .send()
    .await
    .map_err(ClientError::from)?
    .collect_server_error()
    .await?;
  Ok(())
}

pub async fn post_audio(audio: Vec<u8>) -> CResult<ComplexAliasType> {
  use reqwest::multipart::{Form, Part};

  let form = Form::new().part("audio", Part::bytes(audio));

  reqwest::Client::new()
    .post(endpoint("/audio"))
    .multipart(form)
    .send()
    .await
    .map_err(ClientError::from)?
    .collect_server_error()
    .await?
    .msgpack()
    .await
}
