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

use std::collections::HashMap;

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

/// Measured position of a port, relative to its node's top-left corner.
#[derive(Clone, Copy)]
struct PortInfo {
  side: PortSide,
  rel: (f64, f64),
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

/// Canvas-wide reactive state shared with every node and port via context.
#[derive(Clone, Copy)]
struct GraphContext {
  canvas: NodeRef<leptos::html::Div>,
  positions: RwSignal<HashMap<String, (f64, f64)>>,
  ports: RwSignal<HashMap<(String, String), PortInfo>>,
  edges: RwSignal<Vec<GraphEdge>>,
  drag: RwSignal<Option<NodeDrag>>,
  pending: RwSignal<Option<Pending>>,
}

/// Per-node context, so ports can find their node id and box for measuring.
#[derive(Clone)]
struct NodeLocalContext {
  id: String,
  container: NodeRef<leptos::html::Div>,
}

/// Layout and behavior options for a [`GraphCanvas`].
#[derive(Clone, Debug, PartialEq)]
pub struct GraphCanvasOptions {
  /// Canvas height in pixels.
  pub height: f64,
  /// Draw the dotted background grid.
  pub show_grid: bool,
}

impl Default for GraphCanvasOptions {
  fn default() -> Self {
    Self {
      height: 480.0,
      show_grid: true,
    }
  }
}

/// Cursor position relative to the canvas element.
fn pointer_pos(canvas: &NodeRef<leptos::html::Div>, ev: &web_sys::PointerEvent) -> (f64, f64) {
  if let Some(el) = canvas.get_untracked() {
    let rect = el.get_bounding_client_rect();
    (ev.client_x() as f64 - rect.left(), ev.client_y() as f64 - rect.top())
  } else {
    (ev.client_x() as f64, ev.client_y() as f64)
  }
}

/// Cubic-bezier path between two ports (or a port and a free cursor).
fn edge_path(p1: (f64, f64), s1: PortSide, p2: (f64, f64), s2: PortSide) -> String {
  let dx = (p2.0 - p1.0).abs();
  let dy = (p2.1 - p1.1).abs();
  let k = (dx.max(dy) * 0.5).max(40.0);
  let d1 = s1.dir();
  let d2 = s2.dir();
  let c1 = (p1.0 + d1.0 * k, p1.1 + d1.1 * k);
  let c2 = (p2.0 + d2.0 * k, p2.1 + d2.1 * k);
  format!(
    "M {} {} C {} {} {} {} {} {}",
    p1.0, p1.1, c1.0, c1.1, c2.0, c2.1, p2.0, p2.1
  )
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
  let ports = RwSignal::new(HashMap::<(String, String), PortInfo>::new());
  let drag = RwSignal::new(None::<NodeDrag>);
  let pending = RwSignal::new(None::<Pending>);

  let ctx = GraphContext {
    canvas,
    positions,
    ports,
    edges,
    drag,
    pending,
  };
  provide_context(ctx);

  // Edge paths follow node positions reactively.
  let edge_paths = move || {
    let positions = positions.get();
    let ports = ports.get();
    edges
      .get()
      .iter()
      .filter_map(|edge| {
        let (p1, s1) = port_abs(&positions, &ports, &edge.from)?;
        let (p2, s2) = port_abs(&positions, &ports, &edge.to)?;
        let stroke = edge
          .color
          .clone()
          .unwrap_or_else(|| "var(--muted-foreground)".to_string());
        Some(
          view! {
            <path
              d=edge_path(p1, s1, p2, s2)
              fill="none"
              stroke=stroke
              stroke-width="2"
            />
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

  let on_move = move |ev: web_sys::PointerEvent| {
    let c = pointer_pos(&canvas, &ev);
    if let Some(d) = drag.get_untracked() {
      positions.update(|m| {
        m.insert(d.id.clone(), (c.0 - d.offset.0, c.1 - d.offset.1));
      });
    }
    if pending.get_untracked().is_some() {
      pending.update(|p| {
        if let Some(p) = p.as_mut() {
          p.cursor = c;
        }
      });
    }
  };
  let on_up = move |_: web_sys::PointerEvent| {
    drag.set(None);
    pending.set(None);
  };

  let mut container_style = format!("height:{}px;touch-action:none;", options.height);
  if options.show_grid {
    container_style.push_str(
      "background-color:var(--background);background-image:radial-gradient(circle, var(--border) 1px, transparent 1px);background-size:16px 16px;",
    );
  }

  view! {
    <div
      node_ref=canvas
      class=cn(&["relative w-full overflow-hidden rounded-lg border border-border", class.as_str()])
      style=container_style
      on:pointermove=on_move
      on:pointerup=on_up
      on:pointerleave=on_up
    >
      <svg class="pointer-events-none absolute inset-0 h-full w-full">
        {edge_paths}
        {pending_path}
      </svg>
      {children()}
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

  provide_context(NodeLocalContext {
    id: id.clone(),
    container,
  });

  let pos_id = id.clone();
  let transform = move || {
    let (px, py) = ctx.positions.with(|m| m.get(&pos_id).copied()).unwrap_or((x, y));
    format!("transform:translate({px}px,{py}px);width:{width}px;")
  };

  view! {
    <div
      node_ref=container
      class=cn(
        &["absolute left-0 top-0 rounded-lg text-card-foreground shadow-sm select-none", variant.class(), class.as_str()],
      )
      style=transform
    >
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
    let cursor = pointer_pos(&ctx.canvas, &ev);
    let pos = ctx
      .positions
      .with_untracked(|m| m.get(&node.id).copied())
      .unwrap_or((0.0, 0.0));
    ctx.drag.set(Some(NodeDrag {
      id: node.id.clone(),
      offset: (cursor.0 - pos.0, cursor.1 - pos.1),
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
#[component]
pub fn GraphPort(
  #[prop(into)] id: String,
  side: PortSide,
  #[prop(optional, into)] label: String,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let ctx = use_context::<GraphContext>().expect("GraphPort must be used within a GraphCanvas");
  let node = use_context::<NodeLocalContext>().expect("GraphPort must be used within a GraphNode");
  let dot = NodeRef::<leptos::html::Div>::new();
  let key = (node.id.clone(), id.clone());

  // Measure the socket center relative to the node once both are mounted.
  {
    let key = key.clone();
    let container = node.container;
    Effect::new(move |_| {
      let (Some(node_el), Some(dot_el)) = (container.get(), dot.get()) else {
        return;
      };
      let nr = node_el.get_bounding_client_rect();
      let dr = dot_el.get_bounding_client_rect();
      let rel = (
        dr.left() + dr.width() / 2.0 - nr.left(),
        dr.top() + dr.height() / 2.0 - nr.top(),
      );
      ctx.ports.update(|m| {
        m.insert(key.clone(), PortInfo { side, rel });
      });
    });
  }

  let start_key = key.clone();
  let on_down = move |ev: web_sys::PointerEvent| {
    ev.stop_propagation();
    let cursor = pointer_pos(&ctx.canvas, &ev);
    ctx.pending.set(Some(Pending {
      from: start_key.clone(),
      cursor,
    }));
  };
  let finish_key = key.clone();
  let on_up = move |ev: web_sys::PointerEvent| {
    ev.stop_propagation();
    if let Some(p) = ctx.pending.get_untracked() {
      if p.from != finish_key {
        let edge = GraphEdge {
          from: p.from,
          to: finish_key.clone(),
          color: None,
        };
        ctx.edges.update(|e| {
          if !e.iter().any(|x| x.from == edge.from && x.to == edge.to) {
            e.push(edge);
          }
        });
      }
      ctx.pending.set(None);
    }
  };

  let socket = view! {
    <div
      node_ref=dot
      class=cn(
        &[
          "h-3 w-3 shrink-0 cursor-crosshair rounded-full border-2 border-border bg-background transition-colors hover:border-primary hover:bg-primary",
          class.as_str(),
        ],
      )
      on:pointerdown=on_down
      on:pointerup=on_up
    />
  };

  let justify = if matches!(side, PortSide::Right) {
    "flex items-center justify-end gap-2"
  } else {
    "flex items-center gap-2"
  };

  if matches!(side, PortSide::Right) {
    view! {
      <div class=justify>
        <span class="text-xs text-muted-foreground">{label}</span>
        {socket}
      </div>
    }
    .into_any()
  } else {
    view! {
      <div class=justify>
        {socket}
        <span class="text-xs text-muted-foreground">{label}</span>
      </div>
    }
    .into_any()
  }
}
