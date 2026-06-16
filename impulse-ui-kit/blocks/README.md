# Impulse UI Kit Blocks

Higher-level **blocks** for the Impulse UI Kit.

Where [`impulse-ui-kit-components`](../components) ships the low-level building
bricks (buttons, inputs, dialogs, …), this crate ships ready-made *blocks*:
small, self-contained widgets that solve a concrete task and are themselves
composed of those components — charts, graphs, markdown, and the like. Think of
them as pre-assembled blocks rather than individual bricks.

## Usage

```toml
[dependencies]
impulse-ui-kit-blocks = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.2.0" }
```

### Wiring up Tailwind

Like the components crate, blocks embed their Tailwind classes in Rust sources,
so the consuming project's Tailwind pass must be able to scan them. This crate's
`build.rs` publishes an aggregated scan file — its own sources **plus** the
forwarded `impulse-ui-kit-components` sources — as build-script metadata:

* `DEP_IMPULSE_UI_KIT_BLOCKS_STYLES` — the aggregated scan file, and
* `DEP_IMPULSE_UI_KIT_BLOCKS_SOURCE_DIR` — the raw `src` directory.

Because the bundle already folds in the upstream component classes, a consumer
only needs to wire up this single source. Add the helper as a build-dependency:

```toml
[build-dependencies]
impulse-tailwind-sources = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.2.0" }
```

```rust
// build.rs
use std::{env, path::Path};

fn main() {
  let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
  let partial = Path::new(&manifest_dir).join(".tailwind-sources.css");
  impulse_tailwind_sources::write_source_partial(partial, &["DEP_IMPULSE_UI_KIT_BLOCKS_STYLES"]);
}
```

…and import the generated partial from your `style.css`. See the
[components README](../components/README.md#wiring-up-tailwind) for the full
explanation of the pattern.

## Blocks

All charts are rendered as plain **SVG** through Leptos `view!`. Every primitive
(column, line point, pie slice, axis tick, gridline) is a real DOM node, so the
charts are tiny in the bundle, theme-aware through the `--chart-*` / `--color-*`
CSS variables, and get hit-testing and pointer events for free — the foundation
for the interactive graphs planned on top of this module. Series default to the
theme `--chart-1..5` palette, or an explicit per-series color.

### `<BarChart>`

A grouped or stacked column (bar) chart with axes, a grid, an optional value
label per column, a legend and a hover tooltip. Set `BarChartOptions.stacked` to
accumulate series per category instead of drawing them side by side.

**Props:**

- `data`: `BarChartData` — `categories` plus one or more `BarSeries`.
- `options`: `BarChartOptions` — sizes, margins, ticks, and toggles for grid,
  value labels, tooltip and legend.
- `classes`: `BarChartClasses` — per-element Tailwind overrides (axis labels via
  `fill-*`, gridlines via `stroke-*`, tooltip, …).
- `class`: `String` — additional classes for the wrapping container.

**Example:**

```rust
use impulse_ui_kit_blocks::charts::{BarChart, BarChartData, BarChartOptions, BarSeries};
use leptos::prelude::*;

let data = BarChartData {
  categories: vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
  series: vec![
    BarSeries::new("2024", vec![12.0, 19.0, 7.0, 15.0]),
    BarSeries::new("2025", vec![16.0, 11.0, 21.0, 9.0]).with_color("var(--chart-3)"),
  ],
};

view! {
  <BarChart
    data=data
    options=BarChartOptions { show_values: true, ..Default::default() }
  />
}
```

### `<LineChart>`

A line / area chart over categorical data. `LineChartOptions` toggles area fill
(`area`), stacking (`stacked`), Catmull-Rom smoothing (`smooth`) and point
markers (`show_markers`). Hovering snaps to the nearest category, drawing a guide
line and a tooltip listing every series at that x.

```rust
use impulse_ui_kit_blocks::charts::{LineChart, LineChartData, LineChartOptions, LineSeries};
use leptos::prelude::*;

let data = LineChartData {
  categories: vec!["Mon".into(), "Tue".into(), "Wed".into(), "Thu".into(), "Fri".into()],
  series: vec![LineSeries::new("Visitors", vec![120.0, 180.0, 150.0, 210.0, 260.0])],
};

view! {
  <LineChart
    data=data
    options=LineChartOptions { area: true, smooth: true, ..Default::default() }
  />
}
```

### `<PieChart>`

A pie or donut chart of category shares, with arc segments, a legend, percentage
labels and a hover tooltip. Set `PieChartOptions.donut` for a hollow center, and
pass `children` to render content (a KPI, the total, …) in the middle.

```rust
use impulse_ui_kit_blocks::charts::{PieChart, PieChartData, PieChartOptions, PieSlice};
use leptos::prelude::*;

let data = PieChartData {
  slices: vec![
    PieSlice::new("Chrome", 64.0),
    PieSlice::new("Safari", 19.0),
    PieSlice::new("Firefox", 9.0),
    PieSlice::new("Other", 8.0),
  ],
};

view! {
  <PieChart data=data options=PieChartOptions { donut: true, ..Default::default() }>
    <div class="text-2xl font-bold">"100%"</div>
  </PieChart>
}
```

### `<Sparkline>`

A compact, axis-less line or bar trend for tables, cards and KPIs.

```rust
use impulse_ui_kit_blocks::charts::{Sparkline, SparklineKind, SparklineOptions};
use leptos::prelude::*;

view! {
  <Sparkline data=vec![3.0, 5.0, 2.0, 8.0, 6.0, 9.0, 7.0] />
  <Sparkline
    data=vec![4.0, 6.0, 3.0, 7.0, 5.0, 8.0]
    options=SparklineOptions { kind: SparklineKind::Bar, ..Default::default() }
  />
}
```

### `<Markdown>`

Render a Markdown document — given either inline text or a URL to fetch an
`.md` file from — into styled HTML. GFM extensions (tables, strikethrough, task
lists, smart punctuation) are enabled.

Every Markdown element is rendered with a sensible default set of Tailwind
classes that follow the UI Kit theme tokens, and every one of them can be
overridden individually via `MarkdownClasses`.

**Props:**

- `source`: `MarkdownSource` — `MarkdownSource::inline(...)` for direct content,
  or `MarkdownSource::url(...)` to fetch an `.md` file at render time.
- `classes`: `MarkdownClasses` — per-element Tailwind overrides; defaults follow
  the UI Kit theme.
- `class`: `String` — additional classes for the wrapping container.

**Example:**

```rust
use impulse_ui_kit_blocks::markdown::{Markdown, MarkdownClasses, MarkdownSource};
use leptos::prelude::*;

view! {
  // Inline Markdown with default styles.
  <Markdown source=MarkdownSource::inline("# Title\n\nHello **world**.") />

  // Fetched from a URL, with a couple of element styles overridden.
  <Markdown
    source=MarkdownSource::url("/docs/readme.md")
    classes=MarkdownClasses {
      h1: "text-4xl font-black text-primary".into(),
      link: "text-blue-500 underline".into(),
      ..Default::default()
    }
  />
}
```

`render_markdown(input, &classes)` is also exposed if you want the rendered HTML
string without the component wrapper.

> [!NOTE]
> Like any Markdown renderer, the output is injected via `inner_html`, and raw
> HTML embedded in the source is passed through as-is. Only render Markdown you
> trust, or sanitize it upstream.
