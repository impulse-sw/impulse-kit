use impulse_server_kit::salvo;
use impulse_server_kit::salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, ToSchema)]
pub struct HelloData {
  pub some_client_hello: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct AnswerData {
  pub some_server_answer: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct UserChangePasswordRequest {
  pub from_hash: String,
  pub to_hash: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ChatData {
  pub users: Vec<String>,
  pub messages: Vec<String>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct User {
  pub id: String,
  pub email: String,
  pub name: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct UserCreateRequest {
  pub email: String,
  pub name: String,
}
