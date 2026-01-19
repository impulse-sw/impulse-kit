#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn KbdGroup(
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] separator: String,
  children: Children,
) -> impl IntoView {
  let separator = if separator.is_empty() {
    "+".to_string()
  } else {
    separator
  };

  provide_context(KbdGroupContext { separator });

  view! {
    <div data-slot="kbd-group" class=cn(&["inline-flex items-center gap-1", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn Kbd(
  #[prop(optional, into)] class: String,
  #[prop(optional)] show_separator: bool,
  children: Children,
) -> impl IntoView {
  let context = use_context::<KbdGroupContext>();

  view! {
    <>
      <kbd
        data-slot="kbd"
        class=cn(
          &[
            "border-input bg-muted text-muted-foreground inline-flex h-5 min-w-5 items-center justify-center rounded border px-1 font-mono text-xs font-medium shadow-sm",
            class.as_str(),
          ],
        )
      >

        {children()}
      </kbd>
      {
        if show_separator {
          if let Some(ctx) = context {
            view! {
              <span class="text-muted-foreground text-xs">{ctx.separator.clone()}</span>
            }
              .into_any()
          } else {
            ().into_any()
          }
        } else {
          ().into_any()
        }
      }

    </>
  }
}

#[derive(Clone)]
struct KbdGroupContext {
  separator: String,
}
