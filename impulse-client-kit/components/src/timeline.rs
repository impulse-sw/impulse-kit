#![allow(missing_docs, dead_code)]

//! A timeline that lays its items out on a CSS grid, horizontally or
//! vertically, optionally alternating items on either side of the line.
//!
//! `Timeline` assigns each direct `TimelineItem` child its position
//! automatically (via a shared context counter), so you can just drop items in:
//!
//! ```ignore
//! view! {
//!   <Timeline orientation=TimelineOrientation::Vertical>
//!     <TimelineItem>
//!       <TimelineItemDate>"12 Mar 2024"</TimelineItemDate>
//!       <TimelineItemTitle>"Shipped"</TimelineItemTitle>
//!       <TimelineItemDescription>"First release went out."</TimelineItemDescription>
//!     </TimelineItem>
//!     <TimelineItem variant=TimelineVariant::Destructive>…</TimelineItem>
//!   </Timeline>
//! }
//! ```

use impulse_client_kit::utils::cn;
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

/// Layout direction of the timeline.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineOrientation {
  #[default]
  Horizontal,
  Vertical,
}

/// Colour treatment of an item (card, dot and branch).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineVariant {
  #[default]
  Default,
  Secondary,
  Destructive,
  Outline,
}

impl TimelineVariant {
  /// Border colour for the centre dot.
  fn dot_border(&self) -> &'static str {
    match self {
      Self::Default => "border-primary",
      Self::Secondary => "border-secondary",
      Self::Destructive => "border-destructive",
      Self::Outline => "",
    }
  }

  /// Solid fill for the centre dot (when not hollow).
  fn dot_fill(&self) -> &'static str {
    match self {
      Self::Default => "bg-primary",
      Self::Secondary => "bg-secondary",
      Self::Destructive => "bg-destructive",
      Self::Outline => "bg-background",
    }
  }

  /// Card classes.
  fn card(&self) -> &'static str {
    match self {
      Self::Default => "bg-card border text-card-foreground shadow-sm",
      Self::Secondary => "bg-secondary text-secondary-foreground shadow-sm",
      Self::Destructive => "bg-destructive/10 border border-destructive/20 text-destructive-foreground shadow-sm",
      Self::Outline => "bg-transparent border shadow-sm",
    }
  }

  /// Branch (the short stub linking dot to card) colour.
  fn branch(&self) -> &'static str {
    match self {
      Self::Default => "bg-primary",
      Self::Secondary => "bg-secondary",
      Self::Destructive => "bg-destructive",
      Self::Outline => "bg-border",
    }
  }
}

/// Which side single-sided (`alternating = false`) timelines place content on.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineAlignment {
  /// Content above the line (horizontal) / left of it (vertical).
  #[default]
  TopLeft,
  /// Content below the line (horizontal) / right of it (vertical).
  BottomRight,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
  Before,
  After,
}

#[derive(Clone, Copy)]
struct TimelineContext {
  orientation: TimelineOrientation,
  total: RwSignal<usize>,
  card_width: u32,
  max_card_width: u32,
  alternating: bool,
  alignment: TimelineAlignment,
  no_cards: bool,
  // Hands each `TimelineItem` its position as the children render (synchronously,
  // read untracked), mirroring React's automatic index injection.
  counter: RwSignal<usize>,
}

fn use_timeline_context() -> TimelineContext {
  use_context::<TimelineContext>().expect("Timeline components must be used within a Timeline component.")
}

#[component]
pub fn Timeline(
  /// Layout direction.
  #[prop(optional)]
  orientation: TimelineOrientation,
  /// Place successive items on alternating sides of the line.
  #[prop(optional, default = true)]
  alternating: bool,
  /// Side used when `alternating` is false.
  #[prop(optional)]
  alignment: TimelineAlignment,
  /// Horizontal card width (px).
  #[prop(optional, default = 220)]
  horiz_item_width: u32,
  /// Horizontal spacing between items (px).
  #[prop(optional, default = 130)]
  horiz_item_spacing: u32,
  /// Vertical spacing between items (px).
  #[prop(optional, default = 130)]
  vert_item_spacing: u32,
  /// Max card width in vertical layout (px).
  #[prop(optional, default = 350)]
  vert_item_max_width: u32,
  /// Drop the card chrome, leaving just the content.
  #[prop(optional, default = false)]
  no_cards: bool,
  #[prop(into, optional)] class: String,
  children: ChildrenFragmentFn,
) -> impl IntoView {
  let is_vertical = orientation == TimelineOrientation::Vertical;
  let safe_padding = ((horiz_item_width as f64 - horiz_item_spacing as f64) / 2.0).max(0.0);

  let total = RwSignal::new(0usize);
  let counter = RwSignal::new(0usize);

  provide_context(TimelineContext {
    orientation,
    total,
    card_width: horiz_item_width,
    max_card_width: vert_item_max_width,
    alternating,
    alignment,
    no_cards,
    counter,
  });

  // Build the items now (so the context counter assigns each its index) and
  // publish the final count.
  let fragment = children();
  total.set(fragment.nodes.iter().len());
  let items = fragment.nodes;

  // Vertical layout pads the list so the first/last cards stay centred on their
  // dots. We measure card heights after layout (and on count changes).
  let list_ref = NodeRef::<leptos::html::Ul>::new();
  let pad_top = RwSignal::new(0.0f64);
  let pad_bottom = RwSignal::new(0.0f64);

  if is_vertical {
    let spacing = vert_item_spacing as f64;
    Effect::new(move |_| {
      // Re-run once the children (and their count) are in place.
      let _ = total.get();
      let Some(list) = list_ref.get() else { return };
      request_animation_frame(move || {
        let Ok(cards) = list.query_selector_all("[data-timeline-card=\"true\"]") else {
          return;
        };
        let len = cards.length();
        if len == 0 {
          return;
        }
        let height = |i: u32| {
          cards
            .item(i)
            .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
            .map(|el| el.get_bounding_client_rect().height())
            .unwrap_or(0.0)
        };
        pad_top.set(((height(0) - spacing) / 2.0).max(0.0));
        pad_bottom.set(((height(len - 1) - spacing) / 2.0).max(0.0));
      });
    });
  }

  let layout_class = if is_vertical {
    "grid-cols-[1fr_2rem_1fr] auto-rows-min"
  } else {
    "grid-flow-col grid-rows-[min-content_2rem_min-content]"
  };

  let grid_style = move || {
    if is_vertical {
      let cols = if alternating {
        "1fr 2rem 1fr"
      } else if alignment == TimelineAlignment::TopLeft {
        "1fr 2rem"
      } else {
        "2rem 1fr"
      };
      format!(
        "grid-auto-rows: {vert_item_spacing}px; grid-template-columns: {cols}; padding-top: {}px; padding-bottom: {}px;",
        pad_top.get(),
        pad_bottom.get()
      )
    } else {
      let rows = if alternating {
        "min-content 2rem min-content"
      } else if alignment == TimelineAlignment::TopLeft {
        "min-content 2rem"
      } else {
        "2rem min-content"
      };
      format!(
        "grid-auto-columns: {horiz_item_spacing}px; grid-template-rows: {rows}; padding-left: {safe_padding}px; padding-right: {safe_padding}px;"
      )
    }
  };

  view! {
    <div
      id="timeline-container"
      data-slot="timeline"
      class=cn(&["flex h-full w-full p-4", if is_vertical { "flex-col" } else { "flex-row" }, class.as_str()])
      role="list"
      aria-label="Timeline"
    >
      <ul node_ref=list_ref id="timeline-grid" class=cn(&["grid relative", layout_class]) style=grid_style>
        {items}
      </ul>
    </div>
  }
}

/// Computes the inline grid placement for an item's card (`grid_style`) and its
/// connecting line cell (`line_style`).
fn grid_and_line_styles(side: Side, index: usize, is_vertical: bool, alternating: bool) -> (String, String) {
  let row = index + 1;
  if is_vertical {
    if alternating {
      let col = if side == Side::Before { 1 } else { 3 };
      (
        format!("grid-column: {col}; grid-row: {row};"),
        format!("grid-column: 2; grid-row: {row}; height: 100%;"),
      )
    } else if side == Side::Before {
      (
        format!("grid-column: 1; grid-row: {row};"),
        format!("grid-column: 2; grid-row: {row}; height: 100%;"),
      )
    } else {
      (
        format!("grid-column: 2; grid-row: {row};"),
        format!("grid-column: 1; grid-row: {row}; height: 100%;"),
      )
    }
  } else if alternating {
    let item_row = if side == Side::Before { 1 } else { 3 };
    (
      format!("grid-column: {row}; grid-row: {item_row};"),
      format!("grid-column: {row}; grid-row: 2; width: 100%;"),
    )
  } else if side == Side::Before {
    (
      format!("grid-column: {row}; grid-row: 1;"),
      format!("grid-column: {row}; grid-row: 2; width: 100%;"),
    )
  } else {
    (
      format!("grid-column: {row}; grid-row: 2;"),
      format!("grid-column: {row}; grid-row: 1; width: 100%;"),
    )
  }
}

fn card_style(is_vertical: bool, card_width: u32, max_card_width: u32) -> String {
  if is_vertical {
    format!("max-width: {max_card_width}px;")
  } else {
    format!("width: {card_width}px; min-width: {card_width}px; max-width: {card_width}px;")
  }
}

fn branch_position(is_vertical: bool, is_even: bool, alternating: bool, alignment: TimelineAlignment) -> &'static str {
  let pick_start = if alternating {
    is_even
  } else {
    alignment == TimelineAlignment::TopLeft
  };
  if is_vertical {
    if pick_start {
      "h-px w-4 left-0"
    } else {
      "h-px w-4 right-0"
    }
  } else if pick_start {
    "w-px h-4 top-0"
  } else {
    "w-px h-4 bottom-0"
  }
}

fn container_class(is_vertical: bool, side: Side) -> String {
  let orient = if is_vertical {
    "h-full items-center"
  } else {
    "w-full justify-center"
  };
  let align = match (is_vertical, side) {
    (false, Side::Before) => "items-end",
    (false, Side::After) => "items-start",
    (true, Side::Before) => "justify-end",
    (true, Side::After) => "justify-start",
  };
  cn(&["flex relative snap-center", orient, align])
}

#[component]
pub fn TimelineItem(
  /// Colour treatment.
  #[prop(optional)]
  variant: TimelineVariant,
  /// Hollow (outlined) centre dot instead of a filled one.
  #[prop(optional, default = false)]
  hollow: bool,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  let ctx = use_timeline_context();
  let index = ctx.counter.get_untracked();
  ctx.counter.set(index + 1);

  let is_vertical = ctx.orientation == TimelineOrientation::Vertical;
  let is_even = index.is_multiple_of(2);

  let side = if ctx.alternating {
    if is_even { Side::Before } else { Side::After }
  } else if ctx.alignment == TimelineAlignment::TopLeft {
    Side::Before
  } else {
    Side::After
  };

  let (grid_style, line_style) = grid_and_line_styles(side, index, is_vertical, ctx.alternating);

  let dot_class = if hollow {
    cn(&[
      "relative h-4 w-4 rounded-full z-10 flex items-center justify-center border-2 bg-card",
      variant.dot_border(),
    ])
  } else {
    cn(&[
      "relative h-4 w-4 rounded-full z-10 flex items-center justify-center border-2",
      variant.dot_border(),
      variant.dot_fill(),
    ])
  };

  let card_cls = if ctx.no_cards {
    cn(&[
      "flex flex-col rounded-md transition-all p-4 shrink-0 border-none shadow-none bg-transparent",
      class.as_str(),
    ])
  } else {
    cn(&[
      "flex flex-col rounded-md transition-all p-4 shrink-0",
      variant.card(),
      class.as_str(),
    ])
  };

  let total = ctx.total;
  let line_class = move || {
    let total = total.get();
    let first = if index == 0 {
      if is_vertical {
        "rounded-t-full"
      } else {
        "rounded-l-full"
      }
    } else {
      ""
    };
    let last = if total > 0 && index == total - 1 {
      if is_vertical {
        "rounded-b-full"
      } else {
        "rounded-r-full"
      }
    } else {
      ""
    };
    cn(&[
      "absolute bg-muted",
      first,
      last,
      if is_vertical { "h-full w-1" } else { "w-full h-1" },
    ])
  };

  let branch_cls = cn(&[
    variant.branch(),
    branch_position(is_vertical, is_even, ctx.alternating, ctx.alignment),
  ]);

  view! {
    <li
      class=container_class(is_vertical, side)
      style=grid_style
      role="listitem"
      aria-posinset=(index + 1).to_string()
      aria-setsize=move || total.get().to_string()
    >
      <div
        class=card_cls
        style=card_style(is_vertical, ctx.card_width, ctx.max_card_width)
        data-timeline-card="true"
      >
        {children()}
      </div>
    </li>

    <li class="relative flex items-center justify-center" style=line_style>
      <div class=line_class aria-hidden="true"></div>
      <div class=branch_cls aria-hidden="true"></div>
      <div class=dot_class aria-hidden="true"></div>
    </li>
  }
}

#[component]
pub fn TimelineItemDate(
  /// Already-formatted date text.
  #[prop(into)]
  children: String,
  /// Optional machine-readable value for a wrapping `<time datetime>` element
  /// (e.g. an ISO-8601 string). When omitted a plain `<span>` is used.
  #[prop(into, optional)]
  datetime: Option<String>,
  #[prop(into, optional)] class: String,
) -> impl IntoView {
  let cls = cn(&["text-xs text-muted-foreground mb-1", class.as_str()]);
  match datetime {
    Some(dt) => view! {
      <span class=cls>
        <time datetime=dt>{children}</time>
      </span>
    }
    .into_any(),
    None => view! { <span class=cls>{children}</span> }.into_any(),
  }
}

#[component]
pub fn TimelineItemTitle(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! { <h3 class=cn(&["font-semibold", class.as_str()])>{children()}</h3> }
}

#[component]
pub fn TimelineItemDescription(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! { <p class=cn(&["text-sm text-muted-foreground mt-2", class.as_str()])>{children()}</p> }
}
