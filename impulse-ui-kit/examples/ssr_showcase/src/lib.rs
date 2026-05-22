//! Shared application code for the SSR showcase.
//!
//! Compiles in three modes:
//! - `ssr`: server-side rendering (used by `bin/ssr-showcase`).
//! - `hydrate`: client-side hydration of SSR markup (built as wasm32).
//! - neither: stub used by `cargo check` without any feature picked.
//!
//! Provides:
//! - `App`: root component shared between server and client.
//! - `greet` / `slow_data`: `#[server]` functions that demonstrate the
//!   `server_fn`-via-Salvo bridge.
//! - `hydrate` (only under `feature = "hydrate"`): exported via `wasm-bindgen`
//!   so the SSR-emitted bootstrap script can drive client-side hydration.

#![allow(dead_code)]

use leptos::prelude::*;
use leptos_meta::*;
use serde::{Deserialize, Serialize};

/// Greeting payload returned by the [`greet`] server function.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Greeting {
  /// Greeting text rendered into the page.
  pub message: String,
  /// Number of words in [`Self::message`].
  pub words: u32,
}

/// Echo a greeting to the user.
#[server(input = server_fn::codec::MsgPack, output = server_fn::codec::MsgPack)]
pub async fn greet(name: String) -> Result<Greeting, server_fn::ServerFnError> {
  let message = format!("Hello from the server, {name}!");
  let words = message.split_whitespace().count() as u32;
  Ok(Greeting { message, words })
}

/// Synthetic slow query, used to demonstrate `<Suspense>` streaming.
#[server(input = server_fn::codec::MsgPack, output = server_fn::codec::MsgPack)]
pub async fn slow_data() -> Result<Vec<String>, server_fn::ServerFnError> {
  tokio::time::sleep(std::time::Duration::from_millis(250)).await;
  Ok(vec![
    "Streamed via <Suspense>".into(),
    "Resolved on the server".into(),
    "Hydrated on the client".into(),
  ])
}

/// Root application component shared by SSR and hydration paths.
#[component]
pub fn App() -> impl IntoView {
  provide_meta_context();

  let greeting = Resource::new(|| (), |_| async move { greet("Impulse".to_string()).await });
  let slow = Resource::new(|| (), |_| async move { slow_data().await });

  view! {
    // Page-level overrides only. Description, canonical, OG and Twitter
    // defaults are emitted by impulse-server-kit from `leptos_seo` in
    // server-example.yaml — declaring them here as well would produce two
    // `<meta name="description">` / `<link rel="canonical">` tags in the
    // rendered HTML, which SEO scanners flag.
    <Title text="UI Kit Showcase | Impulse" />
    <main class="min-h-screen flex flex-col items-center justify-center p-8 gap-4">
      <h1 class="text-4xl font-semibold">"Impulse UI Kit"</h1>
      <p class="text-lg opacity-80">
        "Server-side rendered demo with hydration and Suspense streaming."
      </p>

      <Suspense fallback=move || {
        view! { <p>"Loading greeting…"</p> }
      }>
        {move || {
          greeting
            .get()
            .map(|res| match res {
              Ok(Greeting { message, words }) => {
                view! { <p class="text-base">{message} " (" {words} " words)"</p> }.into_any()
              }
              Err(err) => view! { <p>"Greeting failed: " {err.to_string()}</p> }.into_any(),
            })
        }}
      </Suspense>

      <Suspense fallback=move || {
        view! {
          <ul>
            <li>"Loading slow data…"</li>
          </ul>
        }
      }>
        {move || {
          slow
            .get()
            .map(|res| match res {
              Ok(items) => {
                view! {
                  <ul class="text-sm">
                    {items.into_iter().map(|s| view! { <li>{s}</li> }).collect_view()}
                  </ul>
                }
                  .into_any()
              }
              Err(err) => view! { <p>"Slow data failed: " {err.to_string()}</p> }.into_any(),
            })
        }}
      </Suspense>
    </main>
  }
}

/// Hydration entrypoint exported to JavaScript.
///
/// The SSR-emitted bootstrap `<script>` calls `hydrate()` after wasm has been
/// loaded. This delegates to [`impulse_ui_kit::setup_app`] under
/// `feature = "hydrate"`, which calls `leptos::mount::hydrate_body`.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
  use impulse_ui_kit::prelude::*;
  setup_app(log::Level::Info, Box::new(move || view! { <App /> }.into_any()));
}
