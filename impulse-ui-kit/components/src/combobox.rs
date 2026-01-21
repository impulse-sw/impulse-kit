#![allow(missing_docs, dead_code)]

// Combobox is an autocomplete/searchable select component
// This is a simplified implementation that can be expanded with full functionality

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy)]
struct ComboboxContext {
  is_open: RwSignal<bool>,
  value: RwSignal<String>,
  on_value_change: Option<Callback<String>>,
}

#[component]
pub fn Combobox(
  #[prop(optional, into)] value: Option<RwSignal<String>>,
  #[prop(optional, into)] default_value: String,
  #[prop(optional)] on_value_change: Option<Callback<String>>,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let value = value.unwrap_or_else(|| RwSignal::new(default_value));
  let is_open = RwSignal::new(false);

  provide_context(ComboboxContext {
    is_open,
    value,
    on_value_change,
  });

  view! {
    <div data-slot="combobox" class=cn(&["relative", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn ComboboxTrigger(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<ComboboxContext>().expect("ComboboxTrigger must be used within Combobox");

  let handle_click = move |_| {
    context.is_open.update(|open| *open = !*open);
  };

  view! {
    <button
      type="button"
      data-slot="combobox-trigger"
      aria-expanded=move || context.is_open.get()
      class=cn(
        &[
          "flex h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
          class.as_str(),
        ],
      )
      on:click=handle_click
    >
      {children()}
    </button>
  }
}

#[component]
pub fn ComboboxContent(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<ComboboxContext>().expect("ComboboxContent must be used within Combobox");

  view! {
    <Show when=move || context.is_open.get()>
      <div
        data-slot="combobox-content"
        class=cn(
          &[
            "absolute z-50 mt-1 max-h-60 w-full overflow-auto rounded-md border bg-popover p-1 text-popover-foreground shadow-md",
            class.as_str(),
          ],
        )
      >
        {children()}
      </div>
    </Show>
  }
}

#[component]
pub fn ComboboxInput(
  #[prop(into, optional)] class: String,
  #[prop(into, optional)] placeholder: String,
) -> impl IntoView {
  let context = use_context::<ComboboxContext>().expect("ComboboxInput must be used within Combobox");

  view! {
    <input
      type="text"
      data-slot="combobox-input"
      placeholder=placeholder
      class=cn(
        &[
          "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
          class.as_str(),
        ],
      )
      on:focus=move |_| context.is_open.set(true)
    />
  }
}

#[component]
pub fn ComboboxItem(
  #[prop(into)] value: String,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let context = use_context::<ComboboxContext>().expect("ComboboxItem must be used within Combobox");

  let handle_click = move |_| {
    context.value.set(value.clone());
    if let Some(callback) = context.on_value_change {
      callback.run(value.clone());
    }
    context.is_open.set(false);
  };

  view! {
    <div
      data-slot="combobox-item"
      class=cn(
        &[
          "relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none hover:bg-accent hover:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
          class.as_str(),
        ],
      )
      on:click=handle_click
    >
      {children()}
    </div>
  }
}

#[component]
pub fn ComboboxEmpty(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="combobox-empty" class=cn(&["py-6 text-center text-sm", class.as_str()])>
      {children()}
    </div>
  }
}
