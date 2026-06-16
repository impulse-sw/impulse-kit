//! Compact, axis-less line or bar trend — for tables, cards and KPIs.

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

use super::palette_color;

/// Whether a [`Sparkline`] is drawn as a line or as bars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SparklineKind {
  /// A polyline trend, optionally area-filled.
  #[default]
  Line,
  /// Small vertical bars.
  Bar,
}

/// Layout and styling for a [`Sparkline`].
#[derive(Clone, Debug, PartialEq)]
pub struct SparklineOptions {
  /// SVG viewBox width.
  pub width: f64,
  /// SVG viewBox height.
  pub height: f64,
  /// Line or bar rendering.
  pub kind: SparklineKind,
  /// Optional CSS color. Defaults to the theme `--chart-1`.
  pub color: Option<String>,
  /// Line stroke width (line kind).
  pub stroke_width: f64,
  /// Fill the area under the line (line kind).
  pub area: bool,
  /// Opacity of the area fill (0.0–1.0).
  pub area_opacity: f64,
  /// Draw a dot on the last point (line kind).
  pub show_last_dot: bool,
  /// Fraction of a bar slot left as gap (bar kind, 0.0–1.0).
  pub bar_gap: f64,
}

impl Default for SparklineOptions {
  fn default() -> Self {
    Self {
      width: 120.0,
      height: 32.0,
      kind: SparklineKind::Line,
      color: None,
      stroke_width: 1.5,
      area: false,
      area_opacity: 0.15,
      show_last_dot: true,
      bar_gap: 0.25,
    }
  }
}

/// A compact, axis-less trend chart.
///
/// * `data` — the values to plot.
/// * `options` — line/bar kind, color, area fill, ….
/// * `class` — extra classes for the wrapping container.
///
/// ```rust,ignore
/// use impulse_ui_kit_blocks::charts::{Sparkline, SparklineOptions};
/// use leptos::prelude::*;
///
/// view! {
///   <Sparkline data=vec![3.0, 5.0, 2.0, 8.0, 6.0, 9.0, 7.0] />
///   <Sparkline data=vec![3.0, 5.0, 2.0, 8.0] options=SparklineOptions { area: true, ..Default::default() } />
/// }
/// ```
#[component]
pub fn Sparkline(
  data: Vec<f64>,
  #[prop(optional)] options: SparklineOptions,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let opts = options;
  let color = opts.color.clone().unwrap_or_else(|| palette_color(0));

  if data.is_empty() {
    return view! { <span class=class></span> }.into_any();
  }

  let pad = 2.0;
  let mut data_min = data.iter().copied().fold(f64::INFINITY, f64::min);
  let mut data_max = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
  // Bars grow from a zero baseline; lines use the data's own range.
  if matches!(opts.kind, SparklineKind::Bar) {
    data_min = data_min.min(0.0);
    data_max = data_max.max(0.0);
  }
  if (data_max - data_min).abs() < f64::EPSILON {
    data_max = data_min + 1.0;
  }
  let span = data_max - data_min;
  let plot_h = opts.height - 2.0 * pad;
  let y_of = move |v: f64| opts.height - pad - (v - data_min) / span * plot_h;

  let body = match opts.kind {
    SparklineKind::Line => {
      let n = data.len();
      let plot_w = opts.width - 2.0 * pad;
      let x_of = move |i: usize| {
        if n <= 1 {
          opts.width / 2.0
        } else {
          pad + i as f64 * plot_w / (n - 1) as f64
        }
      };
      let points: Vec<(f64, f64)> = data.iter().enumerate().map(|(i, &v)| (x_of(i), y_of(v))).collect();

      let mut line = format!("M {} {}", points[0].0, points[0].1);
      for p in &points[1..] {
        line.push_str(&format!(" L {} {}", p.0, p.1));
      }

      let area = opts.area.then(|| {
        let last = points[points.len() - 1];
        let first = points[0];
        let base = opts.height - pad;
        let d = format!("{line} L {} {base} L {} {base} Z", last.0, first.0);
        view! { <path d=d fill=color.clone() fill-opacity=opts.area_opacity stroke="none" /> }
      });

      let dot = (opts.show_last_dot).then(|| {
        let last = points[points.len() - 1];
        view! { <circle cx=last.0 cy=last.1 r=opts.stroke_width + 1.0 fill=color.clone() /> }
      });

      view! {
        {area}
        <path
          d=line
          fill="none"
          stroke=color.clone()
          stroke-width=opts.stroke_width
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        {dot}
      }
      .into_any()
    }
    SparklineKind::Bar => {
      let n = data.len();
      let slot = opts.width / n.max(1) as f64;
      let bar_w = slot * (1.0 - opts.bar_gap);
      let base = y_of(0.0);
      let bars = data
        .iter()
        .enumerate()
        .map(|(i, &v)| {
          let x = i as f64 * slot + (slot - bar_w) / 2.0;
          let yv = y_of(v);
          let (y, h) = if yv <= base { (yv, base - yv) } else { (base, yv - base) };
          view! { <rect x=x y=y width=bar_w height=h rx="1" fill=color.clone() /> }
        })
        .collect_view();
      view! { {bars} }.into_any()
    }
  };

  let view_box = format!("0 0 {} {}", opts.width, opts.height);
  view! {
    <svg
      viewBox=view_box
      preserveAspectRatio="none"
      role="img"
      class=cn(&["inline-block align-middle", class.as_str()])
      style=format!("width:{}px;height:{}px", opts.width, opts.height)
    >
      {body}
    </svg>
  }
  .into_any()
}
