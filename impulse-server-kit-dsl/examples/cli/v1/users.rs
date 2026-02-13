use crate::api::types::AnswerData;
use crate::api::types::HelloData;
use crate::api::types::User;
use crate::api::types::UserChangePasswordRequest as UserChangePassReq;
use impulse_utils::prelude::*;

pub async fn post_sign_in(x_sign: &str, user_id: i64, body: &HelloData) -> CResult<AnswerData> {
  reqwest::Client::new()
    .post(endpoint("/sign-in"))
    .header("X-Sign", x_sign)
    .query(&[("user_id", user_id.to_string())])
    .json(body)
    .send()
    .await
    .map_err(ClientError::from)?
    .collect_server_error()
    .await?
    .json()
    .await
    .map_err(ClientError::from)
}

pub async fn patch_change_password(
  x_access: &str,
  x_client: &str,
  x_refresh: &str,
  body: &UserChangePassReq,
) -> CResult<()> {
  reqwest::Client::new()
    .patch(endpoint("/change-password"))
    .header("X-Access", x_access)
    .header("X-Client", x_client)
    .header("X-Refresh", x_refresh)
    .msgpack(body)?
    .send()
    .await
    .map_err(ClientError::from)?
    .collect_server_error()
    .await?;
  Ok(())
}

pub async fn post_logout(x_access: &str, x_client: &str, x_refresh: &str) -> CResult<()> {
  reqwest::Client::new()
    .post(endpoint("/logout"))
    .header("X-Access", x_access)
    .header("X-Client", x_client)
    .header("X-Refresh", x_refresh)
    .send()
    .await
    .map_err(ClientError::from)?
    .collect_server_error()
    .await?;
  Ok(())
}

pub async fn delete_account(x_access: &str, x_client: &str, x_refresh: &str, body: &User) -> CResult<()> {
  reqwest::Client::new()
    .delete(endpoint("/account"))
    .header("X-Access", x_access)
    .header("X-Client", x_client)
    .header("X-Refresh", x_refresh)
    .json(body)
    .send()
    .await
    .map_err(ClientError::from)?
    .collect_server_error()
    .await?;
  Ok(())
}
