# Impulse Client Kit Blocks

Higher-level **blocks** for the Impulse Client Kit.

Where [`impulse-client-kit-components`](../components) ships the low-level building
bricks (buttons, inputs, dialogs, …), this crate ships ready-made *blocks*:
small, self-contained widgets that solve a concrete task and are themselves
composed of those components. Think of them as pre-assembled blocks rather than
individual bricks.

Everything is rendered with plain SVG + HTML through Leptos `view!`, so the
blocks are tiny in the bundle, theme-aware through the kit's CSS variables, and
every element is a real, hit-testable DOM node.

## Contents

- [Installation](#installation)
- [Wiring up Tailwind](#wiring-up-tailwind)
- [`<Markdown>`](#markdown) — render Markdown (inline or fetched) to styled HTML
- [Charts](#charts) — [`<BarChart>`](#barchart), [`<LineChart>`](#linechart),
  [`<PieChart>`](#piechart), [`<Sparkline>`](#sparkline)
- [Graph](#graph) — interactive node editor (`<GraphCanvas>` & friends)
- [Landings](#landings) — marketing-page sections (`<Hero>`, `<FeatureGrid>`,
  `<Pricing>`, `<Faq>`, `<Footer>`, …)
- [Theming & customization](#theming--customization)

## Installation

```toml
[dependencies]
impulse-client-kit-blocks = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.4.10" }
leptos = "0.8"
```

The crate mirrors the kit's feature flags — `csr` (default), `hydrate`, `ssr` —
and forwards them to `impulse-client-kit` / `impulse-client-kit-components`.

### Wiring up Tailwind

Like the components crate, blocks embed their Tailwind classes in Rust sources,
so the consuming project's Tailwind pass must be able to scan them. This crate's
`build.rs` publishes an aggregated scan file — its own sources **plus** the
forwarded `impulse-client-kit-components` sources — as build-script metadata:

* `DEP_IMPULSE_CLIENT_KIT_BLOCKS_STYLES` — the aggregated scan file, and
* `DEP_IMPULSE_CLIENT_KIT_BLOCKS_SOURCE_DIR` — the raw `src` directory.

Because the bundle already folds in the upstream component classes, a consumer
only needs to wire up this single source. Add the helper as a build-dependency:

```toml
[build-dependencies]
impulse-tailwind-sources = { git = "https://github.com/impulse-sw/impulse-kit.git", tag = "1.4.10" }
```

```rust
// build.rs
use std::{env, path::Path};

fn main() {
  let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
  let partial = Path::new(&manifest_dir).join(".tailwind-sources.css");
  impulse_tailwind_sources::write_source_partial(partial, &["DEP_IMPULSE_CLIENT_KIT_BLOCKS_STYLES"]);
}
```

…and import the generated `.tailwind-sources.css` partial from your `style.css`,
next to `tailwindcss`. See the
[components README](../components/README.md#wiring-up-tailwind) for the full
explanation of the pattern.

> **Important:** Tailwind only emits the classes it can *see*. The blocks'
> **default** classes are string literals in this crate's sources, so the scan
> file above covers them. Any classes **you** pass as overrides (e.g. a custom
> `MarkdownClasses` or a node `class`) must live in *your* own scanned sources.

---

## `<Markdown>`

Render a Markdown document — given either inline text or a URL to fetch an
`.md` file from — into styled HTML. GFM extensions are enabled: tables,
strikethrough, task lists, smart punctuation and GitHub-style alerts.

Every Markdown element is rendered with a sensible default set of Tailwind
classes that follow the Client Kit theme tokens, and every one of them can be
overridden individually via `MarkdownClasses`.

```rust
use impulse_client_kit_blocks::markdown::{Markdown, MarkdownClasses, MarkdownSource};
use leptos::prelude::*;

view! {
  // Inline Markdown with the default styles.
  <Markdown source=MarkdownSource::inline("# Title\n\nHello **world**.") />

  // Fetched from a URL at render time (a spinner shows while loading).
  <Markdown source=MarkdownSource::url("/docs/readme.md") />
}
```

### Props

| Prop      | Type              | Description                                                        |
| --------- | ----------------- | ----------------------------------------------------------------- |
| `source`  | `MarkdownSource`  | `MarkdownSource::inline(text)` or `MarkdownSource::url(url)`.      |
| `classes` | `MarkdownClasses` | Per-element Tailwind overrides; defaults follow the theme.        |
| `class`   | `String`          | Extra classes for the wrapping container.                         |

`MarkdownSource` is an enum: `Inline(String)` or `Url(String)`, with the
`inline(..)` / `url(..)` constructors. For a `Url` source the fetch happens on
the client (CSR/hydrate); a spinner is shown while loading and a themed error
box on failure.

### Customizing element styles

`MarkdownClasses` has one `String` field per element. Construct via `Default` and
override only what you need:

```rust
use impulse_client_kit_blocks::markdown::MarkdownClasses;

let classes = MarkdownClasses {
  h1: "text-4xl font-black text-primary".into(),
  link: "text-blue-500 underline".into(),
  inline_code: "rounded bg-primary/10 px-1.5 py-0.5 font-mono text-primary".into(),
  ..Default::default()
};
```

Fields: `h1`–`h6`, `paragraph`, `link`, `emphasis`, `strong`, `strikethrough`,
`inline_code`, `code_block`, `blockquote`, `unordered_list`, `ordered_list`,
`list_item`, `image`, `horizontal_rule`, `table`, `table_head`, `table_row`,
`table_header_cell`, `table_cell`, plus one `AlertClasses` per alert kind:
`alert_note`, `alert_tip`, `alert_important`, `alert_warning`, `alert_caution`
(each with a `container` and `title` field).

`render_markdown(input: &str, classes: &MarkdownClasses) -> String` is also
exposed if you want the HTML string without the component wrapper.

### GitHub-style alerts

Blockquotes whose first line is an alert marker render as titled callouts —
exactly like on GitHub:

```markdown
> [!NOTE]
> Useful information that users should know, even when skimming.

> [!TIP]
> Helpful advice for doing things better or more easily.

> [!IMPORTANT]
> Key information users need to know to achieve their goal.

> [!WARNING]
> Urgent info that needs immediate user attention to avoid problems.

> [!CAUTION]
> Advises about risks or negative outcomes of certain actions.
```

Each kind is a `<div>` callout with a colored border, tinted background and an
icon + label title row, styled via the matching `alert_*` field of
`MarkdownClasses`.

> **Security:** the output is injected via `inner_html`, and raw HTML embedded in
> the source is passed through as-is. Only render Markdown you trust, or sanitize
> it upstream.

---

## Charts

All charts live in `impulse_client_kit_blocks::charts`. They share the same design:

- **SVG, scaling to the container width.** Sizes in the option structs are SVG
  user units; the chart scales responsively.
- **Theme colors.** Series default to the `--chart-1..5` palette; pass an
  explicit CSS color (`"var(--chart-3)"`, `"#ef4444"`, …) per series/slice.
- **Per-element classes.** Each chart has a `*Classes` struct. SVG text is
  colored via Tailwind `fill-*` utilities and lines via `stroke-*`, so the
  defaults reference the same tokens as the rest of the kit.
- **Hover tooltips** that follow the cursor, built to match the popover styling.

Each chart component takes `data`, optional `options`, optional `classes`, and a
`class` for the container. For reactive data, wrap the chart in a `move ||`
closure so it re-renders when your signal changes.

### `<BarChart>`

A grouped or stacked column chart with axes, a grid, optional value labels, a
legend and a hover tooltip.

```rust
use impulse_client_kit_blocks::charts::{BarChart, BarChartData, BarChartOptions, BarSeries};
use leptos::prelude::*;

let data = BarChartData {
  categories: vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
  series: vec![
    BarSeries::new("2024", vec![12.0, 19.0, 7.0, 15.0]),
    BarSeries::new("2025", vec![16.0, 11.0, 21.0, 9.0]).with_color("var(--chart-3)"),
  ],
};

view! {
  <BarChart data=data options=BarChartOptions { show_values: true, ..Default::default() } />
}
```

- `BarSeries { name, values: Vec<f64>, color: Option<String> }` — `new(name, values)`,
  `.with_color(css)`. Missing trailing values count as `0`.
- `BarChartData { categories: Vec<String>, series: Vec<BarSeries> }`.
- `BarChartOptions` (defaults): `width 640`, `height 360`, margins
  `top 16`/`right 16`/`bottom 36`/`left 44`, `y_ticks 5`, `stacked false`,
  `show_grid true`, `show_values false`, `show_tooltip true`, `show_legend true`,
  `value_decimals 0`, `corner_radius 4`, `group_padding 0.2`, `bar_padding 0.15`.
  Set `stacked: true` to accumulate series per category instead of grouping.
- `BarChartClasses`: `axis_label`, `grid_line`, `value_label`, `legend_label`,
  `tooltip`.

### `<LineChart>`

A line / area chart over categorical data. Hovering snaps to the nearest
category, drawing a guide line and a tooltip listing every series at that x.

```rust
use impulse_client_kit_blocks::charts::{LineChart, LineChartData, LineChartOptions, LineSeries};
use leptos::prelude::*;

let data = LineChartData {
  categories: vec!["Mon".into(), "Tue".into(), "Wed".into(), "Thu".into(), "Fri".into()],
  series: vec![LineSeries::new("Visitors", vec![120.0, 180.0, 150.0, 210.0, 260.0])],
};

view! {
  <LineChart data=data options=LineChartOptions { area: true, smooth: true, ..Default::default() } />
}
```

- `LineSeries { name, values, color }` — same constructors as `BarSeries`.
- `LineChartData { categories, series }`.
- `LineChartOptions` (defaults): `width 640`, `height 360`, margins as bar,
  `y_ticks 5`, `area false`, `area_opacity 0.15`, `stacked false`, `smooth false`
  (Catmull-Rom), `show_markers true`, `show_grid true`, `show_legend true`,
  `show_tooltip true`, `value_decimals 0`, `stroke_width 2`.
- `LineChartClasses`: `axis_label`, `grid_line`, `guide_line`, `legend_label`,
  `tooltip`.

### `<PieChart>`

A pie or donut chart of category shares, with arc segments, percentage labels, a
legend and a hover tooltip. Pass `children` to render content in the donut
center.

```rust
use impulse_client_kit_blocks::charts::{PieChart, PieChartData, PieChartOptions, PieSlice};
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

- `PieSlice { label, value, color }` — `new(label, value)`, `.with_color(css)`.
  Non-positive values are ignored.
- `PieChartData { slices: Vec<PieSlice> }`.
- `PieChartOptions` (defaults): `width 360`, `height 300`, `donut false`,
  `inner_radius_ratio 0.6`, `pad_angle 0.0`, `start_angle -π/2` (12 o'clock),
  `show_labels true`, `min_label_fraction 0.05`, `show_legend true`,
  `show_tooltip true`, `value_decimals 0`.
- `PieChartClasses`: `slice_label`, `legend_label`, `tooltip`.

### `<Sparkline>`

A compact, axis-less line or bar trend for tables, cards and KPIs.

```rust
use impulse_client_kit_blocks::charts::{Sparkline, SparklineKind, SparklineOptions};
use leptos::prelude::*;

view! {
  <Sparkline data=vec![3.0, 5.0, 2.0, 8.0, 6.0, 9.0, 7.0] />
  <Sparkline
    data=vec![3.0, 5.0, 2.0, 8.0]
    options=SparklineOptions { area: true, color: Some("var(--chart-2)".into()), ..Default::default() }
  />
  <Sparkline
    data=vec![4.0, 6.0, 3.0, 7.0, 5.0]
    options=SparklineOptions { kind: SparklineKind::Bar, ..Default::default() }
  />
}
```

- Component props: `data: Vec<f64>`, `options: SparklineOptions`, `class`.
- `SparklineKind`: `Line` (default) or `Bar`.
- `SparklineOptions` (defaults): `width 120`, `height 32`, `kind Line`,
  `color None` (→ `--chart-1`), `stroke_width 1.5`, `area false`,
  `area_opacity 0.15`, `show_last_dot true`, `bar_gap 0.25`.

---

## Graph

`impulse_client_kit_blocks::graph` is a small, interactive **node editor** (think a
mini Blender node tree / react-flow): draggable nodes with connectable,
type-safe ports, on a pannable & zoomable canvas.

```rust
use impulse_client_kit_blocks::graph::*;
use impulse_client_kit_components::button::*;
use leptos::prelude::*;

let edges = RwSignal::new(vec![GraphEdge::new("a", "out", "b", "in")]);

view! {
  <GraphCanvas edges=edges>
    <GraphNode id="a" x=40.0 y=40.0 variant=NodeVariant::Solid>
      <GraphNodeHeader>"Source"</GraphNodeHeader>
      <GraphNodeBody>
        <p class="text-muted-foreground">"Any content goes here."</p>
        <GraphPort id="out" side=PortSide::Right data_type="number" label="Value" />
      </GraphNodeBody>
    </GraphNode>

    <GraphNode id="b" x=320.0 y=120.0 variant=NodeVariant::Accent>
      <GraphNodeHeader>"Process"</GraphNodeHeader>
      <GraphNodeBody>
        <GraphPort id="in" side=PortSide::Left data_type="number" label="In" />
        <Button size=ButtonSize::Sm>"Run"</Button>
      </GraphNodeBody>
    </GraphNode>
  </GraphCanvas>
}
```

### Components

| Component          | Role                                                                       |
| ------------------ | -------------------------------------------------------------------------- |
| `GraphCanvas`      | The surface. Owns the reactive state (positions, edges, viewport, …).      |
| `GraphNode`        | A positioned, draggable node holding arbitrary content.                    |
| `GraphNodeHeader`  | The node's **drag handle** (the body stays interactive).                   |
| `GraphNodeBody`    | A padded content area (sockets align to its edges).                        |
| `GraphPort`        | A connector socket. Drag from it to wire up an edge.                       |

**`GraphCanvas` props**

- `positions: Option<RwSignal<HashMap<String, (f64, f64)>>>` — controlled map of
  `node_id -> (x, y)`. Read/mutate it programmatically; dragging updates it in
  place. Omit to let the canvas own it internally.
- `edges: Option<RwSignal<Vec<GraphEdge>>>` — controlled connection list.
  User-made connections are appended here; deletions are removed.
- `options: GraphCanvasOptions` — see below.
- `class: String`, `children: Children` (the nodes).

**`GraphNode` props**

- `id: String` (unique; referenced by edges and ports).
- `x: f64`, `y: f64` — initial position, used only if `positions` has no entry
  for this id yet.
- `variant: NodeVariant` — `Solid` (default), `Outline`, `Dashed`, `Accent`,
  `Ghost`. Or override the look entirely with `class`.
- `width: Option<f64>` — node width in px (default `192`).
- `class: String`, `children: Children`.

**`GraphPort` props**

- `id: String` (unique within its node).
- `side: PortSide` — `Left` / `Right` / `Top` / `Bottom`. Controls which border
  the socket sits on and how the edge curves.
- `kind: Option<PortKind>` — `Input` or `Output`. Defaults from the side:
  left/top → `Input`, right/bottom → `Output`.
- `data_type: Option<String>` — optional type tag, e.g. `data_type="number"`.
- `label: String`, `class: String` (the latter styles the socket dot).

### Options — `GraphCanvasOptions`

| Field         | Default | Description                                              |
| ------------- | ------- | -------------------------------------------------------- |
| `height`      | `480.0` | Canvas height in px.                                     |
| `show_grid`   | `true`  | Dotted background grid.                                  |
| `grid_size`   | `16.0`  | Grid step (background dots **and** snapping).            |
| `snap`        | `false` | Snap dragged nodes to the grid.                          |
| `deletable`   | `true`  | Show delete affordances (wire click, node ×).           |
| `pannable`    | `true`  | Drag the empty background to pan.                        |
| `zoomable`    | `true`  | Scroll to zoom toward the cursor.                        |
| `min_scale`   | `0.25`  | Minimum zoom.                                            |
| `max_scale`   | `2.5`   | Maximum zoom.                                            |
| `layout`      | `None`  | Run a `GraphLayout` once after mount (see below).        |

### Interactions

- **Move nodes** — drag a node by its `GraphNodeHeader`. Buttons/inputs in the
  body keep working because dragging is bound to the header only. With
  `snap: true` the position snaps to `grid_size`. The last node you touch is
  raised above the others (z-index).
- **Connect ports** — drag from a socket to another socket. Sockets sit *on* the
  node border, so wires leave cleanly; the canvas measures node boxes and
  **routes edges around** any node that would otherwise sit under the wire.
- **Type-safe connections** — a connection is accepted only between an `Output`
  and an `Input`, and if both ports declare a `data_type` the types must match
  (a port without a type is a wildcard). The edge is oriented output → input
  regardless of which end you started from. While dragging, compatible target
  sockets are highlighted and incompatible ones dimmed.
- **Delete** (when `deletable`) — click a wire to remove it; hover a node and
  click the **×** to delete it together with its ports and edges.
- **Pan & zoom** — drag the background to pan, scroll to zoom toward the cursor
  (clamped to `min_scale`/`max_scale`). The grid tracks the viewport.

### Controlled state

Owning `positions` and `edges` yourself makes the graph fully programmable:

```rust
use std::collections::HashMap;
use impulse_client_kit_blocks::graph::*;
use leptos::prelude::*;

let positions = RwSignal::new(HashMap::from([
  ("a".to_string(), (40.0, 40.0)),
  ("b".to_string(), (320.0, 120.0)),
]));
let edges = RwSignal::new(vec![GraphEdge::new("a", "out", "b", "in")]);

// Read the live graph, e.g. to serialize it:
let snapshot = move || (positions.get(), edges.get());

view! { <GraphCanvas positions=positions edges=edges>/* nodes */</GraphCanvas> }
```

### Auto layout

Set `GraphCanvasOptions.layout` to place nodes automatically, once, on the frame
after they mount (this overrides their initial `x`/`y`):

```rust
view! {
  <GraphCanvas
    edges=edges
    options=GraphCanvasOptions { layout: Some(GraphLayout::Hierarchical), ..Default::default() }
  >
    /* nodes — their x/y are ignored */
  </GraphCanvas>
}
```

- `GraphLayout::ForceDirected` — Fruchterman-Reingold; good for general graphs.
- `GraphLayout::Hierarchical` — layered left-to-right by longest-path depth; good
  for DAGs / pipelines.

### Data types

- `GraphEdge { from: (String, String), to: (String, String), color: Option<String> }`
  — `(node_id, port_id)` endpoints. `GraphEdge::new(from_node, from_port, to_node, to_port)`,
  `.with_color(css)`.
- `PortSide` — `Left` / `Right` / `Top` / `Bottom`.
- `PortKind` — `Input` / `Output`.
- `NodeVariant` — `Solid` / `Outline` / `Dashed` / `Accent` / `Ghost`.
- `GraphLayout` — `ForceDirected` / `Hierarchical`.

> Pan/zoom uses pointer + wheel events; the canvas sets `touch-action: none` so
> dragging works on touch devices too.

---

## Landings

`impulse_client_kit_blocks::landings` is a set of ready-made **landing-page
sections**: drop them in a column and you have a product page. They were
distilled from two real landings built on the kit (TaskBoard and Деплойер) and
generalised into data-driven, theme-aware blocks.

Every block takes plain data (`Vec<Feature>`, `Vec<PricingTier>`, …) instead of
markup, reads the kit's CSS variables so it follows light/dark mode, and is
self-contained — including the signature "blueprint grid + glow" backdrop, which
ships as `<GridBackdrop>` (inline-styled, no app-level CSS needed).

```rust
use impulse_client_kit_blocks::landings::*;
use leptos::prelude::*;

view! {
  <Navbar
    brand="Деплойер"
    logo_src="/logo.svg"
    version="v4.1.0"
    links=vec![LinkItem::new("Features", "#features"), LinkItem::new("Pricing", "#pricing")]
  >
    // right-hand actions slot: theme toggle, CTA, …
    <a href="#contact">"Contact"</a>
  </Navbar>

  <Hero
    eyebrow="Local CI/CD"
    title="Simple, yet powerful "
    highlight="local CI/CD"
    subtitle="One YAML replaces five to seven scattered configs."
    actions=vec![
      CtaAction::primary("Request a demo", "#contact"),
      CtaAction::secondary("See features", "#features"),
    ]
    note="Linux · macOS · Windows · Built in Rust"
  />

  <StatStrip stats=vec![
    Stat::new("9", "export formats"),
    Stat::new("2", "execution engines"),
    Stat::new("YAML", "one config file"),
    Stat::new("~12 MB", "single static binary"),
  ] />

  <FeatureGrid
    id="features"
    eyebrow="Features"
    title="Everything you need"
    features=vec![
      Feature::new(view! { <span>"⚙"</span> }.into_any(), "Local pipelines", "Run builds on your machine."),
      Feature::text("Signed deploys", "SHA-256 signatures embedded into the archive."),
    ]
  />

  <Pricing
    id="pricing"
    eyebrow="Pricing"
    title="Plans for developers and teams"
    tiers=vec![
      PricingTier::new("Individual", "₽300", "per dev / month", CtaAction::secondary("Get a license", "#contact"))
        .features(["Full CLI & TUI", "AI init", "All 9 export formats"]),
      PricingTier::new("Team", "₽100 000", "per year", CtaAction::primary("Request a demo", "#contact"))
        .note("25 seats")
        .features(["Everything in Individual", "Signed deploys"])
        .highlighted(),
    ]
  />

  <Faq title="Frequently asked questions" items=vec![
    FaqItem::new("Which platforms?", "Linux, macOS and Windows."),
  ] />

  <CallToAction
    title="Ready to start?"
    subtitle="Request a demo and see it on your repo."
    actions=vec![CtaAction::primary("Request a demo", "#contact")]
  />

  <Footer
    brand="Деплойер"
    tagline="Simple, yet powerful local CI/CD."
    columns=vec![FooterColumn::new("Product", [LinkItem::new("Pricing", "#pricing")])]
    notes=vec!["© Verbal Automation Systems LLC".into()]
  />
}
```

### Blocks

| Block               | Role                                                                         |
| ------------------- | ---------------------------------------------------------------------------- |
| `AnnouncementBanner`| Slim promo strip above the navbar.                                           |
| `Navbar`            | Sticky, translucent top nav with a brand, links and a free-form actions slot.|
| `Hero`              | Headline section with eyebrow, gradient-highlighted title, CTAs and backdrop.|
| `StatStrip`         | A band of headline numbers / KPIs.                                          |
| `LogoCloud`         | A muted "trusted by" logo / wordmark row.                                   |
| `FeatureGrid`       | Responsive grid of icon + title + description cards.                        |
| `StepList`          | Auto-numbered "how it works" sequence.                                      |
| `ChecklistSection`  | Two-column section: heading beside a ticked checklist.                      |
| `MetricComparison`  | "before → after" metric cards with optional progress bars.                 |
| `Testimonials`      | Grid of quote cards with author + role + avatar.                           |
| `PillRow`           | Centered row of pills — a tech-stack / "built with" strip.                 |
| `Pricing`           | Row of pricing tiers, with one optionally highlighted.                     |
| `Faq`               | Single-open accordion of questions and answers.                            |
| `CallToAction`      | Closing CTA band over the glow.                                            |
| `Footer`            | Brand block, link columns and a legal/colophon bottom bar.                 |
| `SectionHeading`    | The shared eyebrow + title + subtitle header every section uses.           |
| `GridBackdrop`      | The decorative blueprint-grid + glow backdrop, for your own sections.       |

### Data helpers

Blocks are fed with small, ergonomic structs — most have constructors and
builder-style setters:

- `LinkItem::new(label, href)` — a plain nav/footer link.
- `CtaAction::primary(label, href)` / `CtaAction::secondary(label, href)` — a
  filled or outline button.
- `Stat::new(number, label)`, `Pill::new(name)` / `Pill::noted(name, note)`,
  `Logo::wordmark(name)` / `Logo::image(name, src)`.
- `Feature::new(icon, title, desc)` (icon is any `AnyView`) / `Feature::text(title, desc)`.
- `Step::new(title, body)`, `ChecklistItem::new(title, body)`, `FaqItem::new(q, a)`.
- `Metric::new(label, before, after).progress(pct)`.
- `Testimonial::new(quote, author).role(..).avatar(src)`.
- `PricingTier::new(name, price, period, cta).note(..).features([..]).highlighted()`.
- `FooterColumn::new(title, [LinkItem, …])`.

### Section props

The content sections (`FeatureGrid`, `StepList`, `ChecklistSection`,
`MetricComparison`, `Testimonials`, `Pricing`, `Faq`) share the same surface:
an optional `eyebrow`, a required `title`, an optional `subtitle` and an
optional `id` anchor for in-page navigation. Several also take `muted=true` to
sit on the alternating muted background. `Hero` and `CallToAction` take
`backdrop` / `grid` flags to toggle the grid + glow.

Prose fields (feature titles/descriptions, step and checklist bodies, FAQ
answers) render inline `` `code` `` spans — text wrapped in backticks becomes a
styled `<Raw>` chip, exactly like the kit's `rich` helper. Text without
backticks is unaffected.

> Like every block in this crate, the landing blocks' **default** classes are
> string literals scanned by Tailwind, so they just work. Classes **you** pass
> as data (e.g. text inside a `Feature` icon you build yourself) must live in
> your own scanned sources.

---

## Theming & customization

Blocks read the same CSS variables the kit defines in your `style.css`:
`--background`, `--foreground`, `--card`, `--muted(-foreground)`, `--primary`,
`--border`, `--popover(-foreground)`, `--destructive`, and the chart palette
`--chart-1` … `--chart-5`. Switching light/dark (via the kit's theme handling)
re-colors every block automatically.

Two layers of customization:

1. **Per-element class structs** (`MarkdownClasses`, `BarChartClasses`, …) — set
   the full Tailwind class string for a given element.
2. **The `class` prop** on every block — extra classes on the wrapping
   container, and on `GraphNode` / `GraphPort` the node/socket itself.

Remember the Tailwind note above: any class you pass as an override must appear
in *your* scanned sources for Tailwind to emit it.
