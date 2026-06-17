#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

#[component]
pub fn FieldSet(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <fieldset data-slot="field-set" class=cn(&["space-y-6", class.as_str()])>
      {children()}
    </fieldset>
  }
}

#[component]
pub fn FieldGroup(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="field-group" class=cn(&["space-y-4", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn Field(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="field" class=cn(&["flex flex-col gap-2", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn FieldLabel(
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] r#for: String,
  children: Children,
) -> impl IntoView {
  view! {
    <label
      data-slot="field-label"
      r#for=r#for
      class=cn(
        &[
          "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
          class.as_str(),
        ],
      )
    >

      {children()}
    </label>
  }
}

#[component]
pub fn FieldDescription(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div data-slot="field-description" class=cn(&["text-muted-foreground text-sm", class.as_str()])>
      {children()}
    </div>
  }
}

#[component]
pub fn FieldError(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="field-error"
      class=cn(&["text-destructive text-sm font-medium", class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn FieldSeparator(#[prop(optional, into)] class: String) -> impl IntoView {
  view! { <hr data-slot="field-separator" class=cn(&["border-border my-4", class.as_str()]) /> }
}
