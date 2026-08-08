#![allow(missing_docs, dead_code)]

use impulse_client_kit::utils::cn;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::{CssStyleDeclaration, Element, HtmlElement};

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
  /// The text sets it: the field grows as lines are added and shrinks as they
  /// go. `rows` (and `min-h-…`) is the floor it starts from; `max_rows` or a CSS
  /// `max-height` — `class="max-h-[70vh]"` — is the ceiling where growth stops
  /// and scrolling inside the field takes over. Given neither, it grows without
  /// limit and the page does the scrolling.
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
/// An `Auto` field takes a ceiling from `class="max-h-…"` as readily as from
/// `max_rows`, and that is the one to reach for when the ceiling is the screen
/// rather than a line count: `class="max-h-[70vh]"` says "as tall as the text
/// needs, but never taller than the viewport" on a phone and a desktop alike,
/// which no number of rows says on both. It is also what keeps a field holding
/// a very long document out of the page's own geometry: the browser paints the
/// lines that are on screen rather than a fifty-thousand-pixel box, and the
/// scrollbar beside the article stays the article's, not the field's.
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
    // watching the value will show that. The height is worth watching too — a
    // ceiling written in `vh` moves when the window does, and the field has to
    // be re-fitted under the new one — but only when it is not the height we
    // ourselves just set. Hence what is remembered is the size the field ended
    // up at rather than the size the observer reported: remember the latter and
    // our own resize comes straight back as news, which is the observer handed
    // its own tail to chase.
    let last_size = StoredValue::new((-1, -1));
    use_resize_observer(textarea_ref, move |_, _| {
      let Some(el) = textarea_ref.get_untracked() else {
        return;
      };
      if (el.client_width(), el.client_height()) != last_size.get_value() {
        fit_to_content(textarea_ref, max_rows);
        last_size.set_value((el.client_width(), el.client_height()));
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
  let auto = resizable == TextareaSizing::Auto;

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
        // A height measured before the webfont arrived is a height a line or two
        // short of the text, and a field that is short of its text scrolls
        // inside itself the moment a caret is put in it — the text lurches under
        // the click that placed it. Nothing in the value or the width changed,
        // so only this catches it, and by then it is one measurement away.
        on:focus=move |_| {
          if auto {
            fit_to_content(textarea_ref, max_rows);
          }
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
/// The `rows` attribute is the floor; the ceiling is `max_rows`, a CSS
/// `max-height`, or whichever of the two is lower. Between them the field is as
/// tall as its content and shows no scrollbar of its own; at the ceiling the
/// rest of the text is reached by scrolling inside the field.
fn fit_to_content(textarea_ref: NodeRef<leptos::html::Textarea>, max_rows: Option<i32>) {
  let Some(textarea) = textarea_ref.get_untracked() else {
    return;
  };
  let el: &HtmlElement = textarea.as_ref();
  let style = el.style();

  // `scroll_height` counts the padding but not the border, while `box-sizing:
  // border-box` puts the border inside `height`. The gap between the offset and
  // client heights is that border — cheaper and surer than parsing computed
  // widths, and with wrapping on there is never a horizontal scrollbar in it.
  let border = (el.offset_height() - el.client_height()).max(0);
  let css_ceiling = css_max_height(el);

  // Growing — which is what typing does — needs no measurement at all: the field
  // is already shorter than its text, and a box shorter than its text reports
  // the text's full height as `scroll_height`. Worth its own path, because the
  // measurement below is the one moment this function makes the page *shorter*,
  // and a page that gets shorter takes every scroll position above it along.
  //
  // A ceiling in rows is the exception: reading it means giving the field its
  // natural height for a moment, so `max_rows` always takes the long way round.
  if max_rows.is_none() && el.scroll_height() > el.client_height() {
    let mut height = el.scroll_height() + border;
    if let Some(ceiling) = css_ceiling {
      height = height.min(ceiling);
    }
    apply_height(&style, height, css_ceiling.is_some());
    return;
  }

  // Shrinking (or a ceiling in rows). `auto` first, or the field could never
  // shrink again: `scroll_height` reports the taller of the content and the box
  // it is already in. Left to itself a textarea is exactly `rows` lines tall,
  // which makes this measurement the floor as well.
  //
  // For the length of that measurement the field is as short as `rows`, and so
  // is everything scrollable above it. The browser clamps a scroll offset the
  // page no longer reaches and does *not* hand it back when the height returns —
  // which is a reader thrown to the top of a long document by their own first
  // keystroke. Hence the snapshot: it is put back before this function returns,
  // in the same turn, so the collapsed page is never painted or scrolled from.
  let scrolls = scroll_snapshot(el);
  let _ = style.set_property("height", "auto");
  let floor = el.offset_height();
  let content = el.scroll_height() + border;

  // A ceiling given in rows is the browser's arithmetic to do — the same "no
  // height of its own" measurement, with `rows` borrowed for a moment.
  let ceiling = max_rows
    .map(|max_rows| {
      let asked = textarea.rows();
      textarea.set_rows(max_rows.max(1) as u32);
      let by_rows = el.offset_height();
      textarea.set_rows(asked);
      css_ceiling.map_or(by_rows, |css| by_rows.min(css))
    })
    .or(css_ceiling);

  let mut height = content.max(floor);
  if let Some(ceiling) = ceiling {
    height = height.min(ceiling);
  }
  apply_height(&style, height, ceiling.is_some());
  restore_scroll(&scrolls);
}

/// Writes a fitted height back, and says whether the field may scroll inside
/// itself.
///
/// Without a ceiling the box is exactly its content and `hidden` is honest:
/// there is nothing to scroll to, and a scrollbar would only flicker while the
/// field grows. With one, `auto` — and not only when the text overflows *right
/// now*. A ceiling in `vh` moves on its own between two fits (a rotated phone, a
/// resized window), and `hidden` under a ceiling that has just shrunk swallows
/// the text below it with no way to reach it, while `auto` simply produces the
/// scrollbar the moment there is something to scroll.
fn apply_height(style: &CssStyleDeclaration, height: i32, has_ceiling: bool) {
  let _ = style.set_property("height", &format!("{height}px"));
  let _ = style.set_property("overflow-y", if has_ceiling { "auto" } else { "hidden" });
}

/// The CSS `max-height` in force on the field, in whole pixels.
///
/// `class="max-h-[70vh]"` is how an `Auto` field says "stop at the screen", the
/// same way `class="min-h-…"` already sets its floor — and unlike `max_rows` it
/// says it in a unit that means the same thing on every display. Anything the
/// browser does not resolve to pixels (`none`, or a percentage of a parent with
/// no height of its own) reads as no ceiling at all, which is what an `Auto`
/// field did before this existed.
fn css_max_height(el: &HtmlElement) -> Option<i32> {
  let element: &Element = el.as_ref();
  let computed = web_sys::window()?.get_computed_style(element).ok()??;
  let raw = computed.get_property_value("max-height").ok()?;
  let px = raw.strip_suffix("px")?.parse::<f64>().ok()?;
  Some(px.round() as i32)
}

/// Every scrolling box above the field, with the offset it is scrolled to.
///
/// Anything with more content than box either is scrolled now or will be clamped
/// when the page below it briefly gets shorter, and both want putting back.
fn scroll_snapshot(el: &HtmlElement) -> Vec<(Element, i32)> {
  let mut snapshot = Vec::new();
  let mut next = el.parent_element();
  while let Some(parent) = next {
    if parent.scroll_height() > parent.client_height() {
      snapshot.push((parent.clone(), parent.scroll_top()));
    }
    next = parent.parent_element();
  }
  snapshot
}

/// Puts a [`scroll_snapshot`] back — but only where it actually moved, so a box
/// that was left alone is never handed a scroll it has to animate or report.
fn restore_scroll(snapshot: &[(Element, i32)]) {
  for (el, top) in snapshot {
    if el.scroll_top() != *top {
      el.set_scroll_top(*top);
    }
  }
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
