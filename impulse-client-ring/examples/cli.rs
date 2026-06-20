//! A tiny `curl`-like CLI that calls *any* HTTP method on a Ring application
//! over shared memory.
//!
//! ```sh
//! # GET /hello on application `hello-ring`
//! cargo run -p impulse-client-ring --example ring-cli -- get /hello
//!
//! # POST a body
//! cargo run -p impulse-client-ring --example ring-cli -- post /echo --body 'hi there'
//!
//! # custom app name, headers and an access key
//! cargo run -p impulse-client-ring --example ring-cli -- \
//!   --app my-service --key s3cret \
//!   put /items/1 -H 'content-type: application/json' --body '{"name":"x"}'
//! ```
//!
//! Requires the `impulsed` broker to be running and a server (e.g. the
//! `ring-server` example) registered under the target application name.

use clap::Parser;
use impulse_client_ring::ImpulseRingClient;

#[derive(Parser)]
#[command(about = "Call any HTTP method on a Ring application over shared memory")]
struct Args {
  /// HTTP method (get, post, put, patch, delete, head, ...).
  method: String,
  /// Request target, e.g. `/hello` or `/items?page=2`.
  uri: String,

  /// Target application name registered on the Ring bus.
  #[arg(long, default_value = "hello-ring")]
  app: String,

  /// Access key, if the server requires one.
  #[arg(long)]
  key: Option<String>,

  /// Request body (sent as raw bytes).
  #[arg(long)]
  body: Option<String>,

  /// Extra header in `Name: Value` form. Repeatable.
  #[arg(short = 'H', long = "header")]
  headers: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();

  let method: http::Method = args.method.to_uppercase().parse()?;

  let mut client = ImpulseRingClient::connect(&args.app)?;
  if let Some(key) = args.key.as_deref() {
    client = client.with_key(key);
  }

  let mut builder = client.request(method, &args.uri);
  for h in &args.headers {
    let (name, value) = h.split_once(':').ok_or("headers must be `Name: Value`")?;
    builder = builder.header(name.trim(), value.trim());
  }
  if let Some(body) = args.body {
    builder = builder.body(body.into_bytes());
  }

  let resp = builder.send().await?;

  println!("< {}", resp.status());
  for (name, value) in resp.headers() {
    println!("< {}: {}", name, value.to_str().unwrap_or("<binary>"));
  }
  println!();

  let body = resp.bytes();
  match String::from_utf8(body) {
    Ok(text) => println!("{text}"),
    Err(e) => println!("<{} bytes of binary body>", e.into_bytes().len()),
  }

  Ok(())
}
