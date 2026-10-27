#![allow(missing_docs, dead_code)]

//! Usage:
//!
//! web-sys = { version = "0.3.82", features = ["DomRect", "Element", "HtmlDivElement"] }

use crate::viewport::viewport_size;
use impulse_client_kit::utils::cn;
use impulse_client_kit::utils::{OverlayAlign, OverlaySide, calculate_position};
use leptos::prelude::*;

// `invisible` while closed goes with the collapse below it: the card stays
// mounted, and an element that is only `h-0 w-0 opacity-0` still holds its place
// in the tab order — so Tab walked into links inside a card nobody had hovered.
const BASE_CONTENT_CLASSES: &str = "bg-popover text-popover-foreground fixed z-50 w-64 rounded-md border p-4 shadow-md outline-hidden data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:invisible data-[state=closed]:opacity-0 data-[state=closed]:pointer-events-none data-[state=closed]:h-0 data-[state=closed]:w-0 data-[state=closed]:overflow-hidden";

#[component]
pub fn HoverCard(
  #[prop(optional)] open: Option<RwSignal<bool>>,
  #[prop(optional)] open_delay: Option<u32>,
  #[prop(optional)] close_delay: Option<u32>,
  /// Classes for the wrapper. It is the element a surrounding layout actually
  /// sees — the trigger is a level below it — so a hover card placed in a flex
  /// row needs `min-w-0` *here* or the row can never shrink its text.
  #[prop(optional, into)]
  class: String,
  children: Children,
) -> impl IntoView {
  let is_open = open.unwrap_or_else(|| RwSignal::new(false));
  let open_delay = open_delay.unwrap_or(700);
  let close_delay = close_delay.unwrap_or(300);

  // The trigger and the card are one hover region, so the timers that open and
  // close it live here rather than one pair in each: a card left towards its
  // trigger used to keep the close its own handler had scheduled, because the
  // trigger could only ever cancel a timer of its own.
  provide_context(HoverCardContext {
    is_open,
    open_delay,
    close_delay,
    content_ref: NodeRef::new(),
    open_timeout: StoredValue::new(None),
    close_timeout: StoredValue::new(None),
    last_point: StoredValue::new(None),
  });

  view! {
    <div data-slot="hover-card" class=class>
      {children()}
    </div>
  }
}

#[component]
pub fn HoverCardTrigger(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let context = use_context::<HoverCardContext>().expect("HoverCardTrigger must be used within HoverCard");

  let trigger_ref = NodeRef::<leptos::html::Div>::new();

  provide_context(HoverCardTriggerRef { trigger_ref });

  let handle_mouse_enter = move |ev: web_sys::MouseEvent| {
    context.last_point.set_value(Some((ev.client_x(), ev.client_y())));
    context.open_after_delay();
  };

  // Карточку показывают тому, кто остановился, а не тому, кто просто провёл
  // курсором. Отсчёт от входа этого не различает: достаточно задержаться в
  // широкой строке — везя курсор к тому, по чему собирались щёлкнуть, — и
  // карточка вылезает поверх цели. Поэтому каждое движение внутри начинает
  // отсчёт заново, и открывается она, только когда курсор постоял на месте.
  let handle_mouse_move = move |ev: web_sys::MouseEvent| {
    // Открытую не трогаем: решение уже принято, а движение по уже показанному
    // — не повод показать его ещё раз.
    if context.is_open.get_untracked() {
      return;
    }
    let now = (ev.client_x(), ev.client_y());
    if !moved_beyond_slop(context.last_point.get_value(), now) {
      return;
    }
    context.last_point.set_value(Some(now));
    context.open_after_delay();
  };

  let handle_mouse_leave = move |ev: web_sys::MouseEvent| {
    context.cancel_open();
    // Leaving the trigger *into the card* is not leaving. The card can end up
    // over its own trigger — a trigger at the bottom of the page with a card
    // too tall for the room left — and there the pointer never moves: the card
    // takes the hover, the trigger reports that it was left, closing uncovers
    // the trigger, which opens the card again. Whether the card is under the
    // pointer is a question about where the pointer *is*, so it is answered
    // that way, rather than by waiting for an enter event the card may never
    // get for a cursor that stood still.
    if pointer_within(context.content_ref, &ev) {
      return;
    }
    context.close_after_delay();
  };

  view! {
    <div
      node_ref=trigger_ref
      data-slot="hover-card-trigger"
      class=cn(&["inline-block", class.as_str()])
      on:mouseenter=handle_mouse_enter
      on:mousemove=handle_mouse_move
      on:mouseleave=handle_mouse_leave
    >
      {children()}
    </div>
  }
}

/// Насколько курсор должен сдвинуться, чтобы это считалось движением.
///
/// Нуля мало. Палец, лежащий на тачпаде неподвижно, всё равно даёт дрожание в
/// пиксель-другой, а прокрутка под неподвижным курсором в иных браузерах
/// присылает `mousemove` вовсе без сдвига, — и отсчёт, начинающийся заново на
/// каждое такое событие, не кончился бы никогда.
const REST_SLOP: i32 = 3;

/// Сдвинулся ли курсор настолько, чтобы считать, что он не стоит.
///
/// Запомненная точка обновляется только тогда, когда ответ «да»: иначе
/// медленный уход курсора, по пикселю за событие, так и остался бы стоянием —
/// каждый шаг в допуске, а сумма шагов через полэкрана.
fn moved_beyond_slop(last: Option<(i32, i32)>, now: (i32, i32)) -> bool {
  match last {
    // Где курсор был, неизвестно, — значит, он только что появился.
    None => true,
    Some((x, y)) => (now.0 - x).abs() > REST_SLOP || (now.1 - y).abs() > REST_SLOP,
  }
}

#[component]
pub fn HoverCardContent(
  #[prop(optional)] align: Option<OverlayAlign>,
  #[prop(optional)] side: Option<OverlaySide>,
  #[prop(optional)] side_offset: Option<i32>,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let context = use_context::<HoverCardContext>().expect("HoverCardContent must be used within HoverCard");

  let trigger_context = use_context::<HoverCardTriggerRef>();

  // The card's own node lives in the context: the trigger has to be able to ask
  // where it is before deciding that the pointer has left the pair.
  let content_ref = context.content_ref;
  let align = align.unwrap_or(OverlayAlign::Center);
  let side = side.unwrap_or(OverlaySide::Bottom);
  let side_offset = side_offset.unwrap_or(4);

  let position_style = RwSignal::new(String::new());

  // Position calculation
  Effect::new(move |_| {
    if context.is_open.get() {
      // Use requestAnimationFrame to ensure content is laid out
      request_animation_frame(move || {
        if let Some(trigger_ref) = trigger_context
          && let Some(trigger) = trigger_ref.trigger_ref.get()
          && let Some(content) = content_ref.get()
        {
          let trigger_rect = trigger.get_bounding_client_rect();
          let (viewport_width, viewport_height) = viewport_size();

          let (top, left) = calculate_position(
            trigger_rect.top(),
            trigger_rect.left(),
            trigger_rect.width(),
            trigger_rect.height(),
            content.offset_width() as f64,
            content.offset_height() as f64,
            side,
            align,
            side_offset,
            viewport_width,
            viewport_height,
          );

          position_style.set(format!("position: fixed; top: {}px; left: {}px;", top, left));
        }
      });
    }
  });

  let handle_mouse_enter = move |_| context.cancel_close();

  let handle_mouse_leave = move |_| context.close_after_delay();

  let slide_class = match side {
    OverlaySide::Top => "data-[state=open]:slide-in-from-bottom-2 data-[state=closed]:slide-out-to-bottom-2",
    OverlaySide::Right => "data-[state=open]:slide-in-from-left-2 data-[state=closed]:slide-out-to-left-2",
    OverlaySide::Bottom => "data-[state=open]:slide-in-from-top-2 data-[state=closed]:slide-out-to-top-2",
    OverlaySide::Left => "data-[state=open]:slide-in-from-right-2 data-[state=closed]:slide-out-to-right-2",
  };

  let children = StoredValue::new(children);
  let class = StoredValue::new(class);

  view! {
    <div
      node_ref=content_ref
      data-slot="hover-card-content"
      data-state=move || if context.is_open.get() { "open" } else { "closed" }
      class=cn(&[BASE_CONTENT_CLASSES, slide_class, class.read_value().as_str()])
      style=move || position_style.get()
      on:mouseenter=handle_mouse_enter
      on:mouseleave=handle_mouse_leave
    >
      {children.read_value()()}
    </div>
  }
}

/// Whether the pointer that raised `ev` is inside `node`'s box.
///
/// Asked of the geometry rather than of enter/leave events, because the case
/// that needs answering is a card that appeared *under* a cursor that never
/// moved — and a pointer that doesn't move raises no events to count.
fn pointer_within(node: NodeRef<leptos::html::Div>, ev: &web_sys::MouseEvent) -> bool {
  let Some(el) = node.get_untracked() else {
    return false;
  };
  let rect = el.get_bounding_client_rect();
  let (x, y) = (ev.client_x() as f64, ev.client_y() as f64);
  x >= rect.left() && x <= rect.right() && y >= rect.top() && y <= rect.bottom()
}

#[derive(Clone, Copy)]
struct HoverCardContext {
  is_open: RwSignal<bool>,
  open_delay: u32,
  close_delay: u32,
  /// The card itself, so the trigger can ask whether the pointer landed on it.
  content_ref: NodeRef<leptos::html::Div>,
  /// One pair of timers for the trigger and the card together: they are one
  /// hover region, and a close scheduled by either has to be cancellable by the
  /// other.
  open_timeout: StoredValue<Option<TimeoutHandle>>,
  close_timeout: StoredValue<Option<TimeoutHandle>>,
  /// Где курсор стоял, когда его видели в прошлый раз, — по нему и отличают
  /// стояние от езды.
  last_point: StoredValue<Option<(i32, i32)>>,
}

impl HoverCardContext {
  fn cancel_open(&self) {
    if let Some(handle) = self.open_timeout.get_value() {
      handle.clear();
      self.open_timeout.set_value(None);
    }
  }

  fn cancel_close(&self) {
    if let Some(handle) = self.close_timeout.get_value() {
      handle.clear();
      self.close_timeout.set_value(None);
    }
  }

  fn open_after_delay(&self) {
    // Свой прежний отсчёт — тоже отсчёт: без этого движение курсора не
    // переносило бы открытие, а назначало бы второе, и карточка вылезала бы по
    // первому, заведённому ещё на входе.
    self.cancel_open();
    self.cancel_close();
    let is_open = self.is_open;
    let handle = set_timeout_with_handle(
      move || is_open.set(true),
      std::time::Duration::from_millis(self.open_delay as u64),
    )
    .ok();
    self.open_timeout.set_value(handle);
  }

  fn close_after_delay(&self) {
    self.cancel_open();
    let is_open = self.is_open;
    let handle = set_timeout_with_handle(
      move || is_open.set(false),
      std::time::Duration::from_millis(self.close_delay as u64),
    )
    .ok();
    self.close_timeout.set_value(handle);
  }
}

#[derive(Clone, Copy)]
struct HoverCardTriggerRef {
  trigger_ref: NodeRef<leptos::html::Div>,
}

#[cfg(test)]
mod rest {
  use super::*;

  #[test]
  fn a_cursor_that_has_not_been_seen_yet_counts_as_moving() {
    assert!(moved_beyond_slop(None, (120, 80)));
  }

  #[test]
  fn a_trackpad_wobble_is_still_standing_still() {
    // Палец лежит неподвижно, а события приходят: открыться должно, и допуск
    // ровно за это.
    assert!(!moved_beyond_slop(Some((120, 80)), (120, 80)));
    assert!(!moved_beyond_slop(Some((120, 80)), (122, 78)));
    assert!(!moved_beyond_slop(Some((120, 80)), (117, 83)));
  }

  #[test]
  fn carrying_the_cursor_across_a_task_is_moving() {
    assert!(moved_beyond_slop(Some((120, 80)), (140, 80)));
    assert!(moved_beyond_slop(Some((120, 80)), (120, 60)));
  }

  /// Медленный уход — тоже уход: шаги в допуске складываются, потому что
  /// запомненная точка не сдвигается, пока ответ «стоит».
  #[test]
  fn a_slow_drift_eventually_counts_as_moving() {
    let start = (120, 80);
    let mut last = Some(start);
    let mut moved = false;
    for step in 1..=4 {
      let now = (start.0 + step, start.1);
      moved = moved_beyond_slop(last, now);
      if moved {
        last = Some(now);
      }
    }
    assert!(moved, "четыре шага по пикселю — это больше допуска");
  }
}
