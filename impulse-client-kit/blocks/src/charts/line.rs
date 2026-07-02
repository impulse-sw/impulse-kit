//! Line / area chart, optionally stacked and smoothed.

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use super::{
  Frame, TOOLTIP_CLASS, Tip, TipRow, baseline_view, fmt_value, grid_views, legend_view, pointer_pos, resolve_color,
  tooltip_view, x_label_views,
};

/// A single named data series — one point per category.
#[derive(Clone, Debug, PartialEq)]
pub struct LineSeries {
  /// Series name, shown in the legend and tooltip.
  pub name: String,
  /// One value per category. Missing trailing values are treated as `0`.
  pub values: Vec<f64>,
  /// Optional CSS color. When `None`, a color is picked from the theme palette.
  pub color: Option<String>,
}

impl LineSeries {
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

/// Categories plus one or more [`LineSeries`] to plot against them.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LineChartData {
  /// X-axis category labels.
  pub categories: Vec<String>,
  /// Series drawn as lines (optionally filled / stacked).
  pub series: Vec<LineSeries>,
}

/// Per-element Tailwind classes for a [`LineChart`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineChartClasses {
  /// Axis tick and category labels (`<text>`).
  pub axis_label: String,
  /// Gridlines (`<line>`).
  pub grid_line: String,
  /// Vertical hover guide line.
  pub guide_line: String,
  /// Legend entry text.
  pub legend_label: String,
  /// Floating hover tooltip container.
  pub tooltip: String,
}

impl Default for LineChartClasses {
  fn default() -> Self {
    Self {
      axis_label: "fill-muted-foreground text-xs".into(),
      grid_line: "stroke-border/50".into(),
      guide_line: "stroke-border".into(),
      legend_label: "text-sm text-muted-foreground".into(),
      tooltip: TOOLTIP_CLASS.into(),
    }
  }
}

/// Layout and behavior options for a [`LineChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct LineChartOptions {
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
  /// Fill the area under each line.
  pub area: bool,
  /// Opacity of the area fill (0.0–1.0).
  pub area_opacity: f64,
  /// Stack the series on top of each other (cumulative).
  pub stacked: bool,
  /// Smooth the lines with a Catmull-Rom spline.
  pub smooth: bool,
  /// Draw a marker dot at each data point.
  pub show_markers: bool,
  /// Draw horizontal gridlines.
  pub show_grid: bool,
  /// Show the series legend above the chart.
  pub show_legend: bool,
  /// Show a tooltip on hover.
  pub show_tooltip: bool,
  /// Decimal places used for value and tick labels.
  pub value_decimals: usize,
  /// Line stroke width.
  pub stroke_width: f64,
}

impl Default for LineChartOptions {
  fn default() -> Self {
    Self {
      width: 640.0,
      height: 360.0,
      margin_top: 16.0,
      margin_right: 16.0,
      margin_bottom: 36.0,
      margin_left: 44.0,
      y_ticks: 5,
      area: false,
      area_opacity: 0.15,
      stacked: false,
      smooth: false,
      show_markers: true,
      show_grid: true,
      show_legend: true,
      show_tooltip: true,
      value_decimals: 0,
      stroke_width: 2.0,
    }
  }
}

/// Build an SVG path through `points`, optionally as a Catmull-Rom spline.
fn line_path(points: &[(f64, f64)], smooth: bool) -> String {
  if points.is_empty() {
    return String::new();
  }
  let mut d = format!("M {} {}", points[0].0, points[0].1);
  if !smooth || points.len() < 3 {
    for p in &points[1..] {
      d.push_str(&format!(" L {} {}", p.0, p.1));
    }
    return d;
  }
  let n = points.len();
  for i in 0..n - 1 {
    let p0 = points[i.saturating_sub(1)];
    let p1 = points[i];
    let p2 = points[i + 1];
    let p3 = points[(i + 2).min(n - 1)];
    let c1x = p1.0 + (p2.0 - p0.0) / 6.0;
    let c1y = p1.1 + (p2.1 - p0.1) / 6.0;
    let c2x = p2.0 - (p3.0 - p1.0) / 6.0;
    let c2y = p2.1 - (p3.1 - p1.1) / 6.0;
    d.push_str(&format!(" C {c1x} {c1y} {c2x} {c2y} {} {}", p2.0, p2.1));
  }
  d
}

/// A line / area chart.
///
/// * `data` — categories and one or more series.
/// * `options` — area fill, stacking, smoothing, markers, axes, tooltip, ….
/// * `classes` — per-element Tailwind overrides.
/// * `class` — extra classes for the wrapping container.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::charts::{LineChart, LineChartData, LineChartOptions, LineSeries};
/// use leptos::prelude::*;
///
/// let data = LineChartData {
///   categories: vec!["Mon".into(), "Tue".into(), "Wed".into(), "Thu".into(), "Fri".into()],
///   series: vec![LineSeries::new("Visitors", vec![120.0, 180.0, 150.0, 210.0, 260.0])],
/// };
///
/// view! { <LineChart data=data options=LineChartOptions { area: true, ..Default::default() } /> };
/// ```
#[component]
pub fn LineChart(
  data: LineChartData,
  #[prop(optional)] options: LineChartOptions,
  #[prop(optional)] classes: LineChartClasses,
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
  let hovered = RwSignal::new(None::<usize>);
  let tip = RwSignal::new(None::<Tip>);

  let left = opts.margin_left;
  let top = opts.margin_top;
  let inner_w = opts.width - opts.margin_left - opts.margin_right;
  let inner_h = opts.height - opts.margin_top - opts.margin_bottom;

  // Cumulative tops per series when stacked; raw values otherwise.
  let mut tops: Vec<Vec<f64>> = Vec::with_capacity(n_series);
  let mut running = vec![0.0f64; n_cat];
  for series in &data.series {
    let mut row = Vec::with_capacity(n_cat);
    for (ci, run) in running.iter_mut().enumerate() {
      let v = series.values.get(ci).copied().unwrap_or(0.0);
      if opts.stacked {
        *run += v;
        row.push(*run);
      } else {
        row.push(v);
      }
    }
    tops.push(row);
  }

  // Y domain.
  let mut raw_min = f64::INFINITY;
  let mut raw_max = f64::NEG_INFINITY;
  for row in &tops {
    for &v in row {
      raw_min = raw_min.min(v);
      raw_max = raw_max.max(v);
    }
  }
  if !raw_min.is_finite() {
    raw_min = 0.0;
    raw_max = 1.0;
  }
  let (domain_min, mut domain_max) = if opts.area || opts.stacked {
    (raw_min.min(0.0), raw_max.max(0.0))
  } else {
    (raw_min, raw_max)
  };
  if (domain_max - domain_min).abs() < f64::EPSILON {
    domain_max = domain_min + 1.0;
  }

  let frame = Frame::new(left, top, inner_w, inner_h, domain_min, domain_max, opts.y_ticks);
  let base_y = frame.baseline();

  // Pixel points per series.
  let points: Vec<Vec<(f64, f64)>> = tops
    .iter()
    .map(|row| {
      row
        .iter()
        .enumerate()
        .map(|(ci, &v)| (frame.x_center(ci, n_cat), frame.y_of(v)))
        .collect()
    })
    .collect();

  // Series areas + lines, painted back-to-front so later series sit on top.
  let mut shapes = Vec::new();
  for (si, series) in data.series.iter().enumerate() {
    let color = resolve_color(&series.color, si);
    let pts = &points[si];

    if opts.area {
      let mut d = line_path(pts, opts.smooth);
      if opts.stacked && si > 0 {
        // Close along the previous layer's top, reversed.
        for p in points[si - 1].iter().rev() {
          d.push_str(&format!(" L {} {}", p.0, p.1));
        }
      } else {
        let last = pts[pts.len() - 1];
        let first = pts[0];
        d.push_str(&format!(" L {} {} L {} {}", last.0, base_y, first.0, base_y));
      }
      d.push_str(" Z");
      shapes.push(view! { <path d=d fill=color.clone() fill-opacity=opts.area_opacity stroke="none" /> }.into_any());
    }

    shapes.push(
      view! {
        <path
          d=line_path(pts, opts.smooth)
          fill="none"
          stroke=color.clone()
          stroke-width=opts.stroke_width
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      }
      .into_any(),
    );

    if opts.show_markers {
      for (ci, &(x, y)) in pts.iter().enumerate() {
        let color = color.clone();
        let r = move || if hovered.get() == Some(ci) { "4.5" } else { "3" };
        shapes.push(
          view! {
            <circle
              cx=x
              cy=y
              r=r
              fill=color
              stroke="var(--background)"
              stroke-width="1.5"
              class="transition-all duration-150"
            />
          }
          .into_any(),
        );
      }
    }
  }

  // Pre-compute tooltip rows for each category (all series at that x).
  let rows_per_cat: Vec<Vec<TipRow>> = (0..n_cat)
    .map(|ci| {
      data
        .series
        .iter()
        .enumerate()
        .map(|(si, series)| TipRow {
          color: resolve_color(&series.color, si),
          label: series.name.clone(),
          value: fmt_value(series.values.get(ci).copied().unwrap_or(0.0), opts.value_decimals),
        })
        .collect()
    })
    .collect();

  // Vertical hover guide.
  let guide_class = classes.guide_line.clone();
  let guide = move || {
    hovered.get().map(|ci| {
      let x = frame.x_center(ci, n_cat);
      view! {
        <line
          x1=x
          x2=x
          y1=frame.top
          y2=frame.top + frame.inner_h
          class=guide_class.clone()
          stroke-width="1"
        />
      }
    })
  };

  // Transparent per-category hit areas drive the hover state.
  let slot_w = frame.slot_w(n_cat);
  let mut hit_areas = Vec::new();
  for (ci, rows) in rows_per_cat.iter().enumerate() {
    let category = data.categories[ci].clone();
    let rows = rows.clone();
    let enter = {
      let category = category.clone();
      let rows = rows.clone();
      move |ev: web_sys::PointerEvent| {
        let (px, py) = pointer_pos(&container, &ev);
        hovered.set(Some(ci));
        tip.set(Some(Tip {
          x: px,
          y: py,
          title: category.clone(),
          rows: rows.clone(),
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
    hit_areas.push(
      view! {
        <rect
          x=left + ci as f64 * slot_w
          y=top
          width=slot_w
          height=inner_h
          fill="transparent"
          on:pointerenter=enter
          on:pointermove=moving
          on:pointerleave=leave
        />
      }
      .into_any(),
    );
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
      &["w-full min-w-0", class.as_str()],
    )>
      {legend} <div node_ref=container class="relative w-full">
        <svg viewBox=view_box preserveAspectRatio="xMidYMid meet" role="img" class="h-auto w-full">
          {grid}
          {axis}
          {guide}
          {shapes}
          {hit_areas}
          {x_labels}
        </svg>
        {move || if show_tooltip { tooltip().into_any() } else { ().into_any() }}
      </div>
    </div>
  }
  .into_any()
}
