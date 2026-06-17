//! Pie / donut chart.

use std::f64::consts::PI;

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use super::{TOOLTIP_CLASS, Tip, TipRow, fmt_value, legend_view, pointer_pos, resolve_color, tooltip_view};

/// A single slice of a [`PieChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct PieSlice {
  /// Slice label, shown in the legend and tooltip.
  pub label: String,
  /// Slice value. Non-positive values are ignored.
  pub value: f64,
  /// Optional CSS color. When `None`, a color is picked from the theme palette.
  pub color: Option<String>,
}

impl PieSlice {
  /// Create a slice with the theme default color.
  pub fn new(label: impl Into<String>, value: f64) -> Self {
    Self {
      label: label.into(),
      value,
      color: None,
    }
  }

  /// Set an explicit CSS color for this slice.
  pub fn with_color(mut self, color: impl Into<String>) -> Self {
    self.color = Some(color.into());
    self
  }
}

/// The slices to plot.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PieChartData {
  /// Slices, drawn clockwise from the start angle.
  pub slices: Vec<PieSlice>,
}

/// Per-element Tailwind classes for a [`PieChart`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PieChartClasses {
  /// Percentage labels drawn on the slices (`<text>`).
  pub slice_label: String,
  /// Legend entry text.
  pub legend_label: String,
  /// Floating hover tooltip container.
  pub tooltip: String,
}

impl Default for PieChartClasses {
  fn default() -> Self {
    Self {
      slice_label: "fill-white text-xs font-medium".into(),
      legend_label: "text-sm text-muted-foreground".into(),
      tooltip: TOOLTIP_CLASS.into(),
    }
  }
}

/// Layout and behavior options for a [`PieChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct PieChartOptions {
  /// SVG viewBox width.
  pub width: f64,
  /// SVG viewBox height.
  pub height: f64,
  /// Render a donut (hollow center) instead of a full pie.
  pub donut: bool,
  /// Inner radius as a fraction of the outer radius, for donuts (0.0–1.0).
  pub inner_radius_ratio: f64,
  /// Gap between slices, in radians.
  pub pad_angle: f64,
  /// Start angle, in radians (`-PI/2` = 12 o'clock).
  pub start_angle: f64,
  /// Draw the percentage on each slice.
  pub show_labels: bool,
  /// Hide slices whose share is below this fraction from the labels (0.0–1.0).
  pub min_label_fraction: f64,
  /// Show the legend above the chart.
  pub show_legend: bool,
  /// Show a tooltip on hover.
  pub show_tooltip: bool,
  /// Decimal places used for the value in the tooltip.
  pub value_decimals: usize,
}

impl Default for PieChartOptions {
  fn default() -> Self {
    Self {
      width: 360.0,
      height: 300.0,
      donut: false,
      inner_radius_ratio: 0.6,
      pad_angle: 0.0,
      start_angle: -PI / 2.0,
      show_labels: true,
      min_label_fraction: 0.05,
      show_legend: true,
      show_tooltip: true,
      value_decimals: 0,
    }
  }
}

/// A point on a circle of radius `r` around `(cx, cy)` at `angle` (radians).
fn polar(cx: f64, cy: f64, r: f64, angle: f64) -> (f64, f64) {
  (cx + r * angle.cos(), cy + r * angle.sin())
}

/// Build the path for one ring/pie segment between `a0` and `a1`.
fn slice_path(cx: f64, cy: f64, r_outer: f64, r_inner: f64, a0: f64, a1: f64) -> String {
  let sweep = (a1 - a0).clamp(0.0, 2.0 * PI - 1e-4);
  let a1 = a0 + sweep;
  let large = if sweep > PI { 1 } else { 0 };
  let (x0o, y0o) = polar(cx, cy, r_outer, a0);
  let (x1o, y1o) = polar(cx, cy, r_outer, a1);
  if r_inner > 0.0 {
    let (x1i, y1i) = polar(cx, cy, r_inner, a1);
    let (x0i, y0i) = polar(cx, cy, r_inner, a0);
    format!(
      "M {x0o} {y0o} A {r_outer} {r_outer} 0 {large} 1 {x1o} {y1o} L {x1i} {y1i} A {r_inner} {r_inner} 0 {large} 0 {x0i} {y0i} Z"
    )
  } else {
    format!("M {cx} {cy} L {x0o} {y0o} A {r_outer} {r_outer} 0 {large} 1 {x1o} {y1o} Z")
  }
}

/// A pie / donut chart.
///
/// * `data` — the slices to plot.
/// * `options` — donut mode, labels, legend, tooltip, ….
/// * `classes` — per-element Tailwind overrides.
/// * `class` — extra classes for the wrapping container.
/// * `children` — optional content rendered in the donut center (KPI, total, …).
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::charts::{PieChart, PieChartData, PieChartOptions, PieSlice};
/// use leptos::prelude::*;
///
/// let data = PieChartData {
///   slices: vec![
///     PieSlice::new("Chrome", 64.0),
///     PieSlice::new("Safari", 19.0),
///     PieSlice::new("Firefox", 9.0),
///     PieSlice::new("Other", 8.0),
///   ],
/// };
///
/// view! { <PieChart data=data options=PieChartOptions { donut: true, ..Default::default() } /> };
/// ```
#[component]
pub fn PieChart(
  data: PieChartData,
  #[prop(optional)] options: PieChartOptions,
  #[prop(optional)] classes: PieChartClasses,
  #[prop(optional, into)] class: String,
  #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
  let opts = options;
  let slices: Vec<&PieSlice> = data.slices.iter().filter(|s| s.value > 0.0).collect();
  let total: f64 = slices.iter().map(|s| s.value).sum();

  if slices.is_empty() || total <= 0.0 {
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

  let cx = opts.width / 2.0;
  let cy = opts.height / 2.0;
  let r_outer = (opts.width.min(opts.height) / 2.0 - 8.0).max(1.0);
  let r_inner = if opts.donut {
    r_outer * opts.inner_radius_ratio.clamp(0.0, 0.95)
  } else {
    0.0
  };

  // We index back into the original slice list for legend/tooltip colors.
  let mut segments = Vec::new();
  let mut labels = Vec::new();
  let mut angle = opts.start_angle;
  for (i, slice) in slices.iter().enumerate() {
    let fraction = slice.value / total;
    let sweep = fraction * 2.0 * PI;
    let a0 = angle + opts.pad_angle / 2.0;
    let a1 = angle + sweep - opts.pad_angle / 2.0;
    angle += sweep;

    let color = resolve_color(&slice.color, i);
    let path = slice_path(cx, cy, r_outer, r_inner, a0, a1.max(a0));

    let opacity = move || match hovered.get() {
      Some(h) if h != i => "0.5",
      _ => "1",
    };
    let enter = {
      let label = slice.label.clone();
      let color = color.clone();
      let value = slice.value;
      let percent = format!("{:.0}%", fraction * 100.0);
      let decimals = opts.value_decimals;
      move |ev: web_sys::PointerEvent| {
        let (px, py) = pointer_pos(&container, &ev);
        hovered.set(Some(i));
        tip.set(Some(Tip {
          x: px,
          y: py,
          title: label.clone(),
          rows: vec![TipRow {
            color: color.clone(),
            label: percent.clone(),
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

    segments.push(
      view! {
        <path
          d=path
          fill=color.clone()
          fill-opacity=opacity
          class="cursor-pointer transition-[fill-opacity] duration-150"
          on:pointerenter=enter
          on:pointermove=moving
          on:pointerleave=leave
        />
      }
      .into_any(),
    );

    if opts.show_labels && fraction >= opts.min_label_fraction {
      let mid = (a0 + a1) / 2.0;
      let label_r = if r_inner > 0.0 {
        (r_outer + r_inner) / 2.0
      } else {
        r_outer * 0.6
      };
      let (lx, ly) = polar(cx, cy, label_r, mid);
      let text = format!("{:.0}%", fraction * 100.0);
      labels.push(
        view! {
          <text x=lx y=ly + 4.0 text-anchor="middle" class=classes.slice_label.clone()>
            {text}
          </text>
        }
        .into_any(),
      );
    }
  }

  let legend = (opts.show_legend).then(|| {
    let items = slices
      .iter()
      .enumerate()
      .map(|(i, slice)| (resolve_color(&slice.color, i), slice.label.clone()))
      .collect();
    legend_view(items, &classes.legend_label)
  });

  // Optional donut-center content.
  let center = children.map(|children| {
    view! {
      <div class="pointer-events-none absolute inset-0 flex items-center justify-center text-center">
        {children()}
      </div>
    }
  });

  let tooltip_class = classes.tooltip.clone();
  let tooltip = move || tip.with(|t| t.as_ref().map(|t| tooltip_view(tooltip_class.clone(), t)));

  let show_tooltip = opts.show_tooltip;
  let view_box = format!("0 0 {} {}", opts.width, opts.height);

  view! {
    <div class=cn(
      &["w-full", class.as_str()],
    )>
      {legend}
      <div
        node_ref=container
        class="relative mx-auto w-full"
        style=format!("max-width:{}px", opts.width)
      >
        <svg viewBox=view_box preserveAspectRatio="xMidYMid meet" role="img" class="h-auto w-full">
          {segments}
          {labels}
        </svg>
        {center}
        {move || if show_tooltip { tooltip().into_any() } else { ().into_any() }}
      </div>
    </div>
  }
  .into_any()
}
