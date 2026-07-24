#![allow(missing_docs, dead_code)]

//! Usage:
//!
//! web-sys = { version = "0.3.82", features = ["ClipboardEvent", "DataTransfer"] }

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

const BASE_CLASSES_SINGLE: &str = "relative flex h-9 w-9 items-center justify-center border-y border-r border-input text-sm shadow-xs transition-all outline-none text-center focus:z-10 focus:border-ring focus:ring-[3px] focus:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50";

const BASE_CLASSES_SEPARATED: &str = "relative flex h-9 w-9 items-center justify-center border-y border-r border-input text-sm shadow-xs transition-all outline-none text-center focus:z-10 focus:border-ring focus:ring-[3px] focus:ring-ring/50";

#[component]
pub fn InputOTP(
  #[prop(into)] length: usize,
  #[prop(optional)] on_complete: Option<Callback<String>>,
  #[prop(optional)] on_change: Option<Callback<String>>,
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] container_class: String,
) -> impl IntoView {
  let values = RwSignal::new(vec![String::new(); length]);
  let input_refs = (0..length)
    .map(|_| NodeRef::<leptos::html::Input>::new())
    .collect::<Vec<_>>();

  let _input_refs = input_refs.clone();
  let mouse_click = move |ev: leptos::ev::MouseEvent| {
    ev.prevent_default();

    let vals = values.get();
    let first_empty_index = vals.iter().position(|v| v.is_empty()).unwrap_or(length - 1);

    if let Some(input) = _input_refs[first_empty_index].get() {
      let _ = input.focus();
      let _ = input.set_selection_start(Some(1));
    }
  };

  let _input_refs = input_refs.clone();
  let focus_event = move |ev: leptos::ev::FocusEvent| {
    ev.prevent_default();

    let vals = values.get();
    let first_empty_index = vals.iter().position(|v| v.is_empty()).unwrap_or(length - 1);

    if let Some(input) = _input_refs[first_empty_index].get() {
      let _ = input.focus();
      let _ = input.set_selection_start(Some(1));
    }
  };

  let _input_refs = input_refs.clone();
  let handle_input = move |index: usize, ev: leptos::ev::Event| {
    let input = event_target::<leptos::web_sys::HtmlInputElement>(&ev);
    let value = input.value();

    let filtered: String = value.chars().filter(|c| c.is_numeric()).take(1).collect();

    values.update(|v| {
      v[index] = filtered.clone();
    });

    input.set_value(&filtered);

    if !filtered.is_empty()
      && index < length - 1
      && let Some(next_input) = _input_refs[index + 1].get()
    {
      let _ = next_input.focus();
    }

    if let Some(on_change) = on_change {
      let current_value = values.get().join("");
      on_change.run(current_value.clone());
    }

    if let Some(on_complete) = on_complete {
      let current_value = values.get().join("");
      if current_value.len() == length {
        on_complete.run(current_value);
      }
    }
  };

  let _input_refs = input_refs.clone();
  let handle_keydown = move |index: usize, ev: leptos::ev::KeyboardEvent| {
    let key = ev.key();

    if key == "Backspace" {
      let current_value = values.get()[index].clone();

      if current_value.is_empty() && index > 0 {
        ev.prevent_default();
        if let Some(prev_input) = _input_refs[index - 1].get() {
          values.update(|v| {
            v[index - 1] = String::new();
          });
          let _ = prev_input.focus();
        }
      }
    }
  };

  let _input_refs = input_refs.clone();
  let handle_paste = move |ev: leptos::ev::ClipboardEvent| {
    ev.prevent_default();

    if ev.type_().eq("paste")
      && let Some(clipboard_data) = ev.clipboard_data()
      && let Ok(text) = clipboard_data.get_data("text")
    {
      let digits: Vec<String> = text
        .chars()
        .filter(|c| c.is_numeric())
        .take(length)
        .map(|c| c.to_string())
        .collect();

      values.update(|v| {
        for (i, digit) in digits.iter().enumerate() {
          if i < length {
            v[i] = digit.clone();
          }
        }
      });

      let next_empty = digits.len().min(length - 1);
      if let Some(input) = _input_refs[next_empty].get() {
        let _ = input.focus();
      }

      if let Some(on_change) = on_change {
        let current_value = values.get().join("");
        on_change.run(current_value.clone());
      }

      if let Some(on_complete) = on_complete {
        let current_value = values.get().join("");
        if current_value.len() == length {
          on_complete.run(current_value);
        }
      }
    }
  };

  view! {
    <div class=cn(&["flex items-center gap-2 {}", container_class.as_str()]) on:paste=handle_paste>
      <div class="flex items-center">
        {(0..length)
          .map(|index| {
            let input_ref = input_refs[index];
            let value = move || values.get()[index].clone();
            let handle_input = handle_input.clone();
            let handle_keydown = handle_keydown.clone();
            let mouse_click = mouse_click.clone();
            let focus_event = focus_event.clone();

            view! {
              <input
                type="text"
                inputmode="numeric"
                maxlength="1"
                node_ref=input_ref
                prop:value=value
                on:input=move |ev| handle_input(index, ev)
                on:keydown=move |ev| handle_keydown(index, ev)
                on:focus=focus_event
                on:click=mouse_click
                class=cn(
                  &[
                    BASE_CLASSES_SINGLE,
                    if index == 0 { "rounded-l-md border-l" } else { "" },
                    if index == length - 1 { "rounded-r-md" } else { "" },
                    class.as_str(),
                  ],
                )
              />
            }
          })
          .collect_view()}
      </div>
    </div>
  }
}

#[component]
pub fn InputOTPWithSeparator(
  #[prop(into)] length: usize,
  #[prop(into)] separator_at: usize,
  #[prop(optional)] on_complete: Option<Callback<String>>,
  #[prop(optional)] on_change: Option<Callback<String>>,
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] container_class: String,
) -> impl IntoView {
  let values = RwSignal::new(vec![String::new(); length]);
  let input_refs = (0..length)
    .map(|_| NodeRef::<leptos::html::Input>::new())
    .collect::<Vec<_>>();

  let _input_refs = input_refs.clone();
  let mouse_click = move |ev: leptos::ev::MouseEvent| {
    ev.prevent_default();

    let vals = values.get();
    let first_empty_index = vals.iter().position(|v| v.is_empty()).unwrap_or(length - 1);

    if let Some(input) = _input_refs[first_empty_index].get() {
      let _ = input.focus();
      let _ = input.set_selection_start(Some(1));
    }
  };

  let _input_refs = input_refs.clone();
  let focus_event = move |ev: leptos::ev::FocusEvent| {
    ev.prevent_default();

    let vals = values.get();
    let first_empty_index = vals.iter().position(|v| v.is_empty()).unwrap_or(length - 1);

    if let Some(input) = _input_refs[first_empty_index].get() {
      let _ = input.focus();
      let _ = input.set_selection_start(Some(1));
    }
  };

  let _input_refs = input_refs.clone();
  let handle_input = move |index: usize, ev: leptos::ev::Event| {
    let input = event_target::<leptos::web_sys::HtmlInputElement>(&ev);
    let value = input.value();

    let filtered: String = value.chars().filter(|c| c.is_numeric()).take(1).collect();

    values.update(|v| {
      v[index] = filtered.clone();
    });

    input.set_value(&filtered);

    if !filtered.is_empty()
      && index < length - 1
      && let Some(next_input) = _input_refs[index + 1].get()
    {
      let _ = next_input.focus();
    }

    if let Some(on_change) = on_change {
      let current_value = values.get().join("");
      on_change.run(current_value.clone());
    }

    if let Some(on_complete) = on_complete {
      let current_value = values.get().join("");
      if current_value.len() == length {
        on_complete.run(current_value);
      }
    }
  };

  let _input_refs = input_refs.clone();
  let handle_keydown = move |index: usize, ev: leptos::ev::KeyboardEvent| {
    let key = ev.key();

    if key == "Backspace" {
      let current_value = values.get()[index].clone();

      if current_value.is_empty() && index > 0 {
        ev.prevent_default();
        if let Some(prev_input) = _input_refs[index - 1].get() {
          values.update(|v| {
            v[index - 1] = String::new();
          });
          let _ = prev_input.focus();
        }
      }
    }
  };

  let _input_refs = input_refs.clone();
  let handle_paste = move |ev: leptos::ev::ClipboardEvent| {
    ev.prevent_default();

    if ev.type_().eq("paste")
      && let Some(clipboard_data) = ev.clipboard_data()
      && let Ok(text) = clipboard_data.get_data("text")
    {
      let digits: Vec<String> = text
        .chars()
        .filter(|c| c.is_numeric())
        .take(length)
        .map(|c| c.to_string())
        .collect();

      values.update(|v| {
        for (i, digit) in digits.iter().enumerate() {
          if i < length {
            v[i] = digit.clone();
          }
        }
      });

      let next_empty = digits.len().min(length - 1);
      if let Some(input) = _input_refs[next_empty].get() {
        let _ = input.focus();
      }

      if let Some(on_change) = on_change {
        let current_value = values.get().join("");
        on_change.run(current_value.clone());
      }

      if let Some(on_complete) = on_complete {
        let current_value = values.get().join("");
        if current_value.len() == length {
          on_complete.run(current_value);
        }
      }
    }
  };

  view! {
    <div class=cn(&["flex items-center gap-2 {}", container_class.as_str()]) on:paste=handle_paste>
      <div class="flex items-center">
        {(0..separator_at)
          .map(|index| {
            let input_ref = input_refs[index];
            let value = move || values.get()[index].clone();
            let handle_input = handle_input.clone();
            let handle_keydown = handle_keydown.clone();
            let mouse_click = mouse_click.clone();
            let focus_event = focus_event.clone();

            view! {
              <input
                type="text"
                inputmode="numeric"
                maxlength="1"
                node_ref=input_ref
                prop:value=value
                on:input=move |ev| handle_input(index, ev)
                on:keydown=move |ev| handle_keydown(index, ev)
                on:focus=focus_event
                on:click=mouse_click
                class=cn(
                  &[
                    BASE_CLASSES_SEPARATED,
                    if index == 0 { "rounded-l-md border-l" } else { "" },
                    class.as_str(),
                  ],
                )
              />
            }
          })
          .collect_view()}
      </div>

      <div class="flex items-center justify-center">
        <span class="select-none text-muted-foreground">"-"</span>
      </div>

      <div class="flex items-center">
        {(separator_at..length)
          .map(|index| {
            let input_ref = input_refs[index];
            let value = move || values.get()[index].clone();
            let handle_input = handle_input.clone();
            let handle_keydown = handle_keydown.clone();
            let mouse_click = mouse_click.clone();
            let focus_event = focus_event.clone();

            view! {
              <input
                type="text"
                inputmode="numeric"
                maxlength="1"
                node_ref=input_ref
                prop:value=value
                on:input=move |ev| handle_input(index, ev)
                on:keydown=move |ev| handle_keydown(index, ev)
                on:focus=focus_event
                on:click=mouse_click
                class=cn(
                  &[
                    BASE_CLASSES_SEPARATED,
                    if index == separator_at { "border-l" } else { "" },
                    if index == length - 1 { "rounded-r-md border-r" } else { "" },
                    class.as_str(),
                  ],
                )
              />
            }
          })
          .collect_view()}
      </div>
    </div>
  }
}
