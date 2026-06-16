//! Interactive node graph — draggable nodes with connectable ports.
//!
//! A [`GraphCanvas`] hosts absolutely-positioned [`GraphNode`]s that the
//! developer places programmatically and the user can drag around. Each node
//! exposes [`GraphPort`] connectors (Blender-style sockets): drag from one port
//! and drop on another to wire them together, and a single node can fan out to
//! as many connections as you like. Nodes render arbitrary content and pick a
//! [`NodeVariant`] style (or a fully custom `class`).
//!
//! Everything is plain SVG + HTML through Leptos `view!`, so it is theme-aware
//! and every element is a real, hit-testable DOM node.
//!
//! ```
//! use impulse_ui_kit_blocks::graph::{GraphEdge, NodeVariant, PortSide};
//!
//! let edge = GraphEdge::new("input", "out", "output", "in");
//! assert_eq!(edge.from, ("input".to_string(), "out".to_string()));
//! assert_eq!(NodeVariant::default(), NodeVariant::Solid);
//! let _ = PortSide::Right.opposite();
//! ```

use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

/// The side of a node a [`GraphPort`] sits on. Controls how edges curve out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortSide {
  /// Left edge (typically an input).
  Left,
  /// Right edge (typically an output).
  Right,
  /// Top edge.
  Top,
  /// Bottom edge.
  Bottom,
}

impl PortSide {
  /// The unit direction an edge leaves this side in.
  fn dir(self) -> (f64, f64) {
    match self {
      PortSide::Left => (-1.0, 0.0),
      PortSide::Right => (1.0, 0.0),
      PortSide::Top => (0.0, -1.0),
      PortSide::Bottom => (0.0, 1.0),
    }
  }

  /// The opposing side, used to curve a pending edge toward the cursor.
  pub fn opposite(self) -> Self {
    match self {
      PortSide::Left => PortSide::Right,
      PortSide::Right => PortSide::Left,
      PortSide::Top => PortSide::Bottom,
      PortSide::Bottom => PortSide::Top,
    }
  }
}

/// A built-in node style, mirroring the variant idea of the UI Kit `Button`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NodeVariant {
  /// Thin solid border on a card background.
  #[default]
  Solid,
  /// Thicker solid outline.
  Outline,
  /// Dashed outline.
  Dashed,
  /// Accent (primary) outline.
  Accent,
  /// Borderless, subtle muted fill.
  Ghost,
}

impl NodeVariant {
  fn class(self) -> &'static str {
    match self {
      NodeVariant::Solid => "border border-border bg-card",
      NodeVariant::Outline => "border-2 border-border bg-card",
      NodeVariant::Dashed => "border-2 border-dashed border-border bg-card",
      NodeVariant::Accent => "border-2 border-primary bg-card",
      NodeVariant::Ghost => "border border-transparent bg-muted/40",
    }
  }
}

/// A connection between two ports, identified by `(node_id, port_id)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphEdge {
  /// Source `(node_id, port_id)`.
  pub from: (String, String),
  /// Target `(node_id, port_id)`.
  pub to: (String, String),
  /// Optional CSS stroke color. Defaults to the muted-foreground token.
  pub color: Option<String>,
}

impl GraphEdge {
  /// Build an edge from a source port to a target port.
  pub fn new(
    from_node: impl Into<String>,
    from_port: impl Into<String>,
    to_node: impl Into<String>,
    to_port: impl Into<String>,
  ) -> Self {
    Self {
      from: (from_node.into(), from_port.into()),
      to: (to_node.into(), to_port.into()),
      color: None,
    }
  }

  /// Set an explicit stroke color for this edge.
  pub fn with_color(mut self, color: impl Into<String>) -> Self {
    self.color = Some(color.into());
    self
  }
}

/// Whether a [`GraphPort`] accepts incoming connections or starts outgoing ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortKind {
  /// An input — the target of a connection.
  Input,
  /// An output — the source of a connection.
  Output,
}

impl PortKind {
  /// Default kind inferred from the side: left/top are inputs, right/bottom outputs.
  fn from_side(side: PortSide) -> Self {
    match side {
      PortSide::Left | PortSide::Top => PortKind::Input,
      PortSide::Right | PortSide::Bottom => PortKind::Output,
    }
  }
}

/// Measured position of a port, relative to its node's top-left corner, plus its
/// connection direction and optional data type.
#[derive(Clone)]
struct PortInfo {
  side: PortSide,
  kind: PortKind,
  data_type: Option<String>,
  rel: (f64, f64),
}

/// Whether two ports may be connected: opposite directions and matching types
/// (a port without a `data_type` acts as a wildcard).
fn ports_compatible(a: &PortInfo, b: &PortInfo) -> bool {
  if a.kind == b.kind {
    return false;
  }
  match (&a.data_type, &b.data_type) {
    (Some(x), Some(y)) => x == y,
    _ => true,
  }
}

/// In-progress node drag: which node, and the cursor-to-origin offset.
#[derive(Clone)]
struct NodeDrag {
  id: String,
  offset: (f64, f64),
}

/// In-progress edge creation: the source port and the live cursor position.
#[derive(Clone)]
struct Pending {
  from: (String, String),
  cursor: (f64, f64),
}

/// The canvas viewport: a pan offset (screen px) and a zoom scale.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Viewport {
  tx: f64,
  ty: f64,
  scale: f64,
}

impl Default for Viewport {
  fn default() -> Self {
    Self {
      tx: 0.0,
      ty: 0.0,
      scale: 1.0,
    }
  }
}

impl Viewport {
  /// Convert a canvas-relative screen point to world coordinates.
  fn to_world(self, screen: (f64, f64)) -> (f64, f64) {
    ((screen.0 - self.tx) / self.scale, (screen.1 - self.ty) / self.scale)
  }
}

/// Canvas-wide reactive state shared with every node and port via context.
#[derive(Clone, Copy)]
struct GraphContext {
  canvas: NodeRef<leptos::html::Div>,
  positions: RwSignal<HashMap<String, (f64, f64)>>,
  sizes: RwSignal<HashMap<String, (f64, f64)>>,
  ports: RwSignal<HashMap<(String, String), PortInfo>>,
  edges: RwSignal<Vec<GraphEdge>>,
  drag: RwSignal<Option<NodeDrag>>,
  pending: RwSignal<Option<Pending>>,
  /// Pan/zoom viewport applied to the world layer.
  view: RwSignal<Viewport>,
  /// Id of the node raised to the front (last interacted).
  active: RwSignal<Option<String>>,
  /// Ids of nodes removed by the user (statically-declared nodes are hidden).
  removed: RwSignal<HashSet<String>>,
  /// Whether delete affordances are shown.
  deletable: bool,
}

/// Remove a node and everything referencing it.
fn remove_node(ctx: &GraphContext, id: &str) {
  ctx.removed.update(|s| {
    s.insert(id.to_string());
  });
  ctx.positions.update(|m| {
    m.remove(id);
  });
  ctx.sizes.update(|m| {
    m.remove(id);
  });
  ctx.ports.update(|m| m.retain(|key, _| key.0 != id));
  ctx
    .edges
    .update(|e| e.retain(|edge| edge.from.0 != id && edge.to.0 != id));
}

/// Per-node context, so ports can find their node id and box for measuring.
#[derive(Clone)]
struct NodeLocalContext {
  id: String,
  container: NodeRef<leptos::html::Div>,
  /// Initial position, used as a drag fallback if the map has no entry.
  init: (f64, f64),
}

/// An automatic node-placement algorithm for a [`GraphCanvas`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphLayout {
  /// Force-directed (Fruchterman-Reingold): good for general graphs.
  ForceDirected,
  /// Layered left-to-right placement by edge depth: good for DAGs / pipelines.
  Hierarchical,
}

/// Layout and behavior options for a [`GraphCanvas`].
#[derive(Clone, Debug, PartialEq)]
pub struct GraphCanvasOptions {
  /// Canvas height in pixels.
  pub height: f64,
  /// Draw the dotted background grid.
  pub show_grid: bool,
  /// Grid step in pixels (background dots and snapping).
  pub grid_size: f64,
  /// Snap dragged nodes to the grid.
  pub snap: bool,
  /// Allow removing edges (click a wire) and nodes (× button).
  pub deletable: bool,
  /// Pan the canvas by dragging the empty background.
  pub pannable: bool,
  /// Zoom the canvas with the mouse wheel.
  pub zoomable: bool,
  /// Minimum zoom scale.
  pub min_scale: f64,
  /// Maximum zoom scale.
  pub max_scale: f64,
  /// Run this layout once after the nodes mount (overrides their initial `x`/`y`).
  pub layout: Option<GraphLayout>,
}

impl Default for GraphCanvasOptions {
  fn default() -> Self {
    Self {
      height: 480.0,
      show_grid: true,
      grid_size: 16.0,
      snap: false,
      deletable: true,
      pannable: true,
      zoomable: true,
      min_scale: 0.25,
      max_scale: 2.5,
      layout: None,
    }
  }
}

/// Client coordinates mapped to the canvas element's top-left (screen px).
fn client_to_canvas(canvas: &NodeRef<leptos::html::Div>, client_x: f64, client_y: f64) -> (f64, f64) {
  if let Some(el) = canvas.get_untracked() {
    let rect = el.get_bounding_client_rect();
    (client_x - rect.left(), client_y - rect.top())
  } else {
    (client_x, client_y)
  }
}

/// Pointer position relative to the canvas element (screen px).
fn pointer_pos(canvas: &NodeRef<leptos::html::Div>, ev: &web_sys::PointerEvent) -> (f64, f64) {
  client_to_canvas(canvas, ev.client_x() as f64, ev.client_y() as f64)
}

/// An axis-aligned node rectangle, used as an edge-routing obstacle.
#[derive(Clone, Copy)]
struct Rect {
  x: f64,
  y: f64,
  w: f64,
  h: f64,
}

impl Rect {
  /// Whether `p` lies within the rectangle, inflated by margin `m`.
  fn contains(&self, p: (f64, f64), m: f64) -> bool {
    p.0 >= self.x - m && p.0 <= self.x + self.w + m && p.1 >= self.y - m && p.1 <= self.y + self.h + m
  }
}

/// The two cubic-bezier control points for an edge leaving `s1` into `s2`.
fn controls(p1: (f64, f64), s1: PortSide, p2: (f64, f64), s2: PortSide) -> ((f64, f64), (f64, f64)) {
  let dx = (p2.0 - p1.0).abs();
  let dy = (p2.1 - p1.1).abs();
  let k = (dx.max(dy) * 0.5).max(40.0);
  let d1 = s1.dir();
  let d2 = s2.dir();
  ((p1.0 + d1.0 * k, p1.1 + d1.1 * k), (p2.0 + d2.0 * k, p2.1 + d2.1 * k))
}

/// Plain cubic-bezier path between two ports (or a port and a free cursor).
fn edge_path(p1: (f64, f64), s1: PortSide, p2: (f64, f64), s2: PortSide) -> String {
  let (c1, c2) = controls(p1, s1, p2, s2);
  format!(
    "M {} {} C {} {} {} {} {} {}",
    p1.0, p1.1, c1.0, c1.1, c2.0, c2.1, p2.0, p2.1
  )
}

/// A point on the cubic bezier `p0 c1 c2 p1` at parameter `t`.
fn cubic_point(p0: (f64, f64), c1: (f64, f64), c2: (f64, f64), p1: (f64, f64), t: f64) -> (f64, f64) {
  let u = 1.0 - t;
  let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
  (
    a * p0.0 + b * c1.0 + c * c2.0 + d * p1.0,
    a * p0.1 + b * c1.1 + c * c2.1 + d * p1.1,
  )
}

/// Obstacles the straight bezier passes through (sampled), inflated by `m`.
fn blocking_rects(
  p1: (f64, f64),
  c1: (f64, f64),
  c2: (f64, f64),
  p2: (f64, f64),
  obstacles: &[Rect],
  m: f64,
) -> Vec<&Rect> {
  let steps = 24;
  obstacles
    .iter()
    .filter(|r| (1..steps).any(|i| r.contains(cubic_point(p1, c1, c2, p2, i as f64 / steps as f64), m)))
    .collect()
}

fn is_horizontal(side: PortSide) -> bool {
  matches!(side, PortSide::Left | PortSide::Right)
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
  ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// A point `d` units from `from` toward `to`.
fn toward(from: (f64, f64), to: (f64, f64), d: f64) -> (f64, f64) {
  let len = dist(from, to);
  if len < 1e-6 {
    return from;
  }
  let t = d / len;
  (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
}

/// An SVG path through `pts` with rounded corners of up to `radius`.
fn rounded_path(pts: &[(f64, f64)], radius: f64) -> String {
  let mut d = format!("M {} {}", pts[0].0, pts[0].1);
  if pts.len() < 3 {
    for p in &pts[1..] {
      d.push_str(&format!(" L {} {}", p.0, p.1));
    }
    return d;
  }
  for i in 1..pts.len() - 1 {
    let (a, p, b) = (pts[i - 1], pts[i], pts[i + 1]);
    let entry = toward(p, a, radius.min(dist(a, p) / 2.0));
    let exit = toward(p, b, radius.min(dist(p, b) / 2.0));
    d.push_str(&format!(
      " L {} {} Q {} {} {} {}",
      entry.0, entry.1, p.0, p.1, exit.0, exit.1
    ));
  }
  let last = pts[pts.len() - 1];
  d.push_str(&format!(" L {} {}", last.0, last.1));
  d
}

/// Route an edge from `p1`/`s1` to `p2`/`s2`, detouring around `obstacles`.
///
/// When the direct bezier is clear it is used as-is. Otherwise — for the common
/// horizontal input/output flow — the edge is rerouted as a rounded orthogonal
/// path that hops over or under the blocking nodes.
fn route_edge(p1: (f64, f64), s1: PortSide, p2: (f64, f64), s2: PortSide, obstacles: &[Rect]) -> String {
  let m = 12.0;
  let (c1, c2) = controls(p1, s1, p2, s2);
  let hits = blocking_rects(p1, c1, c2, p2, obstacles, m);
  if hits.is_empty() || !(is_horizontal(s1) && is_horizontal(s2)) {
    return edge_path(p1, s1, p2, s2);
  }

  let top = hits.iter().map(|r| r.y).fold(f64::INFINITY, f64::min) - m;
  let bottom = hits.iter().map(|r| r.y + r.h).fold(f64::NEG_INFINITY, f64::max) + m;
  let avg = (p1.1 + p2.1) / 2.0;
  let detour_y = if avg - top <= bottom - avg { top } else { bottom };

  let out = 24.0;
  let x1 = p1.0 + s1.dir().0 * out;
  let x2 = p2.0 + s2.dir().0 * out;
  let pts = [p1, (x1, p1.1), (x1, detour_y), (x2, detour_y), (x2, p2.1), p2];
  rounded_path(&pts, 14.0)
}

/// Resolve a port's absolute canvas position and side.
fn port_abs(
  positions: &HashMap<String, (f64, f64)>,
  ports: &HashMap<(String, String), PortInfo>,
  key: &(String, String),
) -> Option<((f64, f64), PortSide)> {
  let info = ports.get(key)?;
  let pos = positions.get(&key.0)?;
  Some(((pos.0 + info.rel.0, pos.1 + info.rel.1), info.side))
}

/// Compute node positions for `kind`, given node sizes and edge index pairs.
fn compute_layout(
  kind: GraphLayout,
  edges: &[(usize, usize)],
  sizes: &[(f64, f64)],
  width: f64,
  height: f64,
) -> Vec<(f64, f64)> {
  match kind {
    GraphLayout::ForceDirected => force_layout(edges, sizes, width, height),
    GraphLayout::Hierarchical => hier_layout(edges, sizes, 72.0, 28.0),
  }
}

/// Fruchterman-Reingold force-directed layout. Returns top-left positions.
fn force_layout(edges: &[(usize, usize)], sizes: &[(f64, f64)], width: f64, height: f64) -> Vec<(f64, f64)> {
  let n = sizes.len();
  if n == 0 {
    return Vec::new();
  }
  let (cx, cy) = (width / 2.0, height / 2.0);
  let radius = (width.min(height) / 2.0 - 48.0).max(48.0);
  let mut pos: Vec<(f64, f64)> = (0..n)
    .map(|i| {
      let a = 2.0 * PI * i as f64 / n as f64;
      (cx + radius * a.cos(), cy + radius * a.sin())
    })
    .collect();

  let k = (width * height / n as f64).sqrt();
  let mut temp = width / 8.0;
  for _ in 0..320 {
    let mut disp = vec![(0.0, 0.0); n];
    for i in 0..n {
      for j in 0..n {
        if i == j {
          continue;
        }
        let (dx, dy) = (pos[i].0 - pos[j].0, pos[i].1 - pos[j].1);
        let dist = (dx * dx + dy * dy).sqrt().max(0.01);
        let rep = k * k / dist;
        disp[i].0 += dx / dist * rep;
        disp[i].1 += dy / dist * rep;
      }
    }
    for &(a, b) in edges {
      let (dx, dy) = (pos[a].0 - pos[b].0, pos[a].1 - pos[b].1);
      let dist = (dx * dx + dy * dy).sqrt().max(0.01);
      let att = dist * dist / k;
      let (fx, fy) = (dx / dist * att, dy / dist * att);
      disp[a].0 -= fx;
      disp[a].1 -= fy;
      disp[b].0 += fx;
      disp[b].1 += fy;
    }
    for i in 0..n {
      let dl = (disp[i].0 * disp[i].0 + disp[i].1 * disp[i].1).sqrt().max(0.01);
      pos[i].0 = (pos[i].0 + disp[i].0 / dl * dl.min(temp)).clamp(48.0, width - 48.0);
      pos[i].1 = (pos[i].1 + disp[i].1 / dl * dl.min(temp)).clamp(48.0, height - 48.0);
    }
    temp *= 0.95;
  }

  pos
    .iter()
    .enumerate()
    .map(|(i, c)| (c.0 - sizes[i].0 / 2.0, c.1 - sizes[i].1 / 2.0))
    .collect()
}

/// Layered left-to-right layout by longest-path depth. Returns top-left positions.
fn hier_layout(edges: &[(usize, usize)], sizes: &[(f64, f64)], x_gap: f64, y_gap: f64) -> Vec<(f64, f64)> {
  let n = sizes.len();
  if n == 0 {
    return Vec::new();
  }
  // Longest-path layering, relaxed up to n times (cycles are capped).
  let mut layer = vec![0usize; n];
  for _ in 0..n {
    let mut changed = false;
    for &(a, b) in edges {
      if layer[b] < layer[a] + 1 {
        layer[b] = layer[a] + 1;
        changed = true;
      }
    }
    if !changed {
      break;
    }
  }

  let max_layer = layer.iter().copied().max().unwrap_or(0);
  let mut col_w = vec![0.0f64; max_layer + 1];
  for i in 0..n {
    col_w[layer[i]] = col_w[layer[i]].max(sizes[i].0);
  }
  let mut col_x = vec![0.0f64; max_layer + 1];
  let mut acc = 40.0;
  for l in 0..=max_layer {
    col_x[l] = acc;
    acc += col_w[l] + x_gap;
  }

  let mut y_cursor = vec![40.0f64; max_layer + 1];
  let mut pos = vec![(0.0, 0.0); n];
  for i in 0..n {
    let l = layer[i];
    pos[i] = (col_x[l], y_cursor[l]);
    y_cursor[l] += sizes[i].1 + y_gap;
  }
  pos
}

/// The node-graph canvas: a draggable, connectable surface for [`GraphNode`]s.
///
/// * `positions` — optional controlled map of `node_id -> (x, y)`; the developer
///   can read and mutate it, and dragging updates it in place.
/// * `edges` — optional controlled list of connections; user-made connections
///   are appended here.
/// * `options` — canvas height and grid.
/// * `class` — extra classes for the canvas container.
/// * `children` — the [`GraphNode`]s.
#[component]
pub fn GraphCanvas(
  #[prop(optional)] positions: Option<RwSignal<HashMap<String, (f64, f64)>>>,
  #[prop(optional)] edges: Option<RwSignal<Vec<GraphEdge>>>,
  #[prop(optional)] options: GraphCanvasOptions,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let positions = positions.unwrap_or_else(|| RwSignal::new(HashMap::new()));
  let edges = edges.unwrap_or_else(|| RwSignal::new(Vec::new()));
  let canvas = NodeRef::<leptos::html::Div>::new();
  let sizes = RwSignal::new(HashMap::<String, (f64, f64)>::new());
  let ports = RwSignal::new(HashMap::<(String, String), PortInfo>::new());
  let drag = RwSignal::new(None::<NodeDrag>);
  let pending = RwSignal::new(None::<Pending>);
  let active = RwSignal::new(None::<String>);
  let removed = RwSignal::new(HashSet::<String>::new());
  let view = RwSignal::new(Viewport::default());
  let hovered_edge = RwSignal::new(None::<usize>);
  // Background-drag pan: (start screen point, start translate).
  let panning = RwSignal::new(None::<((f64, f64), (f64, f64))>);

  let grid = options.grid_size;
  let snap = options.snap;
  let deletable = options.deletable;
  let (pannable, zoomable) = (options.pannable, options.zoomable);
  let (min_scale, max_scale) = (options.min_scale, options.max_scale);
  let height = options.height;
  let show_grid = options.show_grid;
  let layout = options.layout;

  let ctx = GraphContext {
    canvas,
    positions,
    sizes,
    ports,
    edges,
    drag,
    pending,
    view,
    active,
    removed,
    deletable,
  };
  provide_context(ctx);

  // Apply an automatic layout once, on the frame after the nodes have mounted
  // and seeded their positions.
  if let Some(kind) = layout {
    let applied = StoredValue::new(false);
    Effect::new(move |_| {
      let has_any = positions.with(|m| !m.is_empty());
      if !has_any || applied.get_value() {
        return;
      }
      applied.set_value(true);
      request_animation_frame(move || {
        let ids: Vec<String> = positions.get_untracked().keys().cloned().collect();
        if ids.is_empty() {
          return;
        }
        let sizes_map = sizes.get_untracked();
        let sizes_vec: Vec<(f64, f64)> = ids
          .iter()
          .map(|id| sizes_map.get(id).copied().unwrap_or((192.0, 96.0)))
          .collect();
        let index: HashMap<&String, usize> = ids.iter().enumerate().map(|(i, id)| (id, i)).collect();
        let pairs: Vec<(usize, usize)> = edges
          .get_untracked()
          .iter()
          .filter_map(|e| Some((*index.get(&e.from.0)?, *index.get(&e.to.0)?)))
          .collect();
        let w = canvas
          .get_untracked()
          .map(|el| el.get_bounding_client_rect().width())
          .filter(|w| *w > 1.0)
          .unwrap_or(800.0);
        let placed = compute_layout(kind, &pairs, &sizes_vec, w, height);
        positions.update(|m| {
          for (i, id) in ids.iter().enumerate() {
            m.insert(id.clone(), placed[i]);
          }
        });
      });
    });
  }

  // Edge paths follow node positions reactively and route around other nodes.
  let edge_paths = move || {
    let positions = positions.get();
    let sizes = sizes.get();
    let ports = ports.get();
    edges
      .get()
      .iter()
      .enumerate()
      .filter_map(|(i, edge)| {
        let (p1, s1) = port_abs(&positions, &ports, &edge.from)?;
        let (p2, s2) = port_abs(&positions, &ports, &edge.to)?;
        // Every other node is an obstacle to route around.
        let obstacles: Vec<Rect> = positions
          .iter()
          .filter(|(id, _)| **id != edge.from.0 && **id != edge.to.0)
          .filter_map(|(id, pos)| {
            sizes.get(id).map(|&(w, h)| Rect {
              x: pos.0,
              y: pos.1,
              w,
              h,
            })
          })
          .collect();
        let d = route_edge(p1, s1, p2, s2, &obstacles);
        let base = edge
          .color
          .clone()
          .unwrap_or_else(|| "var(--muted-foreground)".to_string());

        let stroke = move || {
          if deletable && hovered_edge.get() == Some(i) {
            "var(--destructive)".to_string()
          } else {
            base.clone()
          }
        };
        let stroke_width = move || {
          if deletable && hovered_edge.get() == Some(i) {
            "3"
          } else {
            "2"
          }
        };

        // A wide, transparent hit path makes the thin wire easy to click.
        let hit = deletable.then(|| {
          let edge = edge.clone();
          view! {
            <path
              d=d.clone()
              fill="none"
              stroke="transparent"
              stroke-width="16"
              class="pointer-events-auto cursor-pointer"
              on:pointerenter=move |_| hovered_edge.set(Some(i))
              on:pointerleave=move |_| {
                hovered_edge.update(|h| {
                  if *h == Some(i) {
                    *h = None;
                  }
                })
              }
              on:click=move |ev| {
                ev.stop_propagation();
                edges.update(|v| v.retain(|e| !(e.from == edge.from && e.to == edge.to)));
                hovered_edge.set(None);
              }
            >
              <title>"Click to remove"</title>
            </path>
          }
        });

        Some(
          view! {
            <g>
              {hit}
              <path
                d=d
                fill="none"
                stroke=stroke
                stroke-width=stroke_width
                class="pointer-events-none transition-colors"
              />
            </g>
          }
          .into_any(),
        )
      })
      .collect_view()
  };

  // The edge currently being dragged out of a port.
  let pending_path = move || {
    let positions = positions.get();
    let ports = ports.get();
    pending.get().and_then(|p| {
      let (p1, s1) = port_abs(&positions, &ports, &p.from)?;
      Some(
        view! {
          <path
            d=edge_path(p1, s1, p.cursor, s1.opposite())
            fill="none"
            stroke="var(--primary)"
            stroke-width="2"
            stroke-dasharray="4 4"
          />
        }
        .into_any(),
      )
    })
  };

  // Pointer down on the empty background starts a pan (nodes/ports/header and
  // edge hit paths stop propagation, so this only fires on blank canvas).
  let on_down = move |ev: web_sys::PointerEvent| {
    if pannable {
      let screen = pointer_pos(&canvas, &ev);
      let v = view.get_untracked();
      panning.set(Some((screen, (v.tx, v.ty))));
    }
  };

  let on_move = move |ev: web_sys::PointerEvent| {
    let screen = pointer_pos(&canvas, &ev);
    let v = view.get_untracked();
    let world = v.to_world(screen);

    if let Some(d) = drag.get_untracked() {
      let mut p = (world.0 - d.offset.0, world.1 - d.offset.1);
      if snap && grid > 0.0 {
        p = ((p.0 / grid).round() * grid, (p.1 / grid).round() * grid);
      }
      positions.update(|m| {
        m.insert(d.id.clone(), p);
      });
    }
    if pending.get_untracked().is_some() {
      pending.update(|p| {
        if let Some(p) = p.as_mut() {
          p.cursor = world;
        }
      });
    }
    if let Some((start, start_t)) = panning.get_untracked() {
      view.update(|v| {
        v.tx = start_t.0 + (screen.0 - start.0);
        v.ty = start_t.1 + (screen.1 - start.1);
      });
    }
  };
  let on_up = move |_: web_sys::PointerEvent| {
    drag.set(None);
    pending.set(None);
    panning.set(None);
  };

  // Wheel zooms toward the cursor, keeping the world point under it fixed.
  let on_wheel = move |ev: web_sys::WheelEvent| {
    if !zoomable {
      return;
    }
    ev.prevent_default();
    let cursor = client_to_canvas(&canvas, ev.client_x() as f64, ev.client_y() as f64);
    let v = view.get_untracked();
    let factor = if ev.delta_y() < 0.0 { 1.1 } else { 1.0 / 1.1 };
    let scale = (v.scale * factor).clamp(min_scale, max_scale);
    let world = v.to_world(cursor);
    view.set(Viewport {
      tx: cursor.0 - world.0 * scale,
      ty: cursor.1 - world.1 * scale,
      scale,
    });
  };

  // The dotted grid lives on the fixed container, but pans and zooms with the view.
  let container_style = move || {
    let mut s = format!("height:{height}px;touch-action:none;");
    if show_grid {
      let v = view.get();
      let step = grid * v.scale;
      s.push_str(&format!(
        "background-color:var(--background);background-image:radial-gradient(circle, var(--border) 1px, transparent 1px);background-size:{step}px {step}px;background-position:{}px {}px;",
        v.tx, v.ty
      ));
    }
    s
  };
  // CSS transform mapping world coordinates to screen.
  let world_style = move || {
    let v = view.get();
    format!(
      "position:absolute;inset:0;transform-origin:0 0;transform:translate({}px,{}px) scale({});",
      v.tx, v.ty, v.scale
    )
  };

  let cursor = if pannable {
    "cursor-grab active:cursor-grabbing"
  } else {
    ""
  };

  view! {
    <div
      node_ref=canvas
      class=cn(&["relative w-full overflow-hidden rounded-lg border border-border", cursor, class.as_str()])
      style=container_style
      on:pointerdown=on_down
      on:pointermove=on_move
      on:pointerup=on_up
      on:pointerleave=on_up
      on:wheel=on_wheel
    >
      <div style=world_style>
        <svg class="pointer-events-none absolute inset-0 h-full w-full" style="overflow:visible">
          {edge_paths}
          {pending_path}
        </svg>
        {children()}
      </div>
    </div>
  }
}

/// A draggable node placed on a [`GraphCanvas`].
///
/// * `id` — unique node id, referenced by [`GraphEdge`]s and ports.
/// * `x` / `y` — initial position (used only if the canvas `positions` map has
///   no entry for this `id` yet).
/// * `variant` — built-in style; override entirely with `class`.
/// * `width` — node width in pixels (default `192`).
/// * `children` — node content; wrap a [`GraphNodeHeader`] to get a drag handle.
#[component]
pub fn GraphNode(
  #[prop(into)] id: String,
  #[prop(optional)] x: f64,
  #[prop(optional)] y: f64,
  #[prop(optional)] variant: NodeVariant,
  #[prop(optional)] width: Option<f64>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let ctx = use_context::<GraphContext>().expect("GraphNode must be used within a GraphCanvas");
  let container = NodeRef::<leptos::html::Div>::new();
  let width = width.unwrap_or(192.0);

  // Seed the position map once, without clobbering a developer-set entry.
  {
    let id = id.clone();
    Effect::new(move |_| {
      ctx.positions.update(|m| {
        m.entry(id.clone()).or_insert((x, y));
      });
    });
  }

  // Measure the rendered node box (in world units) so edges can route around it.
  {
    let id = id.clone();
    Effect::new(move |_| {
      let Some(el) = container.get() else {
        return;
      };
      let rect = el.get_bounding_client_rect();
      // getBoundingClientRect is post-transform, so undo the current zoom.
      let scale = ctx.view.get_untracked().scale.max(f64::MIN_POSITIVE);
      let size = (rect.width() / scale, rect.height() / scale);
      ctx.sizes.update(|m| {
        m.insert(id.clone(), size);
      });
    });
  }

  provide_context(NodeLocalContext {
    id: id.clone(),
    container,
    init: (x, y),
  });

  let pos_id = id.clone();
  let transform = move || {
    // A node removed by the user is hidden in place (its data is already gone).
    if ctx.removed.with(|s| s.contains(&pos_id)) {
      return "display:none;".to_string();
    }
    let (px, py) = ctx.positions.with(|m| m.get(&pos_id).copied()).unwrap_or((x, y));
    // The last-interacted node is raised above the others.
    let z = if ctx.active.with(|a| a.as_deref() == Some(pos_id.as_str())) {
      30
    } else {
      1
    };
    format!("transform:translate({px}px,{py}px);width:{width}px;z-index:{z};")
  };

  // Optional delete button, revealed on node hover.
  let delete = ctx.deletable.then(|| {
    let id = id.clone();
    let on_delete = move |ev: web_sys::MouseEvent| {
      ev.stop_propagation();
      remove_node(&ctx, &id);
    };
    view! {
      <button
        type="button"
        aria-label="Remove node"
        class="absolute -right-2 -top-2 z-10 flex h-5 w-5 items-center justify-center rounded-full border border-border bg-background text-xs leading-none text-muted-foreground opacity-0 shadow-sm transition hover:text-destructive group-hover:opacity-100"
        on:pointerdown=|ev| ev.stop_propagation()
        on:click=on_delete
      >
        "×"
      </button>
    }
  });

  view! {
    <div
      node_ref=container
      class=cn(
        &[
          "group absolute left-0 top-0 rounded-lg text-card-foreground shadow-sm select-none",
          variant.class(),
          class.as_str(),
        ],
      )
      style=transform
    >
      {delete}
      {children()}
    </div>
  }
}

/// A node header that doubles as the drag handle for its [`GraphNode`].
#[component]
pub fn GraphNodeHeader(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  let ctx = use_context::<GraphContext>().expect("GraphNodeHeader must be used within a GraphCanvas");
  let node = use_context::<NodeLocalContext>().expect("GraphNodeHeader must be used within a GraphNode");

  let on_down = move |ev: web_sys::PointerEvent| {
    ev.stop_propagation();
    let world = ctx.view.get_untracked().to_world(pointer_pos(&ctx.canvas, &ev));
    let pos = ctx
      .positions
      .with_untracked(|m| m.get(&node.id).copied())
      .unwrap_or(node.init);
    ctx.active.set(Some(node.id.clone()));
    ctx.drag.set(Some(NodeDrag {
      id: node.id.clone(),
      offset: (world.0 - pos.0, world.1 - pos.1),
    }));
  };

  view! {
    <div
      class=cn(
        &[
          "flex cursor-grab items-center gap-2 rounded-t-lg border-b border-border/60 bg-muted/40 px-3 py-2 text-sm font-medium active:cursor-grabbing",
          class.as_str(),
        ],
      )
      on:pointerdown=on_down
    >
      {children()}
    </div>
  }
}

/// A padded content area for a [`GraphNode`] body.
#[component]
pub fn GraphNodeBody(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! { <div class=cn(&["space-y-2 p-3 text-sm", class.as_str()])>{children()}</div> }
}

/// A connector socket on a node. Drag from it to start a connection and drop on
/// another port to wire them up. `label` is shown next to the socket.
///
/// Connections are validated: an output may only connect to an input, and if
/// both ports declare a `data_type` the types must match (a port without a type
/// is a wildcard). `kind` defaults to the side (left/top inputs, right/bottom
/// outputs).
#[component]
pub fn GraphPort(
  #[prop(into)] id: String,
  side: PortSide,
  #[prop(optional)] kind: Option<PortKind>,
  #[prop(optional, into)] data_type: Option<String>,
  #[prop(optional, into)] label: String,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let ctx = use_context::<GraphContext>().expect("GraphPort must be used within a GraphCanvas");
  let node = use_context::<NodeLocalContext>().expect("GraphPort must be used within a GraphNode");
  let dot = NodeRef::<leptos::html::Div>::new();
  let key = (node.id.clone(), id.clone());
  let kind = kind.unwrap_or_else(|| PortKind::from_side(side));

  // Measure the socket center relative to the node once both are mounted.
  {
    let key = key.clone();
    let container = node.container;
    let data_type = data_type.clone();
    Effect::new(move |_| {
      let (Some(node_el), Some(dot_el)) = (container.get(), dot.get()) else {
        return;
      };
      let nr = node_el.get_bounding_client_rect();
      let dr = dot_el.get_bounding_client_rect();
      // Socket center offset within the node, converted to world units.
      let scale = ctx.view.get_untracked().scale.max(f64::MIN_POSITIVE);
      let rel = (
        (dr.left() + dr.width() / 2.0 - nr.left()) / scale,
        (dr.top() + dr.height() / 2.0 - nr.top()) / scale,
      );
      ctx.ports.update(|m| {
        m.insert(
          key.clone(),
          PortInfo {
            side,
            kind,
            data_type: data_type.clone(),
            rel,
          },
        );
      });
    });
  }

  let start_key = key.clone();
  let node_id = node.id.clone();
  let on_down = move |ev: web_sys::PointerEvent| {
    ev.stop_propagation();
    let world = ctx.view.get_untracked().to_world(pointer_pos(&ctx.canvas, &ev));
    ctx.active.set(Some(node_id.clone()));
    ctx.pending.set(Some(Pending {
      from: start_key.clone(),
      cursor: world,
    }));
  };
  let finish_key = key.clone();
  let on_up = move |ev: web_sys::PointerEvent| {
    ev.stop_propagation();
    if let Some(p) = ctx.pending.get_untracked() {
      if p.from != finish_key {
        // Validate direction + type and orient the edge output -> input.
        let edge = ctx
          .ports
          .with_untracked(|m| match (m.get(&p.from), m.get(&finish_key)) {
            (Some(src), Some(dst)) if ports_compatible(src, dst) => {
              let (from, to) = if src.kind == PortKind::Output {
                (p.from.clone(), finish_key.clone())
              } else {
                (finish_key.clone(), p.from.clone())
              };
              Some(GraphEdge { from, to, color: None })
            }
            _ => None,
          });
        if let Some(edge) = edge {
          ctx.edges.update(|e| {
            if !e.iter().any(|x| x.from == edge.from && x.to == edge.to) {
              e.push(edge);
            }
          });
        }
      }
      ctx.pending.set(None);
    }
  };

  // The socket sits ON the node's outer border (not inside the padded body), so
  // edges leave from the edge and never tuck under the block. The row is pulled
  // full-bleed with `-mx-3` to cancel `GraphNodeBody`'s `px-3`, and the dot is
  // translated half its size past the border.
  let (row_class, dot_translate): (&str, &str) = match side {
    PortSide::Right => ("-mx-3 flex items-center justify-end gap-2 pl-3", "translate-x-1/2"),
    PortSide::Left => ("-mx-3 flex items-center gap-2 pr-3", "-translate-x-1/2"),
    PortSide::Top => ("flex items-center gap-2", "-translate-y-1/2"),
    PortSide::Bottom => ("flex items-center gap-2", "translate-y-1/2"),
  };

  // While a connection is being dragged, highlight compatible targets and dim
  // incompatible ones.
  let class_for_socket = class.clone();
  let highlight_key = key.clone();
  let highlight_type = data_type.clone();
  let highlight = move || match ctx.pending.get() {
    None => "",
    Some(p) if p.from == highlight_key => "ring-2 ring-primary",
    Some(p) => {
      let me = PortInfo {
        side,
        kind,
        data_type: highlight_type.clone(),
        rel: (0.0, 0.0),
      };
      let ok = ctx
        .ports
        .with(|m| m.get(&p.from).map(|src| ports_compatible(src, &me)).unwrap_or(false));
      if ok {
        "ring-2 ring-primary ring-offset-1"
      } else {
        "opacity-40"
      }
    }
  };

  let socket = view! {
    <div
      node_ref=dot
      class=move || {
        cn(
          &[
            "h-3 w-3 shrink-0 cursor-crosshair rounded-full border-2 border-border bg-background transition-all hover:border-primary hover:bg-primary",
            dot_translate,
            highlight(),
            class_for_socket.as_str(),
          ],
        )
      }
      on:pointerdown=on_down
      on:pointerup=on_up
    />
  };

  if matches!(side, PortSide::Right) {
    view! {
      <div class=row_class>
        <span class="text-xs text-muted-foreground">{label}</span>
        {socket}
      </div>
    }
    .into_any()
  } else {
    view! {
      <div class=row_class>
        {socket}
        <span class="text-xs text-muted-foreground">{label}</span>
      </div>
    }
    .into_any()
  }
}
