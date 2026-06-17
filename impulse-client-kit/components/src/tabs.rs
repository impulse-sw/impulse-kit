#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

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
      aria-orientation="horizontal"
      class=cn(
        &[
          "bg-muted text-muted-foreground flex h-10 w-full max-w-full items-center justify-start gap-1 overflow-x-auto rounded-md p-1 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden",
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

  let trigger_id = format!("tabs-trigger-{value}");
  let panel_id = format!("tabs-content-{value}");

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
    let key = ev.key();

    // Activate the focused tab.
    if (key == " " || key == "Enter") && !disabled {
      ev.prevent_default();
      context.value.set(value_for_keydown.clone());
      if let Some(callback) = context.on_value_change {
        callback.run(value_for_keydown.clone());
      }
      return;
    }

    // Roving keyboard navigation between tabs (WAI-ARIA tabs pattern).
    if !matches!(key.as_str(), "ArrowRight" | "ArrowLeft" | "Home" | "End") {
      return;
    }
    ev.prevent_default();

    let target = event_target::<web_sys::HtmlButtonElement>(&ev);
    let Ok(Some(list)) = target.closest("[role='tablist']") else {
      return;
    };
    let Ok(tabs) = list.query_selector_all("[role='tab']:not([disabled])") else {
      return;
    };

    let len = tabs.length();
    if len == 0 {
      return;
    }

    let mut current = 0u32;
    for i in 0..len {
      if let Some(node) = tabs.item(i)
        && node.is_same_node(Some(target.unchecked_ref()))
      {
        current = i;
        break;
      }
    }

    let next = match key.as_str() {
      "ArrowRight" => (current + 1) % len,
      "ArrowLeft" => (current + len - 1) % len,
      "Home" => 0,
      "End" => len - 1,
      _ => current,
    };

    // Focusing also brings the tab into view inside the scrollable list, and
    // clicking reuses the activation logic above (automatic activation).
    if let Some(node) = tabs.item(next)
      && let Ok(el) = node.dyn_into::<web_sys::HtmlElement>()
    {
      let _ = el.focus();
      el.click();
    }
  };

  let value_clone = value.clone();
  let is_active_memo = Memo::new(move |_| context.value.get() == value_clone);

  view! {
    <button
      type="button"
      role="tab"
      id=trigger_id
      aria-controls=panel_id
      data-slot="tabs-trigger"
      data-state=move || if is_active_memo.get() { "active" } else { "inactive" }
      aria-selected=move || if is_active_memo.get() { "true" } else { "false" }
      disabled=disabled
      class=cn(
        &[
          "inline-flex shrink-0 items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm",
          class.as_str(),
        ],
      )
      on:click=handle_click
      on:keydown=handle_keydown
      tabindex=move || if !disabled && is_active_memo.get() { "0" } else { "-1" }
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

  let panel_id = format!("tabs-content-{value}");
  let trigger_id = format!("tabs-trigger-{value}");

  let value_clone = value.clone();
  let is_active_memo = Memo::new(move |_| context.value.get() == value_clone);

  let children = StoredValue::new(children);

  view! {
    <Show when=move || is_active_memo.get()>
      <div
        data-slot="tabs-content"
        id=panel_id.clone()
        aria-labelledby=trigger_id.clone()
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
