//! HTML prefix/suffix builder.
//!
//! The prefix encodes the `<!doctype html>...<head>` opener with a
//! `<!--HEAD-->` marker that `leptos_meta::ServerMetaContextOutput::inject_meta_context`
//! splices `<Title>`/`<Meta>`/`<Link>` components into. The suffix closes
//! `<body>` and `<html>`.
//!
//! The rendered application markup is emitted as a direct child of `<body>`
//! so that `leptos::mount::hydrate_body` (which walks `document.body`) finds
//! the same DOM structure that was streamed from the server. Wrapping the
//! body in any extra element (`<div id="main">`, etc.) would cause a
//! `failed_to_cast_element` panic during hydration.

use super::options::{LeptosOptions, PKG_SUBDIR};

/// Inputs for [`build_html_prefix`].
pub(super) struct PrefixContext<'a> {
  pub opts: &'a LeptosOptions,
  pub initial_theme: Option<&'a str>,
  pub request_path: &'a str,
}

pub(super) fn build_html_prefix(ctx: &PrefixContext<'_>) -> String {
  let seo = &ctx.opts.seo_defaults;
  let theme_class = ctx.initial_theme.unwrap_or("light");
  let lang = seo.locale.as_deref().unwrap_or("en");

  let mut head_extras = String::new();
  if let Some(desc) = &seo.description {
    head_extras.push_str(&format!(
      "<meta name=\"description\" content=\"{}\">",
      html_escape(desc)
    ));
  }
  if let Some(robots) = &seo.robots {
    head_extras.push_str(&format!("<meta name=\"robots\" content=\"{}\">", html_escape(robots)));
  }
  if let Some(base) = &seo.canonical_base {
    let trimmed = base.trim_end_matches('/');
    head_extras.push_str(&format!(
      "<link rel=\"canonical\" href=\"{}{}\">",
      html_escape(trimmed),
      html_escape(ctx.request_path)
    ));
  }
  if let Some(handle) = &seo.twitter_handle {
    head_extras.push_str(&format!(
      "<meta name=\"twitter:site\" content=\"{}\">",
      html_escape(handle)
    ));
  }
  head_extras.push_str("<meta name=\"twitter:card\" content=\"summary_large_image\">");
  if let Some(img) = &seo.og_image {
    head_extras.push_str(&format!(
      "<meta property=\"og:image\" content=\"{}\">",
      html_escape(img)
    ));
    head_extras.push_str(&format!(
      "<meta name=\"twitter:image\" content=\"{}\">",
      html_escape(img)
    ));
  }
  if let Some(logo) = &seo.og_logo {
    head_extras.push_str(&format!(
      "<meta property=\"og:logo\" content=\"{}\">",
      html_escape(logo)
    ));
  }
  if let Some(default_title) = &seo.default_title {
    head_extras.push_str(&format!(
      "<meta property=\"og:title\" content=\"{}\">",
      html_escape(default_title)
    ));
  }
  if let Some(desc) = &seo.description {
    head_extras.push_str(&format!(
      "<meta property=\"og:description\" content=\"{}\">",
      html_escape(desc)
    ));
  }
  head_extras.push_str("<meta property=\"og:type\" content=\"website\">");
  if let Some(base) = &seo.canonical_base {
    let trimmed = base.trim_end_matches('/');
    head_extras.push_str(&format!(
      "<meta property=\"og:url\" content=\"{}{}\">",
      html_escape(trimmed),
      html_escape(ctx.request_path)
    ));
  }
  if let Some(site_name) = seo.site_name.as_ref().or(seo.default_title.as_ref()) {
    head_extras.push_str(&format!(
      "<meta property=\"og:site_name\" content=\"{}\">",
      html_escape(site_name)
    ));
  }
  if let Some(locale) = &seo.locale {
    head_extras.push_str(&format!(
      "<meta property=\"og:locale\" content=\"{}\">",
      html_escape(&og_locale(locale))
    ));
  }

  let asset_links = build_asset_links(ctx.opts);

  format!(
    "<!DOCTYPE html><html lang=\"{lang}\" class=\"{theme_class}\"><head>\
       <meta charset=\"UTF-8\">\
       <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
       {THEME_INIT_SCRIPT}\
       {head_extras}{asset_links}<!--HEAD--></head><body>"
  )
}

/// Blocking, render-before-paint script that reconciles the `<html>` theme
/// class with the visitor's actual preference, eliminating the dark-mode
/// flash-of-incorrect-theme (FOIT) on SSR.
///
/// The server picks `theme_class` from the `impulse_theme` cookie, defaulting
/// to `light`. For a first-time visitor the cookie is absent, so the server
/// can never know the OS-level `prefers-color-scheme` and would paint light
/// before the hydration runtime swaps in dark — the classic flicker.
///
/// This synchronous inline script runs while the `<head>` is parsed, *before*
/// the `<body>` is painted, and sets the class from the same source of truth
/// the client [`ThemeProvider`] uses:
/// 1. the `theme` key in `localStorage` (an explicit user choice), then
/// 2. the `prefers-color-scheme` media query (the OS default).
///
/// Because it precedes first paint, the body is never painted with the wrong
/// theme. The literals (`theme`, `dark`, `light`) mirror the constants in
/// `impulse-ui-kit-components::theme`; keep them in sync.
const THEME_INIT_SCRIPT: &str = "<script>(function(){try{\
var e=localStorage.getItem(\"theme\"),c=document.documentElement.classList;\
if(e===\"dark\"||(e!==\"light\"&&window.matchMedia&&window.matchMedia(\"(prefers-color-scheme: dark)\").matches)){c.add(\"dark\");c.remove(\"light\")}\
else{c.remove(\"dark\");c.add(\"light\")}\
}catch(_){}})()</script>";

pub(super) fn build_html_suffix(opts: &LeptosOptions) -> String {
  let mut buf = String::new();
  if opts.include_hydration_script && !opts.output_name.is_empty() {
    buf.push_str(&format!(
      "<script type=\"module\">import init,{{hydrate}} from \"/{pkg}/{out}.js\";\
       init({{module_or_path:\"/{pkg}/{out}_bg.wasm\"}}).then(()=>hydrate());</script>",
      pkg = PKG_SUBDIR,
      out = opts.output_name,
    ));
  }
  buf.push_str("</body></html>");
  buf
}

fn build_asset_links(opts: &LeptosOptions) -> String {
  if opts.output_name.is_empty() {
    return String::new();
  }
  let pkg = PKG_SUBDIR;
  let out = &opts.output_name;
  let css = format!("<link rel=\"stylesheet\" href=\"/{pkg}/{out}.css\">");
  if opts.include_hydration_script {
    format!(
      "{css}\
       <link rel=\"modulepreload\" href=\"/{pkg}/{out}.js\">\
       <link rel=\"preload\" as=\"fetch\" type=\"application/wasm\" crossorigin href=\"/{pkg}/{out}_bg.wasm\">",
    )
  } else {
    css
  }
}

fn html_escape(s: &str) -> String {
  s.replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
    .replace('\'', "&#39;")
}

/// Normalise an HTML `lang` value to OpenGraph's `lang_REGION` form.
///
/// OG validators expect locales like `en_US`, `ru_RU`, `pt_BR`. We accept
/// either form on input: a bare language code (`ru`) is expanded using a
/// small lookup; an already-region-tagged value (`pt-BR`, `pt_BR`) is
/// normalised to the underscore form. Unknown codes pass through unchanged.
fn og_locale(lang: &str) -> String {
  let trimmed = lang.trim();
  if let Some((primary, region)) = trimmed.split_once('-').or_else(|| trimmed.split_once('_')) {
    return format!("{}_{}", primary.to_ascii_lowercase(), region.to_ascii_uppercase());
  }
  let lower = trimmed.to_ascii_lowercase();
  let region = match lower.as_str() {
    "en" => "US",
    "ru" => "RU",
    "de" => "DE",
    "fr" => "FR",
    "es" => "ES",
    "it" => "IT",
    "pt" => "PT",
    "ja" => "JP",
    "ko" => "KR",
    "zh" => "CN",
    "uk" => "UA",
    "pl" => "PL",
    "nl" => "NL",
    "sv" => "SE",
    "tr" => "TR",
    _ => return lower,
  };
  format!("{lower}_{region}")
}

#[cfg(test)]
mod tests {
  use super::{PrefixContext, build_html_prefix, og_locale};
  use crate::leptos_ssr::options::LeptosOptions;

  #[test]
  fn prefix_embeds_blocking_theme_script_before_body() {
    let opts = LeptosOptions::default();
    let html = build_html_prefix(&PrefixContext {
      opts: &opts,
      initial_theme: Some("dark"),
      request_path: "/",
    });
    // Server-side class reflects the cookie, the blocking script lives in the
    // <head> ahead of the <body>, and it reads both signals the client uses.
    assert!(html.contains("class=\"dark\""));
    let script_at = html.find("<script>(function()").expect("theme init script present");
    let body_at = html.find("<body>").expect("body opener present");
    assert!(script_at < body_at, "theme script must run before <body> is painted");
    assert!(html.contains("localStorage.getItem(\"theme\")"));
    assert!(html.contains("prefers-color-scheme: dark"));
  }

  #[test]
  fn og_locale_expands_known_primary() {
    assert_eq!(og_locale("ru"), "ru_RU");
    assert_eq!(og_locale("en"), "en_US");
    assert_eq!(og_locale("pt"), "pt_PT");
  }

  #[test]
  fn og_locale_preserves_explicit_region() {
    assert_eq!(og_locale("pt-BR"), "pt_BR");
    assert_eq!(og_locale("en_GB"), "en_GB");
    assert_eq!(og_locale("ZH-tw"), "zh_TW");
  }

  #[test]
  fn og_locale_passes_unknown_through() {
    assert_eq!(og_locale("xx"), "xx");
  }
}
