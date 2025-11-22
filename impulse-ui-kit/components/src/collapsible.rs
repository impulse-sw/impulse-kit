#![allow(missing_docs, dead_code)]

use leptos::prelude::*;

#[component]
pub fn Collapsible(
  #[prop(optional)] open: Option<RwSignal<bool>>,
  #[prop(optional)] default_open: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(default_open));

  provide_context(CollapsibleContext { is_open });

  view! {
    <div data-slot="collapsible" class=class>
      {children()}
    </div>
  }
}

#[component]
pub fn CollapsibleTrigger(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<CollapsibleContext>().expect("CollapsibleTrigger must be used within Collapsible");

  let handle_click = move |_| {
    context.is_open.update(|open| *open = !*open);
  };

  view! {
    <div data-slot="collapsible-trigger" class=class on:click=handle_click>
      {children()}
    </div>
  }
}

#[component]
pub fn CollapsibleContent(#[prop(optional, into)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<CollapsibleContext>().expect("CollapsibleContent must be used within Collapsible");

  let inner_ref = NodeRef::<leptos::html::Div>::new();
  let should_render = RwSignal::new(false);
  let content_height = RwSignal::new(0);

  let children = StoredValue::new(children);
  let class = StoredValue::new_local(class);

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
      set_timeout(move || should_render.set(false), std::time::Duration::from_millis(300));
    }
  });

  let data_state = move || {
    if context.is_open.get() { "open" } else { "closed" }
  };

  let content_style = move || format!("--radix-collapsible-content-height: {}px", content_height.get());

  view! {
    <Show when=move || should_render.get()>
      <div
        data-slot="collapsible-content"
        data-state=data_state
        class="data-[state=closed]:animate-collapsible-up data-[state=open]:animate-collapsible-down overflow-hidden"
        style=content_style
      >
        <div node_ref=inner_ref class=class.get_value()>
          {children.get_value()()}
        </div>
      </div>
    </Show>
  }
}

#[derive(Clone, Copy)]
struct CollapsibleContext {
  is_open: RwSignal<bool>,
}
