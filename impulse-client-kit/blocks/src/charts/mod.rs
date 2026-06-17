//! SVG-based chart blocks.
//!
//! Charts are rendered as plain SVG through Leptos `view!`, so every primitive
//! (bar, line point, pie slice, axis tick, gridline) is a real DOM node. That
//! keeps the binary tiny, makes the charts theme-aware via the `--chart-*` /
//! `--color-*` CSS variables, and — crucially for the interactive graphs planned
//! on top of this module — gives hit-testing and pointer events for free.
//!
//! Available charts:
//!
//! * [`BarChart`] — grouped or stacked column chart.
//! * [`LineChart`] — line / area chart, optionally stacked and smoothed.
//! * [`PieChart`] — pie / donut chart.
//! * [`Sparkline`] — compact, axis-less line or bar trend.
//!
//! ```
//! use impulse_client_kit_blocks::charts::{BarChartData, BarSeries};
//!
//! let data = BarChartData {
//!   categories: vec!["Jan".into(), "Feb".into(), "Mar".into()],
//!   series: vec![BarSeries::new("Revenue", vec![12.0, 19.0, 7.0])],
//! };
//! assert_eq!(data.categories.len(), 3);
//! ```

use leptos::prelude::*;

pub mod bar;
pub mod line;
pub mod pie;
pub mod sparkline;

pub use bar::*;
pub use line::*;
pub use pie::*;
pub use sparkline::*;

/// Default floating-tooltip container classes, shared by every chart.
pub(crate) const TOOLTIP_CLASS: &str = "pointer-events-none absolute z-50 rounded-md border border-border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md";

/// A color from the theme `--chart-1..5` palette, by index.
pub(crate) fn palette_color(index: usize) -> String {
  format!("var(--chart-{})", index % 5 + 1)
}

/// Resolve an optional explicit color, falling back to the theme palette.
pub(crate) fn resolve_color(color: &Option<String>, index: usize) -> String {
  color.clone().unwrap_or_else(|| palette_color(index))
}

/// Format a value with the configured number of decimals.
pub(crate) fn fmt_value(value: f64, decimals: usize) -> String {
  format!("{value:.decimals$}")
}

/// Round `value` to a "nice" number (1/2/5 × 10ⁿ), as in axis-tick algorithms.
pub(crate) fn nice_num(value: f64, round: bool) -> f64 {
  if value <= 0.0 {
    return 0.0;
  }
  let exponent = value.log10().floor();
  let fraction = value / 10f64.powf(exponent);
  let nice = if round {
    if fraction < 1.5 {
      1.0
    } else if fraction < 3.0 {
      2.0
    } else if fraction < 7.0 {
      5.0
    } else {
      10.0
    }
  } else if fraction <= 1.0 {
    1.0
  } else if fraction <= 2.0 {
    2.0
  } else if fraction <= 5.0 {
    5.0
  } else {
    10.0
  };
  nice * 10f64.powf(exponent)
}

/// The cartesian plot frame shared by [`BarChart`] and [`LineChart`]: the inner
/// plot rectangle plus a "nice" y-domain, with helpers to map data to pixels.
#[derive(Clone, Copy)]
pub(crate) struct Frame {
  pub left: f64,
  pub top: f64,
  pub inner_w: f64,
  pub inner_h: f64,
  pub nice_min: f64,
  pub nice_max: f64,
  pub step: f64,
}

impl Frame {
  /// Build a frame from the plot rectangle and a data domain `[min, max]`.
  pub fn new(left: f64, top: f64, inner_w: f64, inner_h: f64, min: f64, max: f64, target_ticks: usize) -> Self {
    let range = nice_num(max - min, false);
    let step = nice_num(range / target_ticks.max(1) as f64, true).max(f64::MIN_POSITIVE);
    let nice_min = (min / step).floor() * step;
    let nice_max = (max / step).ceil() * step;
    Self {
      left,
      top,
      inner_w: inner_w.max(1.0),
      inner_h: inner_h.max(1.0),
      nice_min,
      nice_max,
      step,
    }
  }

  /// Map a data value to its y pixel coordinate.
  pub fn y_of(&self, value: f64) -> f64 {
    let span = (self.nice_max - self.nice_min).max(f64::MIN_POSITIVE);
    self.top + self.inner_h * (self.nice_max - value) / span
  }

  /// The y coordinate of the zero baseline.
  pub fn baseline(&self) -> f64 {
    self.y_of(0.0)
  }

  /// Width of one category slot for `n` categories.
  pub fn slot_w(&self, n: usize) -> f64 {
    self.inner_w / n.max(1) as f64
  }

  /// Center x of category `index` of `n`.
  pub fn x_center(&self, index: usize, n: usize) -> f64 {
    self.left + (index as f64 + 0.5) * self.slot_w(n)
  }
}

/// Horizontal gridlines with y-axis tick labels.
pub(crate) fn grid_views(frame: &Frame, grid_class: &str, axis_class: &str, decimals: usize) -> Vec<AnyView> {
  let mut out = Vec::new();
  let mut tick = frame.nice_min;
  let mut guard = 0;
  while tick <= frame.nice_max + frame.step * 0.5 && guard < 1000 {
    let y = frame.y_of(tick);
    let label = fmt_value(tick, decimals);
    out.push(
      view! {
        <line
          x1=frame.left
          x2=frame.left + frame.inner_w
          y1=y
          y2=y
          class=grid_class.to_string()
          stroke-width="1"
        />
        <text x=frame.left - 8.0 y=y + 4.0 text-anchor="end" class=axis_class.to_string()>
          {label}
        </text>
      }
      .into_any(),
    );
    tick += frame.step;
    guard += 1;
  }
  out
}

/// Centered x-axis category labels.
pub(crate) fn x_label_views(frame: &Frame, categories: &[String], axis_class: &str) -> Vec<AnyView> {
  let n = categories.len();
  categories
    .iter()
    .enumerate()
    .map(|(ci, category)| {
      let x = frame.x_center(ci, n);
      view! {
        <text
          x=x
          y=frame.top + frame.inner_h + 20.0
          text-anchor="middle"
          class=axis_class.to_string()
        >
          {category.clone()}
        </text>
      }
      .into_any()
    })
    .collect()
}

/// The solid zero-baseline axis line.
pub(crate) fn baseline_view(frame: &Frame, class: &str) -> AnyView {
  let y = frame.baseline();
  view! {
    <line
      x1=frame.left
      x2=frame.left + frame.inner_w
      y1=y
      y2=y
      class=class.to_string()
      stroke-width="1"
    />
  }
  .into_any()
}

/// A swatch + label legend row.
pub(crate) fn legend_view(items: Vec<(String, String)>, label_class: &str) -> AnyView {
  let entries = items
    .into_iter()
    .map(|(color, label)| {
      view! {
        <div class="flex items-center gap-1.5">
          <span class="h-2.5 w-2.5 rounded-sm" style=format!("background-color:{color}") />
          <span class=label_class.to_string()>{label}</span>
        </div>
      }
    })
    .collect_view();
  view! { <div class="mb-2 flex flex-wrap items-center gap-x-4 gap-y-1">{entries}</div> }.into_any()
}

/// One row of a chart tooltip.
#[derive(Clone)]
pub(crate) struct TipRow {
  pub color: String,
  pub label: String,
  pub value: String,
}

/// Floating-tooltip payload: a position, a title and one or more rows.
pub(crate) struct Tip {
  pub x: f64,
  pub y: f64,
  pub title: String,
  pub rows: Vec<TipRow>,
}

/// Render a [`Tip`] as an absolutely positioned, cursor-following tooltip.
pub(crate) fn tooltip_view(class: String, tip: &Tip) -> AnyView {
  let rows = tip
    .rows
    .iter()
    .map(|row| {
      view! {
        <div class="flex items-center gap-1.5">
          <span class="h-2 w-2 rounded-sm" style=format!("background-color:{}", row.color) />
          <span>{row.label.clone()}": "</span>
          <span class="font-medium text-foreground">{row.value.clone()}</span>
        </div>
      }
    })
    .collect_view();
  view! {
    <div class=class style=format!("left:{}px;top:{}px", tip.x + 12.0, tip.y + 12.0)>
      <div class="mb-0.5 font-medium text-foreground">{tip.title.clone()}</div>
      {rows}
    </div>
  }
  .into_any()
}

/// Cursor position relative to a chart container element, for tooltip placement.
pub(crate) fn pointer_pos(container: &NodeRef<leptos::html::Div>, ev: &web_sys::PointerEvent) -> (f64, f64) {
  if let Some(el) = container.get_untracked() {
    let rect = el.get_bounding_client_rect();
    (ev.client_x() as f64 - rect.left(), ev.client_y() as f64 - rect.top())
  } else {
    (ev.client_x() as f64, ev.client_y() as f64)
  }
}
