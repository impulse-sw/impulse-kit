//! Error pages for Impulse services.
//!
//! Works with 400, 401, 403, 404, 405 & 500 status codes.
//! Just redirect to `/{status-code}` or return `dist/index.html`
//! instead of requested resource.

#![deny(warnings, clippy::todo, clippy::unimplemented)]

use impulse_ui_kit_components as components;

use impulse_ui_kit::prelude::*;
use impulse_ui_kit::router::{get_path, redirect};
use leptos_meta::*;

use components::button::Button;

fn main() {
  setup_app(log::Level::Info, Box::new(move || view! { <App /> }.into_any()))
}

#[component]
fn App() -> impl IntoView {
  provide_meta_context();

  view! {
    <Title text="Error! - Impulse Services" />
    <main>
      {move || match get_path().unwrap().as_str() {
        "/400" => {
          view! { <ErrorPage err_num="400" err_msg="Oops! That's a Bad Request!" /> }.into_any()
        }
        "/401" => {
          view! { <ErrorPage err_num="401" err_msg="Oops! You're unauthorized." /> }.into_any()
        }
        "/403" => view! { <ErrorPage err_num="403" err_msg="Access denied." /> }.into_any(),
        "/405" => {
          view! { <ErrorPage err_num="405" err_msg="This method is not allowed." /> }.into_any()
        }
        "/500" => {
          view! {
            <ErrorPage
              err_num="500"
              err_msg="Oops! Internal server error. Contact the administrator."
            />
          }
            .into_any()
        }
        "/oops" => {
          view! {
            <ErrorPage
              err_num="???"
              err_msg="Specific error. Check with the administrator for details."
            />
          }
            .into_any()
        }
        "/" | "/404" => {
          view! {
            <ErrorPage err_num="404" err_msg="Oops! The page you're looking for doesn't exist." />
          }
            .into_any()
        }
        s if s.len() != 4 => {
          view! {
            <ErrorPage err_num="404" err_msg="Oops! The page you're looking for doesn't exist." />
          }
            .into_any()
        }
        _ => {
          redirect("/oops").unwrap();
          ().into_any()
        }
      }}
    </main>
  }
}

#[component]
fn ErrorPage(#[prop(into)] err_num: String, #[prop(into)] err_msg: String) -> impl IntoView {
  view! {
    <div class="flex flex-col items-center justify-center min-h-screen bg-gray-100 dark:bg-gray-900">
      <h1 style="font-size: 72pt;" class="mb-10 text-gray-800 dark:text-gray-200 font-bold">
        {err_num}
      </h1>
      <p class="mb-4 text-xl text-gray-600 dark:text-gray-300 text-center">{err_msg}</p>
      <GoBack />
    </div>
  }
}

fn get_referrer() -> String {
  web_sys::window().unwrap().document().unwrap().referrer()
}

#[cfg(not(debug_assertions))]
fn get_go_back_path() -> String {
  let search = web_sys::window().unwrap().location().search().unwrap();
  let params = web_sys::UrlSearchParams::new_with_str(&search).unwrap();
  let value = params.get("back").unwrap_or("/".to_string());
  value
}

#[component]
fn GoBack() -> impl IntoView {
  let ref_is_empty = get_referrer().is_empty();
  let go_back = move || redirect(get_referrer()).unwrap();

  #[cfg(not(debug_assertions))]
  let go_back_through_query = move || redirect(get_go_back_path()).unwrap();

  #[cfg(not(debug_assertions))]
  view! {
    <Show
      when=move || { !ref_is_empty }
      fallback=move || {
        view! { <Button on:click=move |_| go_back_through_query()>"Go back"</Button> }
      }
    >
      <Button on:click=move |_| go_back()>"Go back"</Button>
    </Show>
  }
  #[cfg(debug_assertions)]
  view! {
    <Show when=move || { !ref_is_empty } fallback=move || view! { <a href="/404">"Go to 404"</a> }>
      <Button on:click=move |_| go_back()>"Go back"</Button>
    </Show>
  }
}
