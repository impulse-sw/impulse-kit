#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy)]
struct TabsContext {
  value: RwSignal<String>,
  on_value_change: Option<Callback<String>>,
}

#[component]
pub fn Tabs(
  #[prop(optional, into)] value: Option<RwSignal<String>>,
  #[prop(optional, into)] default_value: String,
  #[prop(optional)] on_value_change: Option<Callback<String>>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let value = value.unwrap_or_else(|| RwSignal::new(default_value));

  provide_context(TabsContext { value, on_value_change });

  view! {
    <div data-slot="tabs" class=cn(&["w-full", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn TabsList(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="tabs-list"
      role="tablist"
      class=cn(
        &[
          "bg-muted text-muted-foreground inline-flex h-10 items-center justify-center rounded-md p-1",
          class.as_str(),
        ],
      )
    >
      {children()}
    </div>
  }
}

#[component]
pub fn TabsTrigger(
  #[prop(into)] value: String,
  #[prop(optional, into)] class: String,
  #[prop(optional)] disabled: bool,
  children: Children,
) -> impl IntoView {
  let context = use_context::<TabsContext>().expect("TabsTrigger must be used within Tabs");

  let value_for_click = value.clone();
  let handle_click = move |_| {
    if !disabled {
      context.value.set(value_for_click.clone());
      if let Some(callback) = context.on_value_change {
        callback.run(value_for_click.clone());
      }
    }
  };

  let value_for_keydown = value.clone();
  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    if (ev.key() == " " || ev.key() == "Enter") && !disabled {
      ev.prevent_default();
      context.value.set(value_for_keydown.clone());
      if let Some(callback) = context.on_value_change {
        callback.run(value_for_keydown.clone());
      }
    }
  };

  let value_clone = value.clone();
  let is_active_memo = Memo::new(move |_| context.value.get() == value_clone);

  view! {
    <button
      type="button"
      role="tab"
      data-slot="tabs-trigger"
      data-state=move || if is_active_memo.get() { "active" } else { "inactive" }
      aria-selected=move || if is_active_memo.get() { "true" } else { "false" }
      disabled=disabled
      class=cn(
        &[
          "inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm",
          class.as_str(),
        ],
      )
      on:click=handle_click
      on:keydown=handle_keydown
      tabindex=if disabled { "-1" } else { "0" }
    >
      {children()}
    </button>
  }
}

#[component]
pub fn TabsContent(
  #[prop(into)] value: String,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<TabsContext>().expect("TabsContent must be used within Tabs");

  let value_clone = value.clone();
  let is_active_memo = Memo::new(move |_| context.value.get() == value_clone);

  let children = StoredValue::new(children);

  view! {
    <Show when=move || is_active_memo.get()>
      <div
        data-slot="tabs-content"
        data-state=move || if is_active_memo.get() { "active" } else { "inactive" }
        role="tabpanel"
        class=cn(
          &[
            "mt-2 ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
            class.as_str(),
          ],
        )
        tabindex="0"
      >
        {children.get_value()()}
      </div>
    </Show>
  }
}
