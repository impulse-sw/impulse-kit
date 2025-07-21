use impulse_server_kit::prelude::*;
use std::collections::HashMap;

type ComplexAliasType = HashMap<String, u32>;

pub fn test_router() -> Router {
  Router::new()
    .push(Router::with_path("/test").get(get_test))
    .push(Router::with_path("/audio").post(post_audio))
}

/// Get test
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Test"),
  parameters(
    ("X-Access" = String, Header, description = ""),
    ("X-Client" = String, Header, description = ""),
    ("X-Refresh" = String, Header, description = "")
  ),
  responses((
    status_code = 200,
    description = "",
    headers(("X-Sign" = String, description = ""))
  ))
)]
pub async fn get_test(req: &mut Request, res: &mut Response) -> MResult<OK> {
  use impulse_server_kit::salvo::http::cookie::CookieBuilder;

  todo!();

  // res.add_cookie(CookieBuilder::new("X-Sign", x_sign).build());

  // ok!()
}

/// Post audio
#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Test"),
  request_body(content = Vec<u8>, content_type = "multipart/form-data", description = ""),
  parameters(("gitlab_session" = String, Cookie, description = "")),
  responses((
    status_code = 200,
    description = "",
    body = ComplexAliasType,
    content_type = ["application/msgpack"]
  ))
)]
pub async fn post_audio(req: &mut Request) -> MResult<MsgPack<ComplexAliasType>> {
  let audio = req
    .form::<Vec<u8>>("audio")
    .await
    .ok_or(ServerError::from_public("Can't find `audio` form key!").with_400())?;

  todo!();

  // msgpack!(data)
}
