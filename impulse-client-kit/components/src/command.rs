#![allow(missing_docs, dead_code)]

// Usage:
//
// uuid = { version = "1.18.1", features = ["v4", "js"] }

use impulse_client_kit::utils::cn;
use leptos::attribute_interceptor::AttributeInterceptor;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use std::collections::{HashMap, HashSet};
use web_sys::{Element, KeyboardEvent};

#[component]
pub fn Command(
  #[prop(optional, into)] label: String,
  #[prop(optional)] should_filter: Option<bool>,
  #[prop(optional, into)] default_value: String,
  #[prop(optional)] value: Option<RwSignal<String>>,
  #[prop(optional)] on_value_change: Option<Callback<String>>,
  #[prop(optional)] loop_navigation: bool,
  #[prop(optional)] disable_pointer_selection: bool,
  #[prop(optional)] vim_bindings: Option<bool>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let list_id = uuid::Uuid::new_v4().to_string();
  let label_id = uuid::Uuid::new_v4().to_string();
  let input_id = uuid::Uuid::new_v4().to_string();

  let search = RwSignal::new(String::new());
  let internal_value = RwSignal::new(default_value);
  let value_signal = value.unwrap_or(internal_value);
  let selected_item_id = RwSignal::new(None);
  let filtered_count = RwSignal::new(0);
  let filtered_items = RwSignal::new(HashMap::new());
  let filtered_groups = RwSignal::new(HashSet::new());
  let all_items = RwSignal::new(HashSet::new());
  let all_groups = RwSignal::new(HashMap::new());
  let item_values = RwSignal::new(HashMap::new());
  let list_inner_ref = NodeRef::<leptos::html::Div>::new();

  let context = CommandContext {
    search,
    value: value_signal,
    selected_item_id,
    filtered_count,
    filtered_items,
    filtered_groups,
    all_items,
    all_groups,
    item_values,
    should_filter: should_filter.unwrap_or(true),
    loop_navigation,
    disable_pointer_selection,
    vim_bindings: vim_bindings.unwrap_or(true),
    list_id: list_id.clone(),
    label_id: label_id.clone(),
    input_id: input_id.clone(),
    list_inner_ref,
  };

  provide_context(context.clone());

  let _context = context.clone();
  Effect::new(move |_| {
    let _ = search.get();
    filter_items(&_context);
    sort_items(&_context);
    select_first_item(&_context);
  });

  if let Some(cb) = on_value_change {
    Effect::new(move |_| {
      let val = value_signal.get();
      cb.run(val);
    });
  }

  let handle_keydown = move |ev: KeyboardEvent| {
    if ev.default_prevented() || ev.is_composing() || ev.key_code() == 229 {
      return;
    }

    let key = ev.key();
    match key.as_str() {
      "n" | "j" if context.vim_bindings && ev.ctrl_key() => {
        ev.prevent_default();
        update_selected_by_item(&context, 1);
      }
      "ArrowDown" => {
        ev.prevent_default();
        if ev.meta_key() {
          update_selected_to_last(&context);
        } else if ev.alt_key() {
          update_selected_by_group(&context, 1);
        } else {
          update_selected_by_item(&context, 1);
        }
      }
      "p" | "k" if context.vim_bindings && ev.ctrl_key() => {
        ev.prevent_default();
        update_selected_by_item(&context, -1);
      }
      "ArrowUp" => {
        ev.prevent_default();
        if ev.meta_key() {
          update_selected_to_index(&context, 0);
        } else if ev.alt_key() {
          update_selected_by_group(&context, -1);
        } else {
          update_selected_by_item(&context, -1);
        }
      }
      "Home" => {
        ev.prevent_default();
        update_selected_to_index(&context, 0);
      }
      "End" => {
        ev.prevent_default();
        update_selected_to_last(&context);
      }
      "Enter" => {
        ev.prevent_default();
        if let Some(item) = get_selected_item(&context) {
          let _ = item.dispatch_event(&web_sys::Event::new("cmdk-item-select").unwrap());
        }
      }
      _ => {}
    }
  };

  view! {
    <div
      data-slot="command"
      class=cn(
        &[
          "bg-popover text-popover-foreground flex h-full w-full flex-col overflow-hidden rounded-md",
          class.as_str(),
        ],
      )
      on:keydown=handle_keydown
      tabindex="-1"
    >
      <label id=label_id for=input_id.clone() class="sr-only">
        {label}
      </label>
      {children()}
    </div>
  }
}

#[component]
pub fn CommandInput(
  #[prop(optional)] on_value_change: Option<Callback<String>>,
  #[prop(optional, into)] placeholder: String,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let context = use_context::<CommandContext>().expect("CommandInput must be used within Command");
  let placeholder = StoredValue::new_local(placeholder);

  let handle_input = move |ev: leptos::ev::Event| {
    let target = event_target_value(&ev);
    context.search.set(target.clone());
    if let Some(cb) = on_value_change {
      cb.run(target);
    }
  };

  view! {
    <AttributeInterceptor let:attrs>
      <div data-slot="command-input-wrapper" class="flex h-9 items-center gap-2 border-b px-3">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="size-4 shrink-0 opacity-50"
        >
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.3-4.3" />
        </svg>
        <input
          {..attrs}
          data-slot="command-input"
          id=context.input_id.clone()
          type="text"
          autocomplete="off"
          spellcheck="false"
          aria-autocomplete="list"
          role="combobox"
          aria-expanded="true"
          aria-controls=context.list_id.clone()
          aria-labelledby=context.label_id.clone()
          aria-activedescendant=move || context.selected_item_id.get()
          class=cn(
            &[
              "placeholder:text-muted-foreground flex h-10 w-full rounded-md bg-transparent py-3 text-sm outline-hidden disabled:cursor-not-allowed disabled:opacity-50",
              class.as_str(),
            ],
          )
          placeholder=placeholder.get_value()
          bind:value=context.search
          on:input=handle_input
        />
      </div>
    </AttributeInterceptor>
  }
}

#[component]
pub fn CommandList(
  #[prop(optional, into)] label: String,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let context = use_context::<CommandContext>().expect("CommandList must be used within Command");
  let label = if label.is_empty() {
    "Suggestions".to_string()
  } else {
    label
  };

  view! {
    <div
      data-slot="command-list"
      id=context.list_id.clone()
      role="listbox"
      tabindex="-1"
      aria-activedescendant=move || context.selected_item_id.get()
      aria-label=label
      class=cn(&["max-h-[300px] scroll-py-1 overflow-x-hidden overflow-y-auto", class.as_str()])
    >
      <div node_ref=context.list_inner_ref>{children()}</div>
    </div>
  }
}

#[component]
pub fn CommandEmpty(#[prop(optional, into)] class: String, children: ChildrenFn) -> impl IntoView {
  let context = use_context::<CommandContext>().expect("CommandEmpty must be used within Command");
  let children = StoredValue::new_local(children);

  let render = Memo::new(move |_| context.filtered_count.get() == 0);

  view! {
    <Show when=move || render.get()>
      <div
        data-slot="command-empty"
        role="presentation"
        class=cn(&["py-6 text-center text-sm", class.as_str()])
      >
        {children.get_value()()}
      </div>
    </Show>
  }
}

#[component]
pub fn CommandGroup(
  #[prop(optional, into)] heading: String,
  #[prop(optional, into)] value: String,
  #[prop(optional)] force_mount: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let context = use_context::<CommandContext>().expect("CommandGroup must be used within Command");
  let heading = StoredValue::new_local(heading);

  let id = if !value.is_empty() {
    value
  } else if !heading.read_value().is_empty() {
    heading.get_value()
  } else {
    uuid::Uuid::new_v4().to_string()
  };

  let heading_id = StoredValue::new_local(uuid::Uuid::new_v4().to_string());

  // Регистрация группы
  Effect::new({
    let id = id.clone();
    let context = context.clone();
    move |_| {
      context.all_groups.update(|groups| {
        groups.entry(id.clone()).or_insert_with(HashSet::new);
      });

      on_cleanup({
        let id = id.clone();
        let context = context.clone();
        move || {
          context.all_groups.update(|groups| {
            groups.remove(&id);
          });
        }
      });
    }
  });

  let render = Memo::new({
    let id = id.clone();
    move |_| {
      if force_mount {
        return true;
      }
      if !context.should_filter {
        return true;
      }
      if context.search.get().is_empty() {
        return true;
      }
      context.filtered_groups.get().contains(&id)
    }
  });

  let group_context = GroupContext {
    id: id.clone(),
    force_mount,
  };

  provide_context(group_context);

  view! {
    <div
      data-slot="command-group"
      data-value=id
      role="presentation"
      hidden=move || !render.get()
      class=cn(
        &[
          "text-foreground [&_[cmdk-group-heading]]:text-muted-foreground overflow-hidden p-1 [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium",
          class.as_str(),
        ],
      )
    >
      <Show when=move || !heading.read_value().is_empty()>
        <div cmdk-group-heading="" aria-hidden="true" id=heading_id.get_value()>
          {heading.get_value()}
        </div>
      </Show>
      <div
        cmdk-group-items=""
        role="group"
        aria-labelledby=if !heading.read_value().is_empty() {
          Some(heading_id.get_value())
        } else {
          None
        }
      >
        {children()}
      </div>
    </div>
  }
}

#[component]
pub fn CommandItem(
  #[prop(optional, into)] value: String,
  #[prop(optional)] keywords: Vec<String>,
  #[prop(optional)] disabled: bool,
  #[prop(optional)] on_select: Option<Callback<String>>,
  #[prop(optional)] force_mount: bool,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<CommandContext>().expect("CommandItem must be used within Command");
  let group_context = use_context::<GroupContext>();
  let children = StoredValue::new_local(children);

  let id = uuid::Uuid::new_v4().to_string();
  let item_ref = NodeRef::<leptos::html::Div>::new();

  let computed_value = RwSignal::new(value.clone());
  let force_mount = force_mount || group_context.as_ref().map(|g| g.force_mount).unwrap_or(false);

  Effect::new(move |_| {
    if let Some(el) = item_ref.get() {
      let text_content = el.text_content().unwrap_or_default().trim().to_string();
      if !text_content.is_empty() && computed_value.get().is_empty() {
        computed_value.set(text_content);
      }
    }
  });

  Effect::new({
    let id = id.clone();
    let context = context.clone();
    move |_| {
      let val = computed_value.get();
      if val.is_empty() {
        return;
      }

      context.all_items.update(|items| {
        items.insert(id.clone());
      });

      if let Some(group) = group_context.as_ref() {
        context.all_groups.update(|groups| {
          groups
            .entry(group.id.clone())
            .or_insert_with(HashSet::new)
            .insert(id.clone());
        });
      }

      context.item_values.update(|values| {
        values.insert(
          id.clone(),
          ItemValue {
            value: val.clone(),
            keywords: keywords.clone(),
          },
        );
      });

      let score = calculate_score(&val, &context.search.get(), &keywords);
      context.filtered_items.update(|items| {
        items.insert(id.clone(), score);
      });

      on_cleanup({
        let id = id.clone();
        let context = context.clone();
        move || {
          context.all_items.update(|items| {
            items.remove(&id);
          });
          context.item_values.update(|values| {
            values.remove(&id);
          });
          context.filtered_items.update(|items| {
            items.remove(&id);
          });
        }
      });
    }
  });

  let selected = Memo::new(move |_| context.value.get() == computed_value.get());

  let render = Memo::new({
    let id = id.clone();
    move |_| {
      if force_mount {
        return true;
      }
      if !context.should_filter {
        return true;
      }
      if context.search.get().is_empty() {
        return true;
      }
      context.filtered_items.get().get(&id).copied().unwrap_or(0.0) > 0.0
    }
  });

  let handle_select = StoredValue::new_local(move |_| {
    if disabled {
      return;
    }
    let val = computed_value.get();
    context.value.set(val.clone());
    if let Some(cb) = on_select {
      cb.run(val);
    }
  });

  let handle_pointer_move = move |_| {
    if disabled || context.disable_pointer_selection {
      return;
    }
    context.value.set(computed_value.get());
  };

  Effect::new(move |_| {
    if let Some(el) = item_ref.get() {
      let closure = leptos::wasm_bindgen::closure::Closure::wrap(Box::new({
        let handle_select = handle_select.get_value();
        move |_: web_sys::Event| {
          handle_select(());
        }
      }) as Box<dyn Fn(web_sys::Event)>);

      let _ = el.add_event_listener_with_callback("cmdk-item-select", closure.as_ref().unchecked_ref());
      closure.forget();
    }
  });

  view! {
    <Show when=move || render.get()>
      <div
        node_ref=item_ref
        id=id.clone()
        data-slot="command-item"
        role="option"
        aria-disabled=disabled
        aria-selected=move || selected.get()
        data-disabled=disabled
        data-selected=move || selected.get()
        data-value=move || computed_value.get()
        class=cn(
          &[
            "data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground [&_svg:not([class*='text-'])]:text-muted-foreground relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
            class.as_str(),
          ],
        )
        on:pointermove=handle_pointer_move
        on:click=move |_| handle_select.get_value()(())
      >
        {children.get_value()()}
      </div>
    </Show>
  }
}

#[component]
pub fn CommandSeparator(#[prop(optional)] always_render: bool, #[prop(optional, into)] class: String) -> impl IntoView {
  let context = use_context::<CommandContext>().expect("CommandSeparator must be used within Command");

  let render = Memo::new(move |_| always_render || context.search.get().is_empty());

  view! {
    <Show when=move || render.get()>
      <div
        data-slot="command-separator"
        role="separator"
        class=cn(&["bg-border -mx-1 h-px", class.as_str()])
      />
    </Show>
  }
}

#[component]
pub fn CommandShortcut(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <span
      data-slot="command-shortcut"
      class=cn(&["text-muted-foreground ml-auto text-xs tracking-widest", class.as_str()])
    >
      {children()}
    </span>
  }
}

fn calculate_score(value: &str, search: &str, keywords: &[String]) -> f64 {
  if search.is_empty() {
    return 1.0;
  }

  let value_lower = value.to_lowercase();
  let search_lower = search.to_lowercase();

  if value_lower.contains(&search_lower) {
    let mut score: f64;

    if value_lower == search_lower {
      score = 1.0;
    } else if value_lower.starts_with(&search_lower) {
      score = 0.9;
    } else {
      score = 0.7;
    }

    for keyword in keywords {
      if keyword.to_lowercase().contains(&search_lower) {
        score = score.max(0.8);
      }
    }

    score
  } else {
    0.0
  }
}

fn filter_items(context: &CommandContext) {
  if !context.should_filter || context.search.get().is_empty() {
    context.filtered_count.set(context.all_items.get().len());
    return;
  }

  context.filtered_groups.set(HashSet::new());
  let mut item_count = 0;

  let search = context.search.get();
  let item_values = context.item_values.get();
  let mut filtered = HashMap::new();

  for id in context.all_items.get().iter() {
    if let Some(item_value) = item_values.get(id) {
      let score = calculate_score(&item_value.value, &search, &item_value.keywords);
      filtered.insert(id.clone(), score);
      if score > 0.0 {
        item_count += 1;
      }
    }
  }

  context.filtered_items.set(filtered.clone());

  let all_groups = context.all_groups.get();
  let mut visible_groups = HashSet::new();

  for (group_id, items) in all_groups.iter() {
    for item_id in items {
      if filtered.get(item_id).copied().unwrap_or(0.0) > 0.0 {
        visible_groups.insert(group_id.clone());
        break;
      }
    }
  }

  context.filtered_groups.set(visible_groups);
  context.filtered_count.set(item_count);
}

#[allow(clippy::needless_return)]
fn sort_items(context: &CommandContext) {
  if !context.should_filter || context.search.get().is_empty() {
    return;
  }

  // Сортировка выполняется через DOM манипуляции
  // В реальности нужна более сложная логика
}

fn get_selected_item(context: &CommandContext) -> Option<Element> {
  if let Some(list) = context.list_inner_ref.get() {
    let list_el: &Element = list.as_ref();
    list_el
      .query_selector("[data-slot='command-item'][aria-selected='true']")
      .ok()
      .flatten()
  } else {
    None
  }
}

fn get_valid_items(context: &CommandContext) -> Vec<Element> {
  if let Some(list) = context.list_inner_ref.get() {
    let list_el: &Element = list.as_ref();
    let node_list = list_el
      .query_selector_all("[data-slot='command-item']:not([aria-disabled='true'])")
      .ok();

    if let Some(nodes) = node_list {
      (0..nodes.length())
        .filter_map(|i| nodes.get(i).and_then(|n| n.dyn_into::<Element>().ok()))
        .collect()
    } else {
      vec![]
    }
  } else {
    vec![]
  }
}

fn select_first_item(context: &CommandContext) {
  let items = get_valid_items(context);
  if let Some(item) = items.first()
    && let Some(value) = item.get_attribute("data-value")
  {
    context.value.set(value);
  }
}

fn update_selected_to_index(context: &CommandContext, index: usize) {
  let items = get_valid_items(context);
  if let Some(item) = items.get(index)
    && let Some(value) = item.get_attribute("data-value")
  {
    context.value.set(value);
  }
}

fn update_selected_to_last(context: &CommandContext) {
  let items = get_valid_items(context);
  if let Some(item) = items.last()
    && let Some(value) = item.get_attribute("data-value")
  {
    context.value.set(value);
  }
}

fn update_selected_by_item(context: &CommandContext, change: i32) {
  let items = get_valid_items(context);
  if items.is_empty() {
    return;
  }

  let selected = get_selected_item(context);
  let index = selected.and_then(|sel| items.iter().position(|item| item == &sel));

  let new_index = if let Some(idx) = index {
    let new_idx = idx as i32 + change;
    if context.loop_navigation {
      if new_idx < 0 {
        items.len() - 1
      } else if new_idx >= items.len() as i32 {
        0
      } else {
        new_idx as usize
      }
    } else {
      new_idx.max(0).min(items.len() as i32 - 1) as usize
    }
  } else {
    0
  };

  if let Some(item) = items.get(new_index)
    && let Some(value) = item.get_attribute("data-value")
  {
    context.value.set(value);
  }
}

fn update_selected_by_group(context: &CommandContext, change: i32) {
  // Упрощенная реализация - переход между группами
  update_selected_by_item(context, change);
}

#[derive(Clone)]
struct CommandContext {
  search: RwSignal<String>,
  value: RwSignal<String>,
  selected_item_id: RwSignal<Option<String>>,
  filtered_count: RwSignal<usize>,
  filtered_items: RwSignal<HashMap<String, f64>>,
  filtered_groups: RwSignal<HashSet<String>>,
  all_items: RwSignal<HashSet<String>>,
  all_groups: RwSignal<HashMap<String, HashSet<String>>>,
  item_values: RwSignal<HashMap<String, ItemValue>>,
  should_filter: bool,
  loop_navigation: bool,
  disable_pointer_selection: bool,
  vim_bindings: bool,
  list_id: String,
  label_id: String,
  input_id: String,
  list_inner_ref: NodeRef<leptos::html::Div>,
}

#[derive(Clone)]
struct ItemValue {
  value: String,
  keywords: Vec<String>,
}

#[derive(Clone)]
struct GroupContext {
  id: String,
  force_mount: bool,
}
