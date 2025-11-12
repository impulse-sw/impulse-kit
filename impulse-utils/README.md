# impulse-utils

A bunch of fullstack utils.

## MessagePack support

With `impulse-utils`, you can:

- parse MsgPack at backend with `impulse_utils::requests::MsgPackParser` extension trait (`salvo::Request`)
- send MsgPack from backend with `msgpack!(data)` macro
- parse MsgPack at client with `impulse_utils::responses::MsgPackResponse` extension trait (`reqwest::Response`)
- send MsgPack from client with `impulse_utils::requests::MsgPackRequest` extension trait (`reqwest::RequestBuilder`)

Example:

```rust
/// server-side

#[handler]
async fn change_text(req: &mut Request) -> MResult<MsgPack<FooData>> {
  let mut foo = req.parse_msgpack::<FooData>().await?;  // parse
  foo.text = String::from("Gotcha!");
  msgpack!(foo)                                         // send
}

/// client-side

async fn change_text_at_backend(old_data: &FooData) -> CResult<FooData> {
  let new_data = reqwest::Client::new()
    .post("127.0.0.1:8080/change-text")
    .msgpack(old_data)?    // send
    .send()
    .await?
    .msgpack::<FooData>()  // parse
    .await?;

  Ok(new_data)
}
```

For MsgPack ser/de, `impulse-utils` uses `rmp_serde::to_vec` and `rmp_serde::from_slice` methods.

## SIMD JSON support by `sonic-rs`

`impulse-utils` provide:

- parse JSON at backend with `impulse_utils::requests::SimdJsonParser` extension trait (`salvo::Request`)
- send JSON from backend with `json!(data)` macro

> [!NOTE]
> To enable SIMD ser/de for JSON, compile your project with compiler option `-C target-cpu=native`.

## Unified responses and error handling at the backend

In Salvo and Server Kit, you're writing backend routes by one of three ways:

```rust
/// like this...
#[handler]
async fn route(req: &mut Request, res: &mut Response) { .. }

/// or like this
#[handler]
async fn route(req: &mut Request) -> impl salvo::Writer + use<> { .. }

/// or like this
#[handler]
async fn route(req: &mut Request) -> Result<MyType, MyError> { .. }
```

Only the last one provides you a `?` way to expose server errors.

`impulse-utils` provide unified type `MResult<T>` for specifying response type while using `ServerError` for errors.

### `impulse_utils::results::MResult`

`MResult` allows you to specify these response types:

- just OK 200 empty response: `MResult<OK>` and `ok!()` at the end of a route
- plain text: `MResult<Plain>` and `plain!(text)`
- HTML from string: `MResult<Html>` and `html!(text)`
- any file: `MResult<File>` and `file_upload!(pathbuf, filename)` (supports file caching with `ETag` and `Cache-Control`)
- JSON: `MResult<Json<T>>` and `json!(object)` (implemented by `sonic-rs` with SIMD support)
- MsgPack: `MResult<MsgPack<T>>` and `msgpack!(object)`

Resulting macros are used in `TRACE` log level to show you what you're sending as a response and from what function.

Examples:

```rust
#[handler]
async fn frontend(req: &Request) -> MResult<Html> {
  let filepath = get_filepath_from_dist("index.html").await?;
  let site = tokio::fs::read_to_string(&filepath)
    .await
    .map_err(|e| ServerError::from_private(e).with_500())?;
  html!(site)
}


#[handler]
async fn json_to_msgpack(req: &mut Request, depot: &mut Depot) -> MResult<MsgPack<HelloData>> {
  let hello = req.parse_json_simd::<HelloData>().await?;
  let app_name = depot.obtain::<Setup>()?.generic_values().app_name.as_str();
  msgpack!(HelloData { text: format!("From `{}` application: {}", app_name, hello.text) })
}
```

### `impulse_utils::errors::ServerError`

`ServerError` also implements `salvo::Writer`, but it does more. Internally it contains HTTP status code to return with (`500` by default), public error message and the list of private error messages. `ServerError` is designed to hide implementation details exposed with errors by separation into two types: public errors and private errors.

`ServerError` on response converts into simple JSON:

```json
{"err":"Public error text"}
```

This is how your clients will get error messages.

If `public_msg` field is set, your server will respond with value of this field. You can set `public_msg` several times, and client will get only one:

```rust
ServerError::from_public("Bad data inside SQL")
  .with_public("Database error")
  .with_public("Internal server error, call 911")
  .with_500()
  .bail()?;

/// Client will see:
/// 
/// ```json
/// {"err":"Internal server error, call 911"}
/// ```
/// 
/// Backend logs:
/// 
/// ```
/// 2025-05-11T00:24:49.474405Z ERROR Error: `500` status code
///   Error message: "Internal server error, call 911"
///     Caused by: Database error
///     Caused by: Bad data inside SQL
/// ```
```

Example with any error type that implements `std::error::Error` trait:

```rust
verify_sign(&challenge, sign, &public_key)
  .map_err(|e| ServerError::from_private(e).with_public("Invalid sign!").with_401())?;
```

Or `Option`:

```rust
let user_data = kv
  .get::<UserData>(&key)
  .await?
  .ok_or(ServerError::from_public("This user isn't exist!").with_404())?;
```

`ServerError` is used by `MResult<T>` type and supports both Salvo and Server Kit frameworks.

## `ExplicitServerWrite` trait

This trait is designed to use only `&mut Response` on write instead of a full signature of `salvo::Writer` trait:

```rust
async fn write(self, req: &mut Request, depot: &mut Depot, res: &mut Response) { .. }
```

You can use it, for example, when you're using any reference to the depot data and responding with this data:

```rust
// Usually you responds by `HTTP 200 OK`, but sometimes need to also include some data that referring to `depot`.
// Of course, you can just `.clone()` your value, but `ExplicitServerWrite` needs no allocations compared to cloning.

#[handler]
async fn maybe_token(depot: &mut Depot, res: &mut Response) -> MResult<OK> {
  let state_ref = depot.obtain::<State>()?;
  
  if state_ref.allow_send_token && let Some(token) = &state_ref.token {
    msgpack!(token).unwrap().explicit_write(res).await;
  }

  ok!()
}
```

This trait is implemented to all exposed by `impulse-utils` response types above except `file_upload!` (internally it needs `req: &mut Request` to compare `ETag` inside headers to allow file caching).

## `impulse_utils::results::CResult` and `impulse_utils::errors::ClientError`

`CResult<T>` is a `Result<T, ClientError>`. `ClientError` was designed to be simple client error which you can just `console.log` at client-side.

Example:

```rust
let resp = reqwest::Client::new()
  .post("127.0.0.1:8080/post-data")
  .json(&data)
  .send()
  .await
  .map_err(|e| ClientError::from(e).context("Can't send request!"))?
  .error_for_status()
  .map_err(|e| ClientError::from(e).context("Server error!"))?;
```

## Redirect or collect server errors

If you have several backends and communicate between them by `reqwest`, you may be considering of usage `redirect_server_error` method on `reqwest::Response` to directly redirect any error, especially `ErrorResponse` (public part of `ServerError`):

```rust
let resp = reqwest::Client::new()
  .post("127.0.0.1:8080/post-data")
  .json(&data)
  .send()
  .await
  .redirect_server_error() // if status >= 400, it will throw `Err::<ServerError>`; if no, returns `Ok::<reqwest::Response>`
  .await?
  .json::<MyResponse>()
  .await?;
```

But, if you're using `reqwest` on the client-side, you may want to use `collect_server_error` to collect `CResult` instead of `MResult`:

```rust
let resp = reqwest::Client::new()
  .post("127.0.0.1:8080/post-data")
  .json(&data)
  .send()
  .await
  .collect_server_error() // if status >= 400, it will throw `Err::<ClientError>`; if no, returns `Ok::<reqwest::Response>`
  .await?
  .json::<MyResponse>()
  .await?;
```

> [!NOTE]
> `redirect_server_error` method is available with `reqwest` and `mresult` features.
> 
> `collect_server_error` method is available with `reqwest` and `cresult` features.
