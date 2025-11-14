use impulse_server_kit::prelude::*;
use crate::api::types::AnswerData;
use crate::api::types::HelloData;
use crate::api::types::User;
use crate::api::types::UserChangePasswordRequest as UserChangePassReq;

pub fn users_router() -> Router {
  Router::new()
    .push(Router::with_path("/sign-in").post(post_sign_in))
    .push(Router::with_path("/change-password").patch(patch_change_password))
    .push(Router::with_path("/logout").post(post_logout))
    .push(Router::with_path("/account").delete(delete_account))
}

/// Post sign in
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Users"),
  request_body(content = HelloData, content_type = "application/json", description = ""),
  parameters(
    ("X-Sign" = String, Header, description = ""),
    ("user_id" = i64, Query, description = "")
  ),
  responses((
    status_code = 200,
    description = "",
    body = AnswerData,
    content_type = ["application/json"]
  ))
)]
pub async fn post_sign_in(req: &mut Request) -> MResult<Json<AnswerData>> {
  let x_sign = req
    .header::<String>("X-Sign")
    .ok_or(ServerError::from_public("Can't find `X-Sign` header!").with_400())?;
  let body = req
    .parse_json_simd::<HelloData>()
    .await
    .map_err(|e| ServerError::from_private(e).with_public("Can't find JSON with `HelloData` type!").with_400())?;
  let user_id = req
    .query::<i64>("user_id")
    .ok_or(ServerError::from_public("Can't find `user_id` query parameter!").with_400())?;

  todo!();

  // json!(data)
}

/// Patch change password
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Users"),
  request_body(content = UserChangePassReq, content_type = "application/msgpack", description = ""),
  parameters(
    ("X-Access" = String, Header, description = ""),
    ("X-Client" = String, Header, description = ""),
    ("X-Refresh" = String, Header, description = "")
  ),
  responses((
    status_code = 200,
    description = ""
  ))
)]
pub async fn patch_change_password(req: &mut Request) -> MResult<OK> {
  let x_access = req
    .header::<String>("X-Access")
    .ok_or(ServerError::from_public("Can't find `X-Access` header!").with_400())?;
  let x_client = req
    .header::<String>("X-Client")
    .ok_or(ServerError::from_public("Can't find `X-Client` header!").with_400())?;
  let x_refresh = req
    .header::<String>("X-Refresh")
    .ok_or(ServerError::from_public("Can't find `X-Refresh` header!").with_400())?;
  let body = req
    .parse_msgpack::<UserChangePassReq>()
    .await
    .map_err(|e| ServerError::from_private(e).with_public("Can't find MsgPack with `UserChangePassReq` type!").with_400())?;

  todo!();

  // ok!()
}

/// Post logout
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Users"),
  parameters(
    ("X-Access" = String, Header, description = ""),
    ("X-Client" = String, Header, description = ""),
    ("X-Refresh" = String, Header, description = "")
  ),
  responses((
    status_code = 200,
    description = ""
  ))
)]
pub async fn post_logout(req: &mut Request) -> MResult<OK> {
  let x_access = req
    .header::<String>("X-Access")
    .ok_or(ServerError::from_public("Can't find `X-Access` header!").with_400())?;
  let x_client = req
    .header::<String>("X-Client")
    .ok_or(ServerError::from_public("Can't find `X-Client` header!").with_400())?;
  let x_refresh = req
    .header::<String>("X-Refresh")
    .ok_or(ServerError::from_public("Can't find `X-Refresh` header!").with_400())?;

  todo!();

  // ok!()
}

/// Delete account
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Users"),
  request_body(content = User, content_type = "application/json", description = ""),
  parameters(
    ("X-Access" = String, Header, description = ""),
    ("X-Client" = String, Header, description = ""),
    ("X-Refresh" = String, Header, description = "")
  ),
  responses((
    status_code = 200,
    description = ""
  ))
)]
pub async fn delete_account(req: &mut Request) -> MResult<OK> {
  let x_access = req
    .header::<String>("X-Access")
    .ok_or(ServerError::from_public("Can't find `X-Access` header!").with_400())?;
  let x_client = req
    .header::<String>("X-Client")
    .ok_or(ServerError::from_public("Can't find `X-Client` header!").with_400())?;
  let x_refresh = req
    .header::<String>("X-Refresh")
    .ok_or(ServerError::from_public("Can't find `X-Refresh` header!").with_400())?;
  let body = req
    .parse_json_simd::<User>()
    .await
    .map_err(|e| ServerError::from_private(e).with_public("Can't find JSON with `User` type!").with_400())?;

  todo!();

  // ok!()
}
