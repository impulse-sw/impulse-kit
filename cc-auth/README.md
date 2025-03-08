# cc-auth

Simple backend authorization system.

## Simple usage example

```rust
use bb8_redis::{RedisConnectionManager, bb8::Pool};
use cc_auth::{ApiToken, check_token};
use cc_utils::prelude::MResult;

pub async fn authorized_action(
  cacher: &Pool<RedisConnectionManager>,
  token: ApiToken,
) -> MResult<()> {
  let user_id = check_token(&token, cacher).await?;
  Ok(())
}
```
