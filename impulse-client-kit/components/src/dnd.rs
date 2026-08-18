#![allow(missing_docs, dead_code)]

//! Drag-and-drop built on pointer events rather than the HTML5 drag API.
//!
//! `draggable="true"` plus `dragstart`/`dragover`/`drop` is the obvious way to
//! move something with the mouse, and it is the one thing in the platform that
//! cannot be relied on: WebKitGTK — the engine every Tauri window on Linux runs
//! — never starts a drag for a plain element, so a board built on it is simply
//! immovable there. Touch is worse: no mobile engine synthesises HTML5 drag
//! events at all, so the same code is dead on a phone.
//!
//! Pointer events have none of that: `pointerdown`/`pointermove`/`pointerup`
//! fire identically for mouse, pen and touch in every engine. The cost is that
//! the browser no longer tells us what is under the pointer — a captured
//! pointer stops firing `pointerover` on anything else — so drop targets are
//! resolved by hit-testing [`Document::element_from_point`] on each move and
//! walking up to the enclosing zones. That is also what makes nesting work: the
//! walk yields the whole chain, so a row inside a column marks *both* as hovered
//! and a drop is offered to the innermost zone first.
//!
//! ```ignore
//! view! {
//!   <DndProvider>
//!     <DropZone kind="column" id=col_id on_drop=Callback::new(move |item: DragId| {
//!       if item.kind == "row" { move_row(item.id, col_id); true } else { false }
//!     })>
//!       <For each=rows key=|r| *r let:row>
//!         <Draggable kind="row" id=row class="data-[dnd-dragging=true]:opacity-50">
//!           {row_view(row)}
//!         </Draggable>
//!       </For>
//!     </DropZone>
//!   </DndProvider>
//! }
//! ```
//!
//! Activation is deliberately not immediate. A mouse or pen has to travel
//! [`DRAG_SLOP`] pixels first, so a click on a button inside a draggable stays a
//! click; a finger has to rest for [`LONG_PRESS_MS`] before it moves, so a swipe
//! across a list still scrolls it. Nothing here calls `preventDefault` until a
//! drag has actually begun, which is what keeps both of those true.

use impulse_client_kit::utils::cn;
use leptos::ev;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::{Element, PointerEvent};

/// How far a mouse or pen must travel before a press becomes a drag.
pub const DRAG_SLOP: f64 = 5.0;
/// How long a finger must rest before a press becomes a drag.
pub const LONG_PRESS_MS: f64 = 350.0;
/// How far a finger may stray during the hold before the press is read as a
/// scroll instead.
pub const TOUCH_SLOP: f64 = 10.0;

/// What is being dragged, or what it is being dropped on: a caller-defined
/// `kind` (`"task"`, `"column"`, …) plus the id of the thing itself.
///
/// One namespace is shared by draggables and drop zones, so a component can be
/// both — a row that other rows drop onto is the usual case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DragId {
  pub kind: String,
  pub id: i64,
}

impl DragId {
  pub fn new(kind: impl Into<String>, id: i64) -> Self {
    Self { kind: kind.into(), id }
  }

  /// Whether this is exactly `(kind, id)`.
  pub fn is(&self, kind: &str, id: i64) -> bool {
    self.kind == kind && self.id == id
  }
}

/// A press that has not (yet) become a drag.
#[derive(Clone, Debug, PartialEq)]
struct Pending {
  item: DragId,
  pointer_id: i32,
  origin: (f64, f64),
  /// `performance.now()` at `pointerdown`, for the touch hold.
  started_at: f64,
  touch: bool,
}

/// A registered drop target. Callbacks answer `true` when they took the drop,
/// which is how a zone declines a kind it doesn't handle and lets the drop fall
/// through to the zone around it.
#[derive(Clone)]
struct Zone {
  id: DragId,
  on_drop: Callback<DragId, bool>,
}

/// The drag state shared by everything inside a [`DndProvider`]. Pull it with
/// [`use_dnd`] to drive your own reactive styling; the `data-dnd-*` attributes
/// [`Draggable`] and [`DropZone`] already set cover the common cases.
#[derive(Clone, Copy)]
pub struct DndContext {
  active: RwSignal<Option<DragId>>,
  /// Every zone under the pointer, innermost first.
  over_path: RwSignal<Vec<DragId>>,
  pending: RwSignal<Option<Pending>>,
  zones: StoredValue<Vec<Zone>>,
}

impl DndContext {
  fn new() -> Self {
    Self {
      active: RwSignal::new(None),
      over_path: RwSignal::new(Vec::new()),
      pending: RwSignal::new(None),
      zones: StoredValue::new(Vec::new()),
    }
  }

  /// The item currently being dragged, if any (reactive).
  pub fn active(&self) -> Option<DragId> {
    self.active.get()
  }

  /// Whether a drag is in progress at all (reactive).
  pub fn is_active(&self) -> bool {
    self.active.get().is_some()
  }

  /// Whether `(kind, id)` is the item being dragged (reactive).
  pub fn is_dragging(&self, kind: &str, id: i64) -> bool {
    self.active.get().is_some_and(|a| a.is(kind, id))
  }

  /// The innermost drop zone under the pointer (reactive).
  pub fn over(&self) -> Option<DragId> {
    self.over_path.get().first().cloned()
  }

  /// Whether the pointer is over `(kind, id)` — true for a zone that merely
  /// *contains* the innermost one, so a column stays highlighted while the
  /// pointer sits on one of its rows (reactive).
  pub fn is_over(&self, kind: &str, id: i64) -> bool {
    self.over_path.get().iter().any(|z| z.is(kind, id))
  }

  fn register(&self, zone: Zone) {
    self.zones.update_value(|z| z.push(zone));
  }

  fn unregister(&self, id: &DragId) {
    self.zones.update_value(|z| z.retain(|x| &x.id != id));
  }

  fn start(&self, pending: Pending) {
    self.pending.set(Some(pending));
  }

  /// Resolves the chain of drop zones under `(x, y)`, innermost first.
  fn zones_at(x: f64, y: f64) -> Vec<DragId> {
    let mut out = Vec::new();
    let mut node = document().element_from_point(x as f32, y as f32);
    while let Some(el) = node {
      if let (Some(kind), Some(id)) = (
        el.get_attribute("data-dnd-zone-kind"),
        el.get_attribute("data-dnd-zone-id"),
      ) && let Ok(id) = id.parse::<i64>()
      {
        out.push(DragId { kind, id });
      }
      node = el.parent_element();
    }
    out
  }

  fn track(&self, x: f64, y: f64) {
    let found = Self::zones_at(x, y);
    if self.over_path.get_untracked() != found {
      self.over_path.set(found);
    }
  }

  fn on_pointer_move(&self, ev: PointerEvent) {
    let (x, y) = (ev.client_x() as f64, ev.client_y() as f64);
    if self.active.get_untracked().is_some() {
      // Only now is the gesture ours: suppress the text selection (and, on
      // touch, the scroll) the browser would otherwise run alongside it.
      ev.prevent_default();
      self.track(x, y);
      return;
    }
    let Some(p) = self.pending.get_untracked() else {
      return;
    };
    if p.pointer_id != ev.pointer_id() {
      return;
    }
    let moved = (x - p.origin.0).abs().max((y - p.origin.1).abs());
    if p.touch {
      // A finger that moves before the hold is over is scrolling the list, not
      // picking anything up — let the browser have it.
      if now_ms() - p.started_at < LONG_PRESS_MS {
        if moved > TOUCH_SLOP {
          self.pending.set(None);
        }
        return;
      }
    } else if moved < DRAG_SLOP {
      return;
    }
    ev.prevent_default();
    self.active.set(Some(p.item));
    self.track(x, y);
  }

  fn on_pointer_up(&self, _ev: PointerEvent) {
    let item = self.active.get_untracked();
    let path = self.over_path.get_untracked();
    self.reset();
    let Some(item) = item else { return };
    // Innermost zone first; a zone that declines the kind passes it outwards.
    for zone in path {
      let handler = self
        .zones
        .with_value(|z| z.iter().find(|x| x.id == zone).map(|x| x.on_drop));
      if let Some(handler) = handler
        && handler.run(item.clone())
      {
        return;
      }
    }
  }

  fn reset(&self) {
    self.pending.set(None);
    if self.active.get_untracked().is_some() {
      self.active.set(None);
    }
    if !self.over_path.get_untracked().is_empty() {
      self.over_path.set(Vec::new());
    }
  }
}

/// Milliseconds on the monotonic clock, falling back to the wall clock where
/// `performance` is unavailable. Only differences over a few hundred
/// milliseconds are read from it, which either clock measures well enough.
fn now_ms() -> f64 {
  window()
    .performance()
    .map(|p| p.now())
    .unwrap_or_else(js_sys::Date::now)
}

/// Provides the drag state and owns the window-level pointer listeners. Mount
/// it once, around everything that drags.
///
/// The listeners live on the window rather than on each draggable because a
/// pointer that leaves the element mid-drag — which is the entire point of
/// dragging — stops delivering events to it.
#[component]
pub fn DndProvider(children: Children) -> impl IntoView {
  let ctx = DndContext::new();
  provide_context(ctx);

  window_event_listener(ev::pointermove, move |ev| ctx.on_pointer_move(ev));
  window_event_listener(ev::pointerup, move |ev| ctx.on_pointer_up(ev));
  window_event_listener(ev::pointercancel, move |_| ctx.reset());
  // A drag interrupted by anything the page didn't see (a context menu, the
  // window losing focus) must not leave the board stuck in "dragging".
  window_event_listener(ev::blur, move |_| ctx.reset());
  // Escape is how every other drag on the platform is called off.
  window_event_listener(ev::keydown, move |ev| {
    if ev.key() == "Escape" {
      ctx.reset();
    }
  });

  view! {
    <div
      data-slot="dnd-provider"
      data-dnd-active=move || ctx.is_active().then_some("true")
      class="contents data-[dnd-active=true]:cursor-grabbing data-[dnd-active=true]:select-none"
    >
      {children()}
    </div>
  }
}

/// The drag state of the enclosing [`DndProvider`].
///
/// # Panics
///
/// If called outside a [`DndProvider`].
pub fn use_dnd() -> DndContext {
  use_context::<DndContext>().expect("use_dnd must be called within a DndProvider")
}

/// Something that can be picked up.
///
/// Sets `data-dnd-dragging="true"` on itself while it is the item being
/// dragged, so the lifted look is a class (`data-[dnd-dragging=true]:opacity-50`)
/// rather than a signal the caller has to thread through.
///
/// Where the press has to land depends on what is inside. By default anywhere
/// but a control (`button`, `a`, `input`, …) starts the drag — a press on a
/// control belongs to that control, or a stray pixel of movement while clicking
/// a row's delete button would turn the click into a drag and the button would
/// never fire. Put a [`DragHandle`] inside and that flips: only the handle
/// starts a drag. That is the answer for a draggable that *is* a control — a
/// whole card rendered as a button — where the default rule would exclude
/// everything.
#[component]
pub fn Draggable(
  #[prop(into)] kind: String,
  id: i64,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let ctx = use_dnd();
  let kind_for_state = kind.clone();
  let root = NodeRef::<leptos::html::Div>::new();

  let on_pointer_down = move |ev: PointerEvent| {
    // Secondary buttons open menus; they never start a drag.
    if disabled || ev.button() != 0 {
      return;
    }
    let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
      return;
    };
    let root: Option<Element> = root.get_untracked().map(|el| el.into());
    let has_handle = root
      .as_ref()
      .and_then(|el| el.query_selector("[data-dnd-handle]").ok().flatten())
      .is_some();
    let starts_drag = if has_handle {
      // The handle must be one of *ours*: a nested draggable's handle drags that
      // one, not this.
      match target.closest("[data-dnd-handle]").ok().flatten() {
        Some(handle) => root.as_ref().is_some_and(|el| el.contains(Some(&handle))),
        None => false,
      }
    } else {
      target
        .closest("button, a, input, textarea, select, [contenteditable=true], [data-dnd-no-drag]")
        .ok()
        .flatten()
        .is_none()
    };
    if !starts_drag {
      return;
    }
    ctx.start(Pending {
      item: DragId::new(kind.clone(), id),
      pointer_id: ev.pointer_id(),
      origin: (ev.client_x() as f64, ev.client_y() as f64),
      started_at: now_ms(),
      touch: ev.pointer_type() == "touch",
    });
  };

  view! {
    <div
      node_ref=root
      data-slot="draggable"
      data-dnd-dragging=move || ctx.is_dragging(&kind_for_state, id).then_some("true")
      // The native drag API is what this component exists to avoid: left on, it
      // hijacks the gesture in the engines that do implement it (and drags the
      // text or the image under the pointer in all of them).
      draggable="false"
      on:dragstart=|ev: web_sys::DragEvent| ev.prevent_default()
      on:pointerdown=on_pointer_down
      class=cn(
        &[
          if disabled { "" } else { "touch-pan-y select-none" },
          class.as_str(),
        ],
      )
    >
      {children()}
    </div>
  }
}

/// The grab area of a [`Draggable`] that contains one.
///
/// Wrap the grip icon (or whatever the user is meant to pull on); everything
/// else in the draggable then behaves normally, which is what makes a clickable
/// card movable without its click becoming ambiguous.
#[component]
pub fn DragHandle(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <span
      data-slot="drag-handle"
      data-dnd-handle="true"
      aria-hidden="true"
      class=cn(&["inline-flex cursor-grab touch-none select-none active:cursor-grabbing", class.as_str()])
    >
      {children()}
    </span>
  }
}

/// Somewhere a [`Draggable`] can be dropped.
///
/// `on_drop` receives the dragged item and answers whether it took it: `false`
/// passes the drop out to the enclosing zone, which is how a row can accept row
/// drags while letting a column drag reach the column it sits in.
///
/// While the pointer is inside, the element carries `data-dnd-over="true"` and
/// `data-dnd-over-kind="<kind of the dragged item>"` — both set on every zone in
/// the chain, not just the innermost, so an outer zone stays lit while the
/// pointer is over one of its children.
#[component]
pub fn DropZone(
  #[prop(into)] kind: String,
  id: i64,
  on_drop: Callback<DragId, bool>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let ctx = use_dnd();
  let zone_id = DragId::new(kind.clone(), id);
  ctx.register(Zone {
    id: zone_id.clone(),
    on_drop,
  });
  let registered = zone_id.clone();
  on_cleanup(move || ctx.unregister(&registered));

  // A memo, not a closure: two attributes read it, and a `Memo` is `Copy`.
  let over_kind = Memo::new(move |_| {
    ctx
      .is_over(&zone_id.kind, zone_id.id)
      .then(|| ctx.active().map(|a| a.kind))
      .flatten()
  });

  view! {
    <div
      data-slot="drop-zone"
      data-dnd-zone-kind=kind
      data-dnd-zone-id=id.to_string()
      data-dnd-over=move || over_kind.get().map(|_| "true")
      data-dnd-over-kind=move || over_kind.get()
      class=class
    >
      {children()}
    </div>
  }
}
