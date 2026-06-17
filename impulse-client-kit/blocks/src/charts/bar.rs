//! Grouped / stacked column (bar) chart.

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use super::{
  Frame, TOOLTIP_CLASS, Tip, TipRow, baseline_view, fmt_value, grid_views, legend_view, pointer_pos, resolve_color,
  tooltip_view, x_label_views,
};

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
  /// Series drawn as grouped or stacked columns.
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
  /// Value labels on the columns (`<text>`).
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
      tooltip: TOOLTIP_CLASS.into(),
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
  /// Plot-area inset at the top.
  pub margin_top: f64,
  /// Plot-area inset on the right.
  pub margin_right: f64,
  /// Plot-area inset at the bottom (x-axis labels).
  pub margin_bottom: f64,
  /// Plot-area inset on the left (y-axis labels).
  pub margin_left: f64,
  /// Target number of y-axis ticks / gridlines.
  pub y_ticks: usize,
  /// Stack the series on top of each other instead of grouping them.
  pub stacked: bool,
  /// Draw horizontal gridlines.
  pub show_grid: bool,
  /// Draw the numeric value on each column.
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
      stacked: false,
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

/// A grouped or stacked column (bar) chart.
///
/// * `data` — categories and one or more series.
/// * `options` — layout and behavior (stacking, axes, grid, tooltip, legend, …).
/// * `classes` — per-element Tailwind overrides.
/// * `class` — extra classes for the wrapping container.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::charts::{BarChart, BarChartData, BarSeries};
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

  if n_cat == 0 || n_series == 0 {
    return view! {
      <div class=cn(
        &[
          "flex h-48 w-full items-center justify-center text-sm text-muted-foreground",
          class.as_str(),
        ],
      )>"No data to display"</div>
    }
    .into_any();
  }

  let container = NodeRef::<leptos::html::Div>::new();
  // Hovered (category, series) for dimming, and the floating tooltip payload.
  let hovered = RwSignal::new(None::<(usize, usize)>);
  let tip = RwSignal::new(None::<Tip>);

  let left = opts.margin_left;
  let top = opts.margin_top;
  let inner_w = opts.width - opts.margin_left - opts.margin_right;
  let inner_h = opts.height - opts.margin_top - opts.margin_bottom;

  // Y domain — always includes zero. Stacked charts accumulate per category.
  let mut data_min = 0.0f64;
  let mut data_max = 0.0f64;
  if opts.stacked {
    for ci in 0..n_cat {
      let mut pos = 0.0;
      let mut neg = 0.0;
      for series in &data.series {
        let v = series.values.get(ci).copied().unwrap_or(0.0);
        if v >= 0.0 { pos += v } else { neg += v }
      }
      data_max = data_max.max(pos);
      data_min = data_min.min(neg);
    }
  } else {
    for series in &data.series {
      for &v in &series.values {
        data_min = data_min.min(v);
        data_max = data_max.max(v);
      }
    }
  }
  if (data_max - data_min).abs() < f64::EPSILON {
    data_max = data_min + 1.0;
  }

  let frame = Frame::new(left, top, inner_w, inner_h, data_min, data_max, opts.y_ticks);
  let baseline_y = frame.baseline();
  let slot_w = frame.slot_w(n_cat);

  // Geometry of a category's group of columns.
  let group_w = slot_w * (1.0 - opts.group_padding);
  let group_offset = (slot_w - group_w) / 2.0;
  // In stacked mode there is a single column per category spanning the group.
  let cols_per_group = if opts.stacked { 1 } else { n_series };
  let col_slot = group_w / cols_per_group as f64;
  let bar_w = col_slot * (1.0 - opts.bar_padding);
  let bar_offset = (col_slot - bar_w) / 2.0;
  let decimals = opts.value_decimals;

  // Columns (+ optional value labels).
  let mut bars = Vec::new();
  for ci in 0..n_cat {
    let mut pos_off = 0.0f64;
    let mut neg_off = 0.0f64;
    for (si, series) in data.series.iter().enumerate() {
      let value = series.values.get(ci).copied().unwrap_or(0.0);
      let color = resolve_color(&series.color, si);

      let (x, rect_y, rect_h) = if opts.stacked {
        let x = left + ci as f64 * slot_w + group_offset + bar_offset;
        let (y_top, y_bot) = if value >= 0.0 {
          let r = (frame.y_of(pos_off + value), frame.y_of(pos_off));
          pos_off += value;
          r
        } else {
          let r = (frame.y_of(neg_off), frame.y_of(neg_off + value));
          neg_off += value;
          r
        };
        (x, y_top.min(y_bot), (y_top - y_bot).abs())
      } else {
        let x = left + ci as f64 * slot_w + group_offset + si as f64 * col_slot + bar_offset;
        let yv = frame.y_of(value);
        if value >= 0.0 {
          (x, yv, (baseline_y - yv).max(0.0))
        } else {
          (x, baseline_y, (yv - baseline_y).max(0.0))
        }
      };

      let fill = color.clone();
      let opacity = move || match hovered.get() {
        Some((hc, hs)) if !(hc == ci && hs == si) => "0.45",
        _ => "1",
      };

      // Hover wiring.
      let enter = {
        let category = data.categories[ci].clone();
        let name = series.name.clone();
        let color = color.clone();
        move |ev: web_sys::PointerEvent| {
          let (px, py) = pointer_pos(&container, &ev);
          hovered.set(Some((ci, si)));
          tip.set(Some(Tip {
            x: px,
            y: py,
            title: category.clone(),
            rows: vec![TipRow {
              color: color.clone(),
              label: name.clone(),
              value: fmt_value(value, decimals),
            }],
          }));
        }
      };
      let moving = move |ev: web_sys::PointerEvent| {
        let (px, py) = pointer_pos(&container, &ev);
        tip.update(|t| {
          if let Some(t) = t.as_mut() {
            t.x = px;
            t.y = py;
          }
        });
      };
      let leave = move |_: web_sys::PointerEvent| {
        hovered.set(None);
        tip.set(None);
      };

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

      if opts.show_values && rect_h > 0.0 {
        let label = fmt_value(value, opts.value_decimals);
        let label_y = if opts.stacked {
          rect_y + rect_h / 2.0 + 4.0
        } else if value >= 0.0 {
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

  let grid = if opts.show_grid {
    grid_views(&frame, &classes.grid_line, &classes.axis_label, opts.value_decimals)
  } else {
    Vec::new()
  };
  let x_labels = x_label_views(&frame, &data.categories, &classes.axis_label);
  let axis = baseline_view(&frame, "stroke-border");

  let legend = (opts.show_legend && n_series > 0).then(|| {
    let items = data
      .series
      .iter()
      .enumerate()
      .map(|(si, series)| (resolve_color(&series.color, si), series.name.clone()))
      .collect();
    legend_view(items, &classes.legend_label)
  });

  let tooltip_class = classes.tooltip.clone();
  let tooltip = move || tip.with(|t| t.as_ref().map(|t| tooltip_view(tooltip_class.clone(), t)));

  let show_tooltip = opts.show_tooltip;
  let view_box = format!("0 0 {} {}", opts.width, opts.height);

  view! {
    <div class=cn(
      &["w-full", class.as_str()],
    )>
      {legend} <div node_ref=container class="relative w-full">
        <svg viewBox=view_box preserveAspectRatio="xMidYMid meet" role="img" class="h-auto w-full">
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
