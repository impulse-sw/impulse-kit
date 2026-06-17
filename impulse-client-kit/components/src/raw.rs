#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use super::button::{Button, ButtonSize, ButtonVariant};
use super::card::Card;

const RAW_INLINE_BASE: &str =
  "rounded bg-muted/70 px-1.5 py-0.5 font-mono text-[0.85em] text-foreground/90 border border-border/40";

#[component]
pub fn Raw(#[prop(into)] text: String, #[prop(into, optional)] class: String) -> impl IntoView {
  view! { <code class=cn(&[RAW_INLINE_BASE, class.as_str()])>{text}</code> }
}

pub fn rich(input: &'static str) -> AnyView {
  let mut nodes: Vec<AnyView> = Vec::new();
  let mut is_code = false;
  for part in input.split('`') {
    if !part.is_empty() {
      if is_code {
        nodes.push(view! { <Raw text=part /> }.into_any());
      } else {
        nodes.push(view! { <span>{part}</span> }.into_any());
      }
    }
    is_code = !is_code;
  }
  nodes.into_iter().collect_view().into_any()
}

#[component]
pub fn RawBlock(#[prop(into)] language: String, #[prop(into)] code: String) -> impl IntoView {
  let copied = RwSignal::new(false);
  let code_for_copy = code.clone();
  let on_copy = move |_| {
    if let Some(window) = web_sys::window() {
      let _ = window.navigator().clipboard().write_text(&code_for_copy);
      copied.set(true);
      let copied_cb = copied;
      set_timeout(move || copied_cb.set(false), std::time::Duration::from_millis(1500));
    }
  };

  view! {
    <Card class="overflow-hidden p-0 gap-0">
      <div class="flex items-center justify-between border-b border-border/60 bg-muted/40 px-4 py-2">
        <div class="flex items-center gap-2">
          <span class="h-3 w-3 rounded-full bg-destructive/60" />
          <span class="h-3 w-3 rounded-full bg-chart-1/70" />
          <span class="h-3 w-3 rounded-full bg-chart-3/70" />
          <span class="ml-3 text-xs font-mono text-muted-foreground">{language}</span>
        </div>
        <Button
          variant=ButtonVariant::Ghost
          size=ButtonSize::Sm
          class="h-7 px-2 text-xs"
          on:click=on_copy
        >
          {move || if copied.get() { "Скопировано" } else { "Копировать" }}
        </Button>
      </div>
      <pre class="overflow-x-auto p-4 text-xs sm:text-sm leading-relaxed font-mono text-foreground/90">
        <code>{code}</code>
      </pre>
    </Card>
  }
}
