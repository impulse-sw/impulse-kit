#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

use super::label::Label;

#[component]
pub fn Form(
  #[prop(into, optional)] class: String,
  #[prop(optional)] on_submit: Option<Callback<web_sys::Event>>,
  children: Children,
) -> impl IntoView {
  let handle_submit = move |ev: web_sys::Event| {
    ev.prevent_default();
    if let Some(callback) = on_submit {
      callback.run(ev);
    }
  };

  view! {
    <form data-slot="form" class=cn(&["space-y-6", class.as_str()]) on:submit=handle_submit>
      {children()}
    </form>
  }
}

#[component]
pub fn FormField(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="form-field" class=cn(&["space-y-2", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn FormItem(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="form-item" class=cn(&["space-y-2", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn FormLabel(
  #[prop(into, optional)] class: String,
  #[prop(into, optional)] r#for: String,
  children: Children,
) -> impl IntoView {
  view! { <Label r#for=r#for class=class>{children()}</Label> }
}

#[component]
pub fn FormControl(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="form-control" class=class>
      {children()}
    </div>
  }
}

#[component]
pub fn FormDescription(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <p data-slot="form-description" class=cn(&["text-sm text-muted-foreground", class.as_str()])>
      {children()}
    </p>
  }
}

#[component]
pub fn FormMessage(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <p
      data-slot="form-message"
      class=cn(&["text-sm font-medium text-destructive", class.as_str()])
    >
      {children()}
    </p>
  }
}
