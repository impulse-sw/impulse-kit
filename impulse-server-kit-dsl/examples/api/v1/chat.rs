use crate::api::types::ChatData;
use impulse_server_kit::prelude::*;

pub fn chat_router() -> Router {
  Router::new()
    .push(Router::with_path("/chats").get(get_chats))
    .push(Router::with_path("/chat/{id}").get(get_chat_by_id))
    .push(Router::with_path("/chat/{id}/audio-request").post(post_chat_by_id_audio_request))
}

/// Get chats
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Chat"),
  parameters(
    ("X-Access" = String, Header, description = ""),
    ("X-Client" = String, Header, description = ""),
    ("X-Refresh" = String, Header, description = ""),
    ("chat_id" = i64, Query, description = "")
  ),
  responses((
    status_code = 200,
    description = "",
    body = Vec<ChatData>,
    content_type = ["application/json"]
  ))
)]
pub async fn get_chats(req: &mut Request) -> MResult<Json<Vec<ChatData>>> {
  let chat_id = req
    .query::<i64>("chat_id")
    .ok_or(ServerError::from_public("Can't find `chat_id` query parameter!").with_400())?;

  todo!();

  // json!(data)
}

/// Get chat by id
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Chat"),
  parameters(
    ("X-Access" = String, Header, description = ""),
    ("X-Client" = String, Header, description = ""),
    ("X-Refresh" = String, Header, description = ""),
    ("id" = u64, Path, description = "")
  ),
  responses((
    status_code = 200,
    description = "",
    body = ChatData,
    content_type = ["application/json"]
  ))
)]
pub async fn get_chat_by_id(req: &mut Request) -> MResult<Json<ChatData>> {
  let id = req
    .param::<u64>("id")
    .ok_or(ServerError::from_public("Can't find `id` parameter!").with_400())?;

  todo!();

  // json!(data)
}

/// Post chat by id audio request
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Chat"),
  request_body(content = Vec<u8>, content_type = "application/octet-stream", description = ""),
  parameters(
    ("X-Access" = String, Header, description = ""),
    ("X-Client" = String, Header, description = ""),
    ("X-Refresh" = String, Header, description = ""),
    ("id" = u64, Path, description = "")
  ),
  responses((
    status_code = 200,
    description = ""
  ))
)]
pub async fn post_chat_by_id_audio_request(req: &mut Request) -> MResult<OK> {
  let body = req
    .file("audio")
    .await
    .ok_or(ServerError::from_public("Can't find `audio` file!").with_400())?;
  let id = req
    .param::<u64>("id")
    .ok_or(ServerError::from_public("Can't find `id` parameter!").with_400())?;

  todo!();

  // ok!()
}
