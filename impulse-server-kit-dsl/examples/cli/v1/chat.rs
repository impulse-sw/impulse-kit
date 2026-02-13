use crate::api::types::ChatData;
use impulse_utils::prelude::*;

pub async fn get_chats(chat_id: i64) -> CResult<Vec<ChatData>> {
  reqwest::Client::new()
    .get(endpoint("/chats"))
    .query(&[("chat_id", chat_id.to_string())])
    .send()
    .await
    .map_err(ClientError::from)?
    .collect_server_error()
    .await?
    .json()
    .await
    .map_err(ClientError::from)
}

pub async fn get_chat_by_id(id: u64) -> CResult<ChatData> {
  reqwest::Client::new()
    .get(endpoint(&format!("/chat/{id}")))
    .send()
    .await
    .map_err(ClientError::from)?
    .collect_server_error()
    .await?
    .json()
    .await
    .map_err(ClientError::from)
}

pub async fn post_chat_by_id_audio_request(id: u64, body: Vec<u8>) -> CResult<()> {
  use reqwest::multipart::{Form, Part};

  let form = Form::new().part("audio", Part::bytes(body));

  reqwest::Client::new()
    .post(endpoint(&format!("/chat/{id}/audio-request")))
    .multipart(form)
    .send()
    .await
    .map_err(ClientError::from)?
    .collect_server_error()
    .await?;
  Ok(())
}
