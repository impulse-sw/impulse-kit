//! Error pages for CC Services.
//!
//! Works with 400, 401, 403, 404, 405, 423 & 500 status codes.
//! Just redirect to `/{status-code}` or return `dist/index.html`
//! instead of requested resource.

#![deny(warnings, clippy::todo, clippy::unimplemented)]

use cc_ui_kit::{
  prelude::*,
  router::{get_path, redirect},
};
use leptos_meta::*;
use lucide_leptos::MoveLeft;

leptos_i18n::load_locales!();
use crate::i18n::*;

fn main() {
  setup_app(
    log::Level::Info,
    Box::new(move || {
      view! {
        <I18nContextProvider>
          <App />
        </I18nContextProvider>
      }
      .into_any()
    }),
  )
}

#[component]
fn App() -> impl IntoView {
  provide_meta_context();
  let i18n = use_i18n();

  view! {
    <Title
      text=move || t_string!(i18n, title)
      formatter=move |text| format!("{text} - CC Services")
    />
    <main>
      {move || match get_path().unwrap().as_str() {
        "/400" => {
          view! {
            <ErrorPage err_num="400".to_string() err_msg=t_string!(i18n, bad_request).to_string() />
          }
            .into_any()
        }
        "/401" => {
          view! {
            <ErrorPage
              err_num="401".to_string()
              err_msg=t_string!(i18n, unauthorized).to_string()
            />
          }
            .into_any()
        }
        "/403" => {
          view! {
            <ErrorPage err_num="403".to_string() err_msg=t_string!(i18n, forbidden).to_string() />
          }
            .into_any()
        }
        "/405" => {
          view! {
            <ErrorPage err_num="405".to_string() err_msg=t_string!(i18n, not_allowed).to_string() />
          }
            .into_any()
        }
        "/423" => {
          view! {
            <ErrorPage err_num="423".to_string() err_msg=t_string!(i18n, locked).to_string() />
          }
            .into_any()
        }
        "/500" => {
          view! {
            <ErrorPage err_num="500".to_string() err_msg=t_string!(i18n, internal).to_string() />
          }
            .into_any()
        }
        "/oops" => {
          view! {
            <ErrorPage err_num="???".to_string() err_msg=t_string!(i18n, specific).to_string() />
          }
            .into_any()
        }
        "/" | "/404" => {
          view! {
            <ErrorPage err_num="404".to_string() err_msg=t_string!(i18n, not_found).to_string() />
          }
            .into_any()
        }
        s if s.len() != 4 => {
          view! {
            <ErrorPage err_num="404".to_string() err_msg=t_string!(i18n, not_found).to_string() />
          }
            .into_any()
        }
        _ => {
          redirect("/oops".to_string()).unwrap();
          ().into_any()
        }
      }}
    </main>
  }
}

#[component]
fn ErrorPage(err_num: String, err_msg: String) -> impl IntoView {
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
  let i18n = use_i18n();
  let ref_is_empty = get_referrer().is_empty();
  let go_back = move || redirect(get_referrer()).unwrap();

  #[cfg(not(debug_assertions))]
  let go_back_through_query = move || redirect(get_go_back_path()).unwrap();

  #[cfg(not(debug_assertions))]
  view! {
    <Show
      when=move || { !ref_is_empty }
      fallback=move || {
        view! {
          <Button appearance=ButtonAppearance::Primary on_click=move |_| go_back_through_query()>
            <MoveLeft size=24 stroke_width=2 />
            <p class="ml-2">{move || t_string!(i18n, go_back)}</p>
          </Button>
        }
      }
    >
      <Button appearance=ButtonAppearance::Primary on_click=move |_| go_back()>
        <MoveLeft size=24 stroke_width=2 />
        <p class="ml-2">{move || t_string!(i18n, go_back)}</p>
      </Button>
    </Show>
  }
  #[cfg(debug_assertions)]
  view! {
    <Show when=move || { !ref_is_empty } fallback=move || view! { <a href="/404">"Go to 404"</a> }>
      <Button appearance=ButtonAppearance::Primary on_click=move |_| go_back()>
        <MoveLeft size=24 stroke_width=2 />
        <p class="ml-2">{move || t_string!(i18n, go_back)}</p>
      </Button>
    </Show>
  }

  // view! {
  //   <Button appearance=ButtonAppearance::Primary on_click=move |_| go_back()>
  //     <MoveLeft color="white" size=24 stroke_width=2 />
  //     <p class="ml-2">{move || t_string!(i18n, go_back)}</p>
  //   </Button>
  // }
}
