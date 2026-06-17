#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::attribute_interceptor::AttributeInterceptor;
use leptos::prelude::*;

#[component]
pub fn Avatar(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  provide_context(AvatarContext {
    fallback: RwSignal::new(true),
  });

  view! {
    <div
      data-slot="avatar"
      class=cn(&["relative flex size-8 shrink-0 overflow-hidden rounded-full", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn AvatarImage(#[prop(into, optional)] class: String) -> impl IntoView {
  let context = use_context::<AvatarContext>().expect("AvatarImage must be used within Avatar");
  context.fallback.set(false);

  let class = StoredValue::new_local(class);

  view! {
    <AttributeInterceptor let:attrs>
      {
        let attrs = StoredValue::new_local(attrs);
        view! {
          <Show when=move || !context.fallback.get()>
            <img
              data-slot="avatar-image"
              class=cn(&["aspect-square size-full", class.read_value().as_str()])
              on:error=move |_| context.fallback.set(true)
              {..attrs.get_value()}
            />
          </Show>
        }
      }
    </AttributeInterceptor>
  }
}

#[component]
pub fn AvatarFallback(#[prop(into, optional)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<AvatarContext>().expect("AvatarFallback must be used within Avatar");
  let children = StoredValue::new(children);

  view! {
    <Show when=move || context.fallback.get()>
      <div
        data-slot="avatar-fallback"
        class=cn(
          &["bg-muted flex size-full items-center justify-center rounded-full", class.as_str()],
        )
      >
        {children.get_value()()}
      </div>
    </Show>
  }
}

#[derive(Clone, Copy)]
struct AvatarContext {
  fallback: RwSignal<bool>,
}
