#![allow(missing_docs, dead_code)]

//! Usage:
//!
//! web-sys = { version = "0.3.82", features = ["Element", "HtmlButtonElement", "HtmlDivElement"] }

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[component]
pub fn Carousel(
  #[prop(optional, into)] orientation: Signal<Orientation>,
  #[prop(optional, into)] class: String,
  #[prop(into, optional, default = 1)] items_per_view: usize,
  children: Children,
) -> impl IntoView {
  let current_index = RwSignal::new(0usize);
  let total_items = RwSignal::new(0usize);

  provide_context(CarouselContext {
    current_index,
    total_items,
    orientation,
    items_per_view,
  });

  let scroll_prev = move |_| {
    if current_index.get() > 0 {
      current_index.update(|i| *i -= 1);
    }
  };

  let scroll_next = move |_| {
    let max_index = total_items.get().saturating_sub(items_per_view);
    if current_index.get() < max_index {
      current_index.update(|i| *i += 1);
    }
  };

  let handle_keydown = move |ev: KeyboardEvent| match ev.key().as_str() {
    "ArrowLeft" => {
      ev.prevent_default();
      scroll_prev(());
    }
    "ArrowRight" => {
      ev.prevent_default();
      scroll_next(());
    }
    _ => {}
  };

  view! {
    <div
      class=cn(&["relative", class.as_str()])
      role="region"
      aria-label="carousel"
      on:keydown=handle_keydown
      tabindex="0"
    >
      {children()}
    </div>
  }
}

#[component]
pub fn CarouselContent(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<CarouselContext>().expect("CarouselContent must be used within Carousel");

  let container_ref = NodeRef::<leptos::html::Div>::new();

  Effect::new(move |_| {
    if let Some(container) = container_ref.get() {
      let slides = container.query_selector_all("[data-carousel-item]").unwrap();
      context.total_items.set(slides.length() as usize);
    }
  });

  let orientation = context.orientation.get();
  let flex_direction = match orientation {
    Orientation::Horizontal => "",
    Orientation::Vertical => "flex-col",
  };

  let margin = match orientation {
    Orientation::Horizontal => "-ml-4",
    Orientation::Vertical => "-mt-4",
  };

  view! {
    <div class="overflow-hidden">
      <div
        node_ref=container_ref
        class=cn(
          &["flex transition-transform duration-300", flex_direction, margin, class.as_str()],
        )
        style=move || {
          let offset = (context.current_index.get() as f64 / context.items_per_view as f64)
            * -100.0;
          match orientation {
            Orientation::Horizontal => format!("transform: translateX({}%)", offset),
            Orientation::Vertical => format!("transform: translateY({}%)", offset),
          }
        }
      >
        {children()}
      </div>
    </div>
  }
}

#[component]
pub fn CarouselItem(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<CarouselContext>().expect("CarouselItem must be used within Carousel");

  let orientation = context.orientation.get();
  let padding = match orientation {
    Orientation::Horizontal => "pl-4",
    Orientation::Vertical => "pt-4",
  };

  let basis = match context.items_per_view {
    1 => "basis-full",
    2 => "basis-1/2",
    3 => "basis-1/3",
    4 => "basis-1/4",
    5 => "basis-1/5",
    6 => "basis-1/6",
    _ => "basis-full",
  };

  view! {
    <div
      data-carousel-item
      role="group"
      aria-roledescription="slide"
      class=cn(&["min-w-0 shrink-0 grow-0", basis, padding, class.as_str()])
    >
      {children()}
    </div>
  }
}

#[component]
pub fn CarouselPrevious(#[prop(optional, into)] class: String) -> impl IntoView {
  let context = use_context::<CarouselContext>().expect("CarouselPrevious must be used within Carousel");

  let can_scroll_prev = move || context.current_index.get() > 0;

  let scroll_prev = move |_| {
    if can_scroll_prev() {
      context.current_index.update(|i| *i -= 1);
    }
  };

  let orientation = context.orientation.get();
  let position = match orientation {
    Orientation::Horizontal => "top-1/2 -left-12 -translate-y-1/2",
    Orientation::Vertical => "-top-12 left-1/2 -translate-x-1/2 rotate-90",
  };

  view! {
    <button
      class=cn(
        &[
          "absolute size-8 rounded-full border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:bg-input/30 dark:border-input dark:hover:bg-input/50 disabled:opacity-50",
          position,
          class.as_str(),
        ],
      )
      disabled=move || !can_scroll_prev()
      on:click=scroll_prev
      aria-label="Previous slide"
    >
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
        class="mx-auto"
      >
        <path d="m12 19-7-7 7-7" />
        <path d="M19 12H5" />
      </svg>
    </button>
  }
}

#[component]
pub fn CarouselNext(#[prop(optional, into)] class: String) -> impl IntoView {
  let context = use_context::<CarouselContext>().expect("CarouselNext must be used within Carousel");

  let can_scroll_next =
    move || context.current_index.get() < context.total_items.get().saturating_sub(context.items_per_view);

  let scroll_next = move |_| {
    if can_scroll_next() {
      context.current_index.update(|i| *i += 1);
    }
  };

  let orientation = context.orientation.get();
  let position = match orientation {
    Orientation::Horizontal => "top-1/2 -right-12 -translate-y-1/2",
    Orientation::Vertical => "-bottom-12 left-1/2 -translate-x-1/2 rotate-90",
  };

  view! {
    <button
      class=cn(
        &[
          "absolute size-8 rounded-full border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:bg-input/30 dark:border-input dark:hover:bg-input/50 disabled:opacity-50",
          position,
          class.as_str(),
        ],
      )
      disabled=move || !can_scroll_next()
      on:click=scroll_next
      aria-label="Next slide"
    >
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
        class="mx-auto"
      >
        <path d="M5 12h14" />
        <path d="m12 5 7 7-7 7" />
      </svg>
    </button>
  }
}

#[derive(Clone, Copy)]
pub struct CarouselContext {
  pub current_index: RwSignal<usize>,
  pub total_items: RwSignal<usize>,
  pub orientation: Signal<Orientation>,
  pub items_per_view: usize,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Orientation {
  #[default]
  Horizontal,
  Vertical,
}
