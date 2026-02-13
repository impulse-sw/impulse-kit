# Server Kit DSL

Simple API description parser and tranlsator for Server Kit.

SK DSL generates prototypes folder `v{api_ver}` with endpoints grouped by tags files `{tag_name}.rs`:

```
├──v1
│  ├──mod.rs
│  ├──users.rs
│  ├──chats.rs
│  └──files.rs
└──v2
   ├──mod.rs
   ├──users.rs
   ├──chats.rs
   └──files.rs
```

## Installation from source

To install Server Kit DSL client from source, clone this repository and run `cargo install`:

```bash
git clone https://github.com/impulse-sw/impulse-kit.git
cd impulse-kit
cargo install --path . --bin skdsl
```

## Usage

```
Usage: skdsl [OPTIONS] --input <FILE> --output <FOLDER>

Options:
  -i, --input <FILE>       Input DSL file
  -o, --output <FOLDER>    Output folder
  -v, --version <VERSION>  API version (optional)
  -r, --regenerate         Don't bump the version and rewrite all generated files (destructive)
  -R, --cli-rs <FOLDER>    Generate Rust API client into FOLDER
  -J, --cli-js <FOLDER>    Generate JS API client into FOLDER
  -h, --help               Print help
```

Examples:

```bash
# will create `api/v1` folder with prototypes
skdsl -i proto-v1.txt -o api

# manually bump API to `v3` and generates clients for Rust and JS
skdsl -i proto-v2.md -o api -v v3 --cli-rs cli --cli-js cli-js

# regenerate `v3` from scratch
skdsl -i proto-v3.md -o api -v v3 -r
```

> [!NOTE]
> `--regenerate` is a destructive operation. If you start to implement actual logic on any endpoints, this option will clear all your changes!

## DSL specification

SK DSL is a custom but simple domain-specific language.

First of all, `skdsl` client will parse any text input file. It searches only lines which started by:

- `type`: type definition - either usage or alias
- `req`: complex requirement
- `api`: tag or endpoint definition

See [`proto-v1.md`](./examples/proto-v1.md) example.

### Types definition

`skdsl` automatically imports used types on every tag module. You just have to define what types you're using in API like this:

```
# will become `use crate::types::MyType;`
type MyType crate::types::MyType

# will become `use crate::types::ComplexRequest as MyType;`
type MyType crate::types::ComplexRequest

# will become `type MyType = HashMap<String, u32>;`
type MyType HashMap<String, u32>
```

> [!NOTE]
> Types you use in endpoint's OpenAPI descriptions should implement `salvo::oapi::ToSchema` trait (Salvo requirement) and `serde::{Deserialize, Serialize}`.
> 
> This done for standard library types and generally is easy enough:
> 
> ```rust
> use impulse_server_kit::salvo;
> use salvo::oapi::ToSchema;
> use serde::{Deserialize, Serialize};
> 
> #[derive(Deserialize, Serialize, ToSchema)]
> pub struct YourOwnType {
>   pub my_field1: String,
>   pub my_field2: u64,
> }
> ```

### Requirements types

`skdsl` provides six types of incoming requirements:

- HTTP method with path and optional params: `<get, post, put, patch or delete>/<actual http path>/{u64/id}/{**rest_path}`
- request header: `h/<type>/<name>`
- request cookie: `c/<key>`
- request path query: `q/<type>/<key>`
- request body: `b/<json or msgpack>/<type>` or `b/file/<form key>`
- request form key: `f/<type>/<key>`

Also, `skdsl` provides three types of outgoing requirements:

- response body: `ok`, `b/<plain, html or file>` or `b/<json or msgpack>/<type>`
- response header: `h/<type>/<name>`
- response cookie: `c/<key>`

These types of requirements are using to describe API endpoint contracts.

> [!NOTE]
> Request method and path can be specified only once.

> [!NOTE]
> You cannot use the body with the form keys.

### API tag and endpoints

To describe some first endpoints of your API, specify API tag:

```
api tag <tag_name>
```

All endpoint listed below will be tagged by `Tag name` and stored at `v{api_ver}/tag_name.rs`.

After this, you can finally specify all your enpoints: `api <incoming...> -> <outgoing...>`.

Example:

```
api tag chat
api get/chats q/i64/chat_id                         -> b/json/Vec<ChatData>
api get/chat/{u64/id}                               -> b/json/ChatData
api post/chat/{str/host}/audio-request b/file/audio -> ok
```

This DSL code will be generated as follows:

```rust
use impulse_server_kit::prelude::*;
use crate::api::types::ChatData;

#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Chat"),
  parameters(("chat_id" = i64, Query, description = "")),
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

  json!(data)
}

#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Chat"),
  parameters(("id" = u64, Path, description = "")),
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

  json!(data)
}

#[instrument(skip_all, fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
#[endpoint(
  tags("Chat"),
  request_body(content = Vec<u8>, content_type = "application/octet-stream", description = ""),
  parameters(("id" = u64, Path, description = "")),
  responses((
    status_code = 200,
    description = ""
  ))
)]
pub async fn post_chat_by_id_audio_request(req: &mut Request) -> MResult<OK> {
  let id = req
    .param::<u64>("id")
    .ok_or(ServerError::from_public("Can't find `id` parameter!").with_400())?;
  let body = req
    .file("audio")
    .await
    .ok_or(ServerError::from_public("Can't find `audio` file!").with_400())?;

  todo!();

  ok!()
}

pub fn chat_router() -> Router {
  Router::new()
    .push(Router::with_path("/chats").get(get_chats))
    .push(Router::with_path("/chat/{id}").get(get_chat_by_id))
    .push(Router::with_path("/chat/{id}/audio-request").post(post_chat_by_id_audio_request))
}
```

#### Hidden APIs

You can specify API endpoint as `api/hidden ...` to exclude it from OpenAPI spec.

### Complex requirements

Complex requirement is a set of incoming and/or outgoing requirements: `req <requirement_name> <incoming...> [-> <outgoing...>]`.

You can use complex requirements as common requirements at several endpoints or even all API tag:

```
api tag chat req/tokens
api get/chats q/i64/chat_id                       -> b/json/Vec<ChatData>
api get/chat/{u64/id}                             -> b/json/ChatData
api post/chat/{u64/id}/audio-request b/file/audio -> ok

or

req master h/str/C3A-Access h/str/C3A-Refresh h/str/C3A-Client -> h/str/C3A-Sign
req/hidden slave c/gitlab_session

api tag test
api req/master get/test                   -> ok c/C3A-Sign
api req/slave  post/audio f/Vec<u8>/audio -> b/msgpack/ComplexAliasType
```

You should specify complex requirements usage *after tag name* on tag definition, or *before all special requirements* on endpoint definition.

> [!NOTE]
> You cannot specify body, forms and path params at complex requirements!

#### Hidden requirements

If you have no need to use some requirements and only want to define them at OpenAPI spec, you can specify complex requirement as `req/hidden`.

## OpenAPI specification

You can generate single OpenAPI router with Server Kit just by configuration:

```yaml
oapi_frontend_type: Scalar  # or SwaggerUI
oapi_name: My API
oapi_ver: 0.1.0
oapi_api_addr: /api         # path to OpenAPI specification frontend
```

## Breaking changes

For non-breaking changes, you should only define new tags, types or endpoints.

You can also modify current endpoints to *remove* incoming requirements and *add* outgoing requirements. All other changes will be marked as breaking.

> [!NOTE]
> New types and endpoints for existing API tag will be added *automatically*, router will be updated too. You should only care about endpoint requirements, because `skdsl` will never rewrite any edited content until you forced him to with `--regenerate` option.
