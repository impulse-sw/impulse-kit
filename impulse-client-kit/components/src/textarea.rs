#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::HtmlElement;

// `resize-none` turns off the native corner grip. It is drawn by the platform,
// not by us: on Android it is a barely-visible white speck in the bottom-right
// corner that most users never find, and every platform draws a different one.
// The grabber below replaces it with a single control that looks the same
// everywhere.
//
// No `min-h-*` here on purpose. `cn` concatenates, so a caller's class and ours
// land in the same layer and Tailwind — not the class list — picks the winner:
// it emits candidates of one utility in name order, so `min-h-[80px]` came out
// after `min-h-[28rem]` and quietly won. The starting height is `rows`' job
// (four lines by default), which leaves `class="min-h-…"` free to do exactly
// what the docs promise.
const BASE_CLASSES: &str = "placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input flex w-full resize-none rounded-md border bg-transparent px-3 py-2 text-base shadow-xs transition-[color,box-shadow] outline-none disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive";

/// How far a single arrow-key press moves the grabber, in pixels.
const KEYBOARD_STEP: f64 = 16.0;
/// A floor for the dragged height; CSS `min-height` still applies on top of it.
const MIN_HEIGHT: f64 = 32.0;

/// Where a [`Textarea`]'s height comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextareaSizing {
  /// The reader sets it, by dragging the grabber under the field.
  #[default]
  Grabber,
  /// Whatever `rows` and the classes say, and nothing moves it afterwards.
  Fixed,
  /// The text sets it: the field grows as lines are added, shrinks as they go,
  /// and never scrolls inside itself — `rows` is the floor it starts from and
  /// `max_rows`, if given, the ceiling where scrolling comes back.
  Auto,
}

impl From<bool> for TextareaSizing {
  /// Keeps `resizable=true` / `resizable=false` meaning what they always did.
  fn from(grabber: bool) -> Self {
    if grabber { Self::Grabber } else { Self::Fixed }
  }
}

/// A multi-line text field, sized one of three ways.
///
/// * `resizable` — a [`TextareaSizing`]; `Grabber` by default. `false` still
///   spells `Fixed` and `true` still spells `Grabber`. The *native* corner
///   resizer is off in every mode.
/// * `max_rows` — `Auto` only: the height at which the field stops growing and
///   starts scrolling. Left out, it grows without limit. Set below `rows`, it
///   wins.
///
/// The field opens `rows` lines tall (four by default) and nothing in the base
/// styling fixes a height beyond that, so `class="min-h-…"` / `class="h-…"` is
/// how a taller field — an editor pane, say — asks for its size.
///
/// The grabber is a pill centred under the field (`mt-2` below it), dragged with
/// a pointer — mouse, touch or pen alike — or moved with ↑/↓ when focused.
#[component]
pub fn Textarea(
  #[prop(into, optional)] class: String,
  #[prop(optional)] value: RwSignal<String>,
  #[prop(optional, into)] placeholder: String,
  #[prop(optional)] disabled: bool,
  #[prop(optional)] rows: Option<i32>,
  #[prop(optional, into)] resizable: TextareaSizing,
  #[prop(optional)] max_rows: Option<i32>,
) -> impl IntoView {
  let textarea_ref = NodeRef::<leptos::html::Textarea>::new();

  if resizable == TextareaSizing::Auto {
    use leptos_use::use_resize_observer;

    // Every change to the text can change the line count — typing, a paste, or
    // a `value.set` from elsewhere. Measuring in a frame callback is what the
    // rest of the kit does: by then the new text is in the DOM, and it is still
    // before the next paint, so no frame is ever drawn a line short.
    Effect::new(move |_| {
      value.track();
      request_animation_frame(move || fit_to_content(textarea_ref, max_rows));
    });

    // The same text wraps into more lines in a narrower field, and no amount of
    // watching the value will show that. Only the width is worth reacting to:
    // the height is what we just set, and re-fitting on our own change would
    // hand the observer its own tail to chase.
    let last_width = StoredValue::new(-1);
    use_resize_observer(textarea_ref, move |_, _| {
      let Some(el) = textarea_ref.get_untracked() else {
        return;
      };
      let width = el.client_width();
      if width != last_width.get_value() {
        last_width.set_value(width);
        fit_to_content(textarea_ref, max_rows);
      }
    });
  }

  // Until the first measurement lands, `overflow-hidden` keeps a scrollbar from
  // flashing up in a field that is about to grow past the need for one.
  let sizing_classes = match resizable {
    TextareaSizing::Auto => "overflow-hidden",
    _ => "",
  };
  let show_grabber = resizable == TextareaSizing::Grabber && !disabled;

  view! {
    <div data-slot="textarea-wrapper" class="flex w-full flex-col">
      <textarea
        node_ref=textarea_ref
        data-slot="textarea"
        class=cn(&[BASE_CLASSES, sizing_classes, class.as_str()])
        prop:value=value
        placeholder=placeholder
        disabled=disabled
        rows=rows.unwrap_or(4)
        on:input:target=move |ev| {
          value.set(ev.target().value());
        }
      />
      <Show when=move || show_grabber>
        <TextareaResizeHandle textarea_ref=textarea_ref />
      </Show>
    </div>
  }
}

/// Sets an `Auto` field's height to exactly what its text needs.
///
/// The `rows` attribute is the floor and `max_rows` the ceiling; between them the
/// field is as tall as its content and shows no scrollbar of its own.
fn fit_to_content(textarea_ref: NodeRef<leptos::html::Textarea>, max_rows: Option<i32>) {
  let Some(textarea) = textarea_ref.get_untracked() else {
    return;
  };
  let el: &HtmlElement = textarea.as_ref();
  let style = el.style();

  // `auto` first, or the field could never shrink again: `scroll_height` reports
  // the taller of the content and the box it is already in. Left to itself a
  // textarea is exactly `rows` lines tall, which makes this measurement the
  // floor as well.
  let _ = style.set_property("height", "auto");
  let floor = el.offset_height();
  // `scroll_height` counts the padding but not the border, while `box-sizing:
  // border-box` puts the border inside `height`. The gap between the offset and
  // client heights is that border — cheaper and surer than parsing computed
  // widths, and with wrapping on there is never a horizontal scrollbar in it.
  let content = el.scroll_height() + (floor - el.client_height()).max(0);

  // A ceiling is given in rows, so let the browser do the row arithmetic — the
  // same "no height of its own" measurement, with `rows` borrowed for a moment.
  let ceiling = max_rows.map(|max_rows| {
    let asked = textarea.rows();
    textarea.set_rows(max_rows.max(1) as u32);
    let ceiling = el.offset_height();
    textarea.set_rows(asked);
    ceiling
  });

  let mut height = content.max(floor);
  if let Some(ceiling) = ceiling {
    height = height.min(ceiling);
  }
  let _ = style.set_property("height", &format!("{height}px"));
  // Below the ceiling there is nothing to scroll to; at it, scrolling is the
  // only way to reach the rest of the text.
  let overflowing = ceiling.is_some_and(|ceiling| content > ceiling);
  let _ = style.set_property("overflow-y", if overflowing { "auto" } else { "hidden" });
}

/// The grabber: an old-iOS-style pill under the field, dragged to resize it.
#[component]
fn TextareaResizeHandle(textarea_ref: NodeRef<leptos::html::Textarea>) -> impl IntoView {
  // Where the drag started, and how tall the field was then: `(client_y, height)`.
  let drag = RwSignal::new(None::<(f64, f64)>);

  let height = move || {
    textarea_ref
      .get_untracked()
      .map(|el| el.get_bounding_client_rect().height())
  };

  let set_height = move |px: f64| {
    if let Some(el) = textarea_ref.get_untracked() {
      let el: &HtmlElement = el.as_ref();
      let _ = el.style().set_property("height", &format!("{}px", px.max(MIN_HEIGHT)));
    }
  };

  let start = move |ev: web_sys::PointerEvent| {
    let Some(height) = height() else {
      return;
    };
    // Keep the press from selecting the text above it while dragging.
    ev.prevent_default();
    drag.set(Some((ev.client_y() as f64, height)));
    // Capturing the pointer routes the rest of the gesture back here even when
    // it leaves the handle — which it does immediately, that being the point.
    if let Some(target) = ev.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
      let _ = target.set_pointer_capture(ev.pointer_id());
    }
  };

  let drag_to = move |ev: web_sys::PointerEvent| {
    if let Some((origin, height)) = drag.get_untracked() {
      set_height(height + (ev.client_y() as f64 - origin));
    }
  };

  let finish = move |_: web_sys::PointerEvent| drag.set(None);

  let keyboard = move |ev: web_sys::KeyboardEvent| {
    let step = match ev.key().as_str() {
      "ArrowDown" => KEYBOARD_STEP,
      "ArrowUp" => -KEYBOARD_STEP,
      _ => return,
    };
    let Some(height) = height() else {
      return;
    };
    ev.prevent_default();
    set_height(height + step);
  };

  view! {
    <div
      data-slot="textarea-resize-handle"
      role="separator"
      aria-orientation="horizontal"
      aria-label="Resize"
      tabindex="0"
      // `touch-none` hands the gesture to us instead of scrolling the page.
      class="group mt-2 flex h-4 w-full shrink-0 cursor-ns-resize touch-none items-center justify-center rounded-full outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
      on:pointerdown=start
      on:pointermove=drag_to
      on:pointerup=finish
      on:pointercancel=finish
      on:keydown=keyboard
    >
      <div class="h-1 w-10 rounded-full bg-muted-foreground/30 transition-colors group-hover:bg-muted-foreground/60 group-active:bg-muted-foreground/70" />
    </div>
  }
}
