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

### `<BarChart>`

A grouped column (bar) chart rendered as plain **SVG**. Every primitive (column,
axis tick, gridline) is a real DOM node, so the chart is tiny in the bundle,
theme-aware through the `--chart-*` / `--color-*` CSS variables, and gets
hit-testing and pointer events for free — the foundation for the interactive
graphs planned on top of this module.

It comes with axes, a grid, an optional value label per column, a legend and a
hover tooltip. Multiple series are drawn as grouped columns and colored from the
theme `--chart-1..5` palette (or an explicit per-series color).

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
