//! SVG-based chart blocks.
//!
//! Charts are rendered as plain SVG through Leptos `view!`, so every primitive
//! (bar, axis tick, gridline) is a real DOM node. That keeps the binary tiny,
//! makes the charts theme-aware via the `--chart-*` / `--color-*` CSS variables,
//! and — crucially for the interactive graphs planned on top of this module —
//! gives hit-testing and pointer events for free.
//!
//! The first chart is [`BarChart`], a column chart with axes, a grid, optional
//! value labels, a legend and a hover tooltip. Multiple series are drawn as
//! grouped columns.
//!
//! ```
//! use impulse_ui_kit_blocks::charts::{BarChartData, BarSeries};
//!
//! let data = BarChartData {
//!   categories: vec!["Jan".into(), "Feb".into(), "Mar".into()],
//!   series: vec![BarSeries::new("Revenue", vec![12.0, 19.0, 7.0])],
//! };
//! assert_eq!(data.categories.len(), 3);
//! ```

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

/// A single named data series — one column per category.
#[derive(Clone, Debug, PartialEq)]
pub struct BarSeries {
  /// Series name, shown in the legend and tooltip.
  pub name: String,
  /// One value per category. Missing trailing values are treated as `0`.
  pub values: Vec<f64>,
  /// Optional CSS color (e.g. `"var(--chart-2)"` or `"#ef4444"`). When `None`,
  /// a color is picked from the theme `--chart-1..5` palette by series index.
  pub color: Option<String>,
}

impl BarSeries {
  /// Create a series with the theme default color.
  pub fn new(name: impl Into<String>, values: Vec<f64>) -> Self {
    Self {
      name: name.into(),
      values,
      color: None,
    }
  }

  /// Set an explicit CSS color for this series.
  pub fn with_color(mut self, color: impl Into<String>) -> Self {
    self.color = Some(color.into());
    self
  }
}

/// Categories plus one or more [`BarSeries`] to plot against them.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BarChartData {
  /// X-axis category labels.
  pub categories: Vec<String>,
  /// Series drawn as grouped columns.
  pub series: Vec<BarSeries>,
}

/// Per-element Tailwind classes for a [`BarChart`].
///
/// SVG text is colored through Tailwind `fill-*` utilities and lines through
/// `stroke-*`, so the defaults reference the same theme tokens as the rest of
/// the kit. Override only what you need via `..Default::default()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarChartClasses {
  /// Axis tick and category labels (`<text>`).
  pub axis_label: String,
  /// Gridlines (`<line>`).
  pub grid_line: String,
  /// Value labels above the columns (`<text>`).
  pub value_label: String,
  /// Legend entry text.
  pub legend_label: String,
  /// Floating hover tooltip container.
  pub tooltip: String,
}

impl Default for BarChartClasses {
  fn default() -> Self {
    Self {
      axis_label: "fill-muted-foreground text-xs".into(),
      grid_line: "stroke-border/50".into(),
      value_label: "fill-foreground text-xs font-medium".into(),
      legend_label: "text-sm text-muted-foreground".into(),
      tooltip:
        "pointer-events-none absolute z-50 rounded-md border border-border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md"
          .into(),
    }
  }
}

/// Layout and behavior options for a [`BarChart`]. All sizes are in SVG user
/// units; the chart itself scales responsively to its container width.
#[derive(Clone, Debug, PartialEq)]
pub struct BarChartOptions {
  /// SVG viewBox width.
  pub width: f64,
  /// SVG viewBox height.
  pub height: f64,
  /// Plot-area insets (space for axes and labels).
  pub margin_top: f64,
  /// Plot-area inset on the right.
  pub margin_right: f64,
  /// Plot-area inset at the bottom (x-axis labels).
  pub margin_bottom: f64,
  /// Plot-area inset on the left (y-axis labels).
  pub margin_left: f64,
  /// Target number of y-axis ticks / gridlines.
  pub y_ticks: usize,
  /// Draw horizontal gridlines.
  pub show_grid: bool,
  /// Draw the numeric value above each column.
  pub show_values: bool,
  /// Show a tooltip on hover.
  pub show_tooltip: bool,
  /// Show the series legend above the chart.
  pub show_legend: bool,
  /// Decimal places used for value and tick labels.
  pub value_decimals: usize,
  /// Corner radius of the columns.
  pub corner_radius: f64,
  /// Fraction of a category slot left as gap between groups (0.0–1.0).
  pub group_padding: f64,
  /// Fraction of a column slot left as gap between columns in a group (0.0–1.0).
  pub bar_padding: f64,
}

impl Default for BarChartOptions {
  fn default() -> Self {
    Self {
      width: 640.0,
      height: 360.0,
      margin_top: 16.0,
      margin_right: 16.0,
      margin_bottom: 36.0,
      margin_left: 44.0,
      y_ticks: 5,
      show_grid: true,
      show_values: false,
      show_tooltip: true,
      show_legend: true,
      value_decimals: 0,
      corner_radius: 4.0,
      group_padding: 0.2,
      bar_padding: 0.15,
    }
  }
}

/// Reactive hover state shared by the columns and the tooltip.
#[derive(Clone, PartialEq)]
struct Hover {
  category_index: usize,
  series_index: usize,
  x: f64,
  y: f64,
  category: String,
  series: String,
  value: f64,
  color: String,
}

/// Resolve a series' color, falling back to the theme `--chart-*` palette.
fn series_color(series: &BarSeries, index: usize) -> String {
  series
    .color
    .clone()
    .unwrap_or_else(|| format!("var(--chart-{})", index % 5 + 1))
}

/// Format a value with the configured number of decimals.
fn fmt_value(value: f64, decimals: usize) -> String {
  format!("{value:.decimals$}")
}

/// Round `value` to a "nice" number (1/2/5 × 10ⁿ), as in axis-tick algorithms.
fn nice_num(value: f64, round: bool) -> f64 {
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

/// Compute a nice axis domain and step covering `[min, max]`.
fn nice_ticks(min: f64, max: f64, target: usize) -> (f64, f64, f64) {
  let range = nice_num(max - min, false);
  let step = nice_num(range / target.max(1) as f64, true).max(f64::MIN_POSITIVE);
  let nice_min = (min / step).floor() * step;
  let nice_max = (max / step).ceil() * step;
  (nice_min, nice_max, step)
}

/// A grouped column (bar) chart.
///
/// * `data` — categories and one or more series.
/// * `options` — layout and behavior (axes, grid, tooltip, legend, …).
/// * `classes` — per-element Tailwind overrides.
/// * `class` — extra classes for the wrapping container.
///
/// ```rust,ignore
/// use impulse_ui_kit_blocks::charts::{BarChart, BarChartData, BarSeries};
/// use leptos::prelude::*;
///
/// let data = BarChartData {
///   categories: vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
///   series: vec![
///     BarSeries::new("2024", vec![12.0, 19.0, 7.0, 15.0]),
///     BarSeries::new("2025", vec![16.0, 11.0, 21.0, 9.0]),
///   ],
/// };
///
/// view! { <BarChart data=data /> };
/// ```
#[component]
pub fn BarChart(
  data: BarChartData,
  #[prop(optional)] options: BarChartOptions,
  #[prop(optional)] classes: BarChartClasses,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let opts = options;
  let n_cat = data.categories.len();
  let n_series = data.series.len();

  // Empty state.
  if n_cat == 0 || n_series == 0 {
    return view! {
      <div class=cn(
        &["flex h-48 w-full items-center justify-center text-sm text-muted-foreground", class.as_str()],
      )>"No data to display"</div>
    }
    .into_any();
  }

  let container = NodeRef::<leptos::html::Div>::new();
  let hover = RwSignal::new(None::<Hover>);

  let left = opts.margin_left;
  let top = opts.margin_top;
  let inner_w = (opts.width - opts.margin_left - opts.margin_right).max(1.0);
  let inner_h = (opts.height - opts.margin_top - opts.margin_bottom).max(1.0);

  // Y domain — always include the zero baseline.
  let mut data_min = 0.0f64;
  let mut data_max = 0.0f64;
  for series in &data.series {
    for &v in &series.values {
      data_min = data_min.min(v);
      data_max = data_max.max(v);
    }
  }
  if (data_max - data_min).abs() < f64::EPSILON {
    data_max = data_min + 1.0;
  }
  let (nice_min, nice_max, step) = nice_ticks(data_min, data_max, opts.y_ticks);
  let span = (nice_max - nice_min).max(f64::MIN_POSITIVE);
  let y_of = move |v: f64| top + inner_h * (nice_max - v) / span;
  let baseline_y = y_of(0.0);

  // Geometry.
  let slot_w = inner_w / n_cat as f64;
  let group_w = slot_w * (1.0 - opts.group_padding);
  let group_offset = (slot_w - group_w) / 2.0;
  let bar_slot = group_w / n_series as f64;
  let bar_w = bar_slot * (1.0 - opts.bar_padding);
  let bar_offset = (bar_slot - bar_w) / 2.0;

  // Gridlines + y-axis tick labels.
  let mut grid = Vec::new();
  if opts.show_grid {
    let mut tick = nice_min;
    // Guard against any pathological step.
    let mut guard = 0;
    while tick <= nice_max + step * 0.5 && guard < 1000 {
      let y = y_of(tick);
      let label = fmt_value(tick, opts.value_decimals);
      grid.push(
        view! {
          <line
            x1=left
            x2=left + inner_w
            y1=y
            y2=y
            class=classes.grid_line.clone()
            stroke-width="1"
          />
          <text
            x=left - 8.0
            y=y + 4.0
            text-anchor="end"
            class=classes.axis_label.clone()
          >
            {label}
          </text>
        }
        .into_any(),
      );
      tick += step;
      guard += 1;
    }
  }

  // X-axis category labels.
  let x_labels = data
    .categories
    .iter()
    .enumerate()
    .map(|(ci, category)| {
      let x = left + ci as f64 * slot_w + slot_w / 2.0;
      view! {
        <text
          x=x
          y=top + inner_h + 20.0
          text-anchor="middle"
          class=classes.axis_label.clone()
        >
          {category.clone()}
        </text>
      }
      .into_any()
    })
    .collect_view();

  // Columns (+ optional value labels), with hover wiring.
  let mut bars = Vec::new();
  for (si, series) in data.series.iter().enumerate() {
    let color = series_color(series, si);
    for ci in 0..n_cat {
      let value = series.values.get(ci).copied().unwrap_or(0.0);
      let x = left + ci as f64 * slot_w + group_offset + si as f64 * bar_slot + bar_offset;
      let y_value = y_of(value);
      let (rect_y, rect_h) = if value >= 0.0 {
        (y_value, (baseline_y - y_value).max(0.0))
      } else {
        (baseline_y, (y_value - baseline_y).max(0.0))
      };

      let fill = color.clone();
      let opacity = move || match hover.get() {
        Some(h) if !(h.category_index == ci && h.series_index == si) => "0.45",
        _ => "1",
      };

      // Hover handlers build the tooltip payload and follow the cursor.
      let enter = {
        let category = data.categories[ci].clone();
        let series_name = series.name.clone();
        let color = color.clone();
        move |ev: web_sys::PointerEvent| {
          let (px, py) = pointer_pos(&container, &ev);
          hover.set(Some(Hover {
            category_index: ci,
            series_index: si,
            x: px,
            y: py,
            category: category.clone(),
            series: series_name.clone(),
            value,
            color: color.clone(),
          }));
        }
      };
      let moving = move |ev: web_sys::PointerEvent| {
        let (px, py) = pointer_pos(&container, &ev);
        hover.update(|h| {
          if let Some(h) = h.as_mut() {
            h.x = px;
            h.y = py;
          }
        });
      };
      let leave = move |_: web_sys::PointerEvent| hover.set(None);

      bars.push(
        view! {
          <rect
            x=x
            y=rect_y
            width=bar_w
            height=rect_h
            rx=opts.corner_radius
            fill=fill
            fill-opacity=opacity
            class="cursor-pointer transition-[fill-opacity] duration-150"
            on:pointerenter=enter
            on:pointermove=moving
            on:pointerleave=leave
          />
        }
        .into_any(),
      );

      if opts.show_values {
        let label = fmt_value(value, opts.value_decimals);
        let label_y = if value >= 0.0 {
          rect_y - 6.0
        } else {
          rect_y + rect_h + 14.0
        };
        bars.push(
          view! {
            <text
              x=x + bar_w / 2.0
              y=label_y
              text-anchor="middle"
              class=classes.value_label.clone()
            >
              {label}
            </text>
          }
          .into_any(),
        );
      }
    }
  }

  // Baseline (zero) axis.
  let axis = view! {
    <line
      x1=left
      x2=left + inner_w
      y1=baseline_y
      y2=baseline_y
      class="stroke-border"
      stroke-width="1"
    />
  };

  // Legend.
  let legend = (opts.show_legend && n_series > 0).then(|| {
    let entries = data
      .series
      .iter()
      .enumerate()
      .map(|(si, series)| {
        let color = series_color(series, si);
        view! {
          <div class="flex items-center gap-1.5">
            <span class="h-2.5 w-2.5 rounded-sm" style=format!("background-color:{color}") />
            <span class=classes.legend_label.clone()>{series.name.clone()}</span>
          </div>
        }
      })
      .collect_view();
    view! { <div class="mb-2 flex flex-wrap items-center gap-x-4 gap-y-1">{entries}</div> }
  });

  // Floating tooltip.
  let tooltip_class = classes.tooltip.clone();
  let tooltip = move || {
    let tooltip_class = tooltip_class.clone();
    hover.get().map(move |h| {
      let value = fmt_value(h.value, opts.value_decimals);
      view! {
        <div class=tooltip_class.clone() style=format!("left:{}px;top:{}px", h.x + 12.0, h.y + 12.0)>
          <div class="mb-0.5 font-medium text-foreground">{h.category.clone()}</div>
          <div class="flex items-center gap-1.5">
            <span class="h-2 w-2 rounded-sm" style=format!("background-color:{}", h.color) />
            <span>{h.series.clone()}": "</span>
            <span class="font-medium text-foreground">{value}</span>
          </div>
        </div>
      }
    })
  };

  let show_tooltip = opts.show_tooltip;
  let view_box = format!("0 0 {} {}", opts.width, opts.height);

  view! {
    <div class=cn(&["w-full", class.as_str()])>
      {legend}
      <div node_ref=container class="relative w-full">
        <svg
          viewBox=view_box
          preserveAspectRatio="xMidYMid meet"
          role="img"
          class="h-auto w-full"
        >
          {grid}
          {axis}
          {bars}
          {x_labels}
        </svg>
        {move || if show_tooltip { tooltip().into_any() } else { ().into_any() }}
      </div>
    </div>
  }
  .into_any()
}

/// Cursor position relative to the chart container, for tooltip placement.
fn pointer_pos(container: &NodeRef<leptos::html::Div>, ev: &web_sys::PointerEvent) -> (f64, f64) {
  if let Some(el) = container.get_untracked() {
    let rect = el.get_bounding_client_rect();
    (ev.client_x() as f64 - rect.left(), ev.client_y() as f64 - rect.top())
  } else {
    (ev.client_x() as f64, ev.client_y() as f64)
  }
}
