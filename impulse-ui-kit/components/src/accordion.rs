#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum AccordionType {
  #[default]
  Single,
  Multiple,
}

#[component]
pub fn Accordion(
  #[prop(optional)] accordion_type: AccordionType,
  #[prop(optional)] default_value: Option<Vec<String>>,
  #[prop(optional)] value: Option<RwSignal<Vec<String>>>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let open_items = value.unwrap_or_else(|| RwSignal::new(default_value.unwrap_or_default()));

  provide_context(AccordionContext {
    open_items,
    accordion_type,
  });

  view! {
    <div data-slot="accordion" class=class>
      {children()}
    </div>
  }
}

#[component]
pub fn AccordionItem(
  #[prop(into)] value: String,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let context = use_context::<AccordionContext>().expect("AccordionItem must be used within Accordion");

  let _value = value.clone();
  let is_open = Memo::new(move |_| context.open_items.get().contains(&_value));

  provide_context(AccordionItemContext {
    value: value.clone(),
    is_open,
  });

  view! {
    <div data-slot="accordion-item" class=cn(&["border-b last:border-b-0", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn AccordionTrigger(
  #[prop(optional, into)] class: String,
  #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
  let accordion_context = use_context::<AccordionContext>().expect("AccordionTrigger must be used within Accordion");

  let item_context = use_context::<AccordionItemContext>().expect("AccordionTrigger must be used within AccordionItem");

  let value = item_context.value.clone();

  let handle_click = move |_| {
    accordion_context
      .open_items
      .update(|items| match accordion_context.accordion_type {
        AccordionType::Single => {
          if items.contains(&value) {
            items.clear();
          } else {
            items.clear();
            items.push(value.clone());
          }
        }
        AccordionType::Multiple => {
          if items.contains(&value) {
            items.retain(|v| v != &value);
          } else {
            items.push(value.clone());
          }
        }
      });
  };

  let data_state = move || {
    if item_context.is_open.get() { "open" } else { "closed" }
  };

  view! {
    <div class="flex">
      <button
        type="button"
        data-slot="accordion-trigger"
        data-state=data_state
        class=cn(
          &[
            "focus-visible:border-ring focus-visible:ring-ring/50 flex flex-1 items-start justify-between gap-4 rounded-md py-4 text-left text-sm font-medium transition-all outline-none hover:underline focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50 [&[data-state=open]>svg]:rotate-180",
            class.as_str(),
          ],
        )
        on:click=handle_click
      >
        {if let Some(children) = children { children().into_any() } else { ().into_any() }}

        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="text-muted-foreground pointer-events-none size-4 shrink-0 translate-y-0.5 transition-transform duration-200"
        >
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>
    </div>
  }
}

#[component]
pub fn AccordionContent(#[prop(optional, into)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<AccordionItemContext>().expect("AccordionContent must be used within AccordionItem");

  let inner_ref = NodeRef::<leptos::html::Div>::new();
  let should_render = RwSignal::new(false);
  let content_height = RwSignal::new(0);
  let children = StoredValue::new(children);

  Effect::new(move |_| {
    if context.is_open.get() {
      request_animation_frame(move || {
        if let Some(inner) = inner_ref.get() {
          let height = inner.scroll_height();
          content_height.set(height);
        }
      });
    }
  });

  Effect::new(move |_| {
    if context.is_open.get() {
      should_render.set(true);
    } else if should_render.get() {
      set_timeout(move || should_render.set(false), std::time::Duration::from_millis(200));
    }
  });

  let data_state = move || {
    if context.is_open.get() { "open" } else { "closed" }
  };

  let content_style = move || format!("--radix-accordion-content-height: {}px", content_height.get());

  view! {
    <Show when=move || should_render.get()>
      <div
        data-slot="accordion-content"
        data-state=data_state
        class="data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down overflow-hidden text-sm"
        style=content_style
      >
        <div node_ref=inner_ref class=cn(&["pt-0 pb-4", class.as_str()])>
          {children.get_value()()}
        </div>
      </div>
    </Show>
  }
}

#[derive(Clone, Copy)]
struct AccordionContext {
  open_items: RwSignal<Vec<String>>,
  accordion_type: AccordionType,
}

#[derive(Clone)]
struct AccordionItemContext {
  value: String,
  is_open: Memo<bool>,
}
