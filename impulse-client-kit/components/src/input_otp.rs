#![allow(missing_docs, dead_code)]

//! One-time-password entry.
//!
//! The field *looks* like one box per digit, but it is a single `<input>`
//! holding the whole code, stretched transparently across a row of
//! presentational slots.
//!
//! The obvious construction — one `<input maxlength="1">` per digit, moving the
//! focus along on every keystroke — cannot be made to work on a phone. A
//! software keyboard is an input method, and input methods report their keys as
//! `keydown` with `key` = `"Unidentified"` (`keyCode` 229); the character itself
//! only ever surfaces in the `input` event that follows. So the one rule such a
//! field needs most — "Backspace in an empty box clears the previous one and
//! steps back" — asks a question the browser refuses to answer there, and the
//! code becomes unerasable: the box under the caret is already empty, so the
//! keypress deletes nothing, fires no `input`, and the component never hears
//! about it.
//!
//! With a single input there is nothing left to detect. Backspace deletes the
//! character before the caret because there *is* one; selection, autorepeat,
//! paste, undo and the platform's SMS-code autofill (`autocomplete`
//! `one-time-code`, which a per-digit field cannot offer at all) are the
//! browser's own. All this component does is keep the value to digits, keep the
//! caret at the end, and paint the boxes.

use impulse_client_kit::utils::cn;
use leptos::prelude::*;
use std::ops::Range;

/// One digit box. Presentational only — it is painted under the real input, and
/// nothing in it can take focus, so the `:focus` variants the boxes used to
/// style themselves with are gone; see [`SLOT_ACTIVE_CLASSES`].
const SLOT_CLASSES: &str = "relative flex h-9 w-9 items-center justify-center border-y border-r border-input bg-transparent text-sm shadow-xs transition-all";

/// Drawn on the slot the caret sits in, and only while the field has the focus —
/// it stands in for a `:focus` ring, and a blurred field must not look focused.
/// `z-10` lifts the ring over the neighbouring borders, the way `focus:z-10` did.
const SLOT_ACTIVE_CLASSES: &str = "z-10 border-ring ring-[3px] ring-ring/50";

/// The invisible overlay that is the actual field. It covers the whole row so a
/// tap anywhere focuses it, and sits above the slots (`z-20` clears the active
/// slot's `z-10`) so nothing else can swallow that tap.
///
/// `text-base` is not cosmetic: iOS zooms the page in on a focused field whose
/// text is smaller than 16px, and it does that by the computed font size, not by
/// whether the text can be seen.
const INPUT_CLASSES: &str = "absolute inset-0 z-20 h-full w-full border-0 bg-transparent p-0 text-base text-transparent caret-transparent outline-none selection:bg-transparent selection:text-transparent";

/// Keyframes for the fake caret. The real one is hidden (`caret-transparent`)
/// because it would sit wherever the transparent text happens to lay out, which
/// has nothing to do with where the boxes are.
const CARET_STYLES: &str = r#"
@keyframes otp-caret-blink {
  0%, 49% { opacity: 1; }
  50%, 100% { opacity: 0; }
}

.otp-caret {
  animation: otp-caret-blink 1s steps(1) infinite;
}

@media (prefers-reduced-motion: reduce) {
  .otp-caret { animation: none; }
}
"#;

/// Shared body of [`InputOTP`] and [`InputOTPWithSeparator`]. `separator_at`
/// splits the row into two groups at that index; `None` renders one group.
fn otp_field(
  length: usize,
  separator_at: Option<usize>,
  value: RwSignal<String>,
  on_change: Option<Callback<String>>,
  on_complete: Option<Callback<String>>,
  class: String,
  container_class: String,
) -> impl IntoView {
  let focused = RwSignal::new(false);
  let input_ref = NodeRef::<leptos::html::Input>::new();

  // ASCII only, deliberately: `char::is_numeric` also accepts digits from other
  // scripts, which look like a code and are not the one the server issued.
  let sanitize = move |raw: &str| -> String { raw.chars().filter(|c| c.is_ascii_digit()).take(length).collect() };

  // A code is entered and erased from the end, so the caret belongs there —
  // otherwise tapping the fourth box drops it wherever the invisible text
  // happens to fall and the next digit lands mid-code.
  let pin_caret = move || {
    let Some(input) = input_ref.get_untracked() else {
      return;
    };

    let end = input.value().chars().count() as u32;
    let start_now = input.selection_start().ok().flatten().unwrap_or(end);
    let end_now = input.selection_end().ok().flatten().unwrap_or(end);

    // Leave a select-all alone: typing over the whole code is a legitimate way
    // to start again, and re-pinning here would also loop through `on:select`.
    if start_now == 0 && end_now == end && end > 0 {
      return;
    }

    if start_now != end || end_now != end {
      let _ = input.set_selection_range(end, end);
    }
  };

  let handle_input = move |ev: leptos::ev::Event| {
    let input = event_target::<leptos::web_sys::HtmlInputElement>(&ev);
    let raw = input.value();
    let next = sanitize(&raw);

    // Written back even when the signal below does not change (a rejected
    // character leaves the value as it was), or the stray character stays in the
    // DOM: `prop:value` only rewrites the element when the signal actually moves.
    if next != raw {
      input.set_value(&next);
    }

    pin_caret();

    if next == value.get_untracked() {
      return;
    }

    value.set(next.clone());

    if let Some(on_change) = on_change {
      on_change.run(next.clone());
    }

    if let Some(on_complete) = on_complete
      && next.chars().count() == length
    {
      on_complete.run(next);
    }
  };

  let active_index = move || value.get().chars().count().min(length.saturating_sub(1));

  let group = move |range: Range<usize>| {
    let (first, last) = (range.start, range.end.saturating_sub(1));

    range
      .map(|index| {
        let extra = class.clone();
        let digit = move || value.get().chars().nth(index).map(String::from);

        view! {
          <div
            aria-hidden="true"
            class=move || {
              cn(
                &[
                  SLOT_CLASSES,
                  if index == first { "rounded-l-md border-l" } else { "" },
                  if index == last { "rounded-r-md" } else { "" },
                  if focused.get() && index == active_index() { SLOT_ACTIVE_CLASSES } else { "" },
                  extra.as_str(),
                ],
              )
            }
          >
            {digit}
            {move || {
              (focused.get() && index == active_index() && digit().is_none())
                .then(|| {
                  view! {
                    <div class="otp-caret pointer-events-none absolute inset-y-2 left-1/2 w-px -translate-x-1/2 bg-foreground"></div>
                  }
                })
            }}
          </div>
        }
      })
      .collect_view()
  };

  let slots = match separator_at {
    Some(at) if at > 0 && at < length => view! {
      <div class="flex items-center">{group(0..at)}</div>
      <div class="flex items-center justify-center">
        <span class="select-none text-muted-foreground">"-"</span>
      </div>
      <div class="flex items-center">{group(at..length)}</div>
    }
    .into_any(),
    _ => view! { <div class="flex items-center">{group(0..length)}</div> }.into_any(),
  };

  view! {
    <style inner_html=CARET_STYLES></style>
    // `w-fit`, or the invisible input — which is sized to this row — reaches
    // across the full width of whatever contains it and catches taps in the
    // empty space beside the boxes.
    <div class=cn(&["relative flex w-fit items-center gap-2", container_class.as_str()])>
      {slots}
      <input
        node_ref=input_ref
        type="text"
        inputmode="numeric"
        pattern="[0-9]*"
        autocomplete="one-time-code"
        autocapitalize="off"
        spellcheck="false"
        class=INPUT_CLASSES
        prop:value=move || value.get()
        on:input=handle_input
        on:focus=move |_| {
          focused.set(true);
          pin_caret();
        }
        on:blur=move |_| focused.set(false)
        on:click=move |_| pin_caret()
        on:select=move |_| pin_caret()
      />
    </div>
  }
}

/// A one-time-password field of `length` digit boxes.
///
/// Pass `value` to read or clear the code from outside — setting that signal
/// (e.g. to `String::new()` after the server rejects a code) re-renders the
/// field. `on_change` fires on every edit, `on_complete` once the last digit
/// lands; neither fires for a change made through `value`.
#[component]
pub fn InputOTP(
  #[prop(into)] length: usize,
  #[prop(optional)] value: RwSignal<String>,
  #[prop(optional)] on_complete: Option<Callback<String>>,
  #[prop(optional)] on_change: Option<Callback<String>>,
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] container_class: String,
) -> impl IntoView {
  otp_field(length, None, value, on_change, on_complete, class, container_class)
}

/// [`InputOTP`] with the boxes split into two groups by a dash, e.g. `123-456`
/// for `length=6, separator_at=3`.
#[component]
pub fn InputOTPWithSeparator(
  #[prop(into)] length: usize,
  #[prop(into)] separator_at: usize,
  #[prop(optional)] value: RwSignal<String>,
  #[prop(optional)] on_complete: Option<Callback<String>>,
  #[prop(optional)] on_change: Option<Callback<String>>,
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] container_class: String,
) -> impl IntoView {
  otp_field(
    length,
    Some(separator_at),
    value,
    on_change,
    on_complete,
    class,
    container_class,
  )
}
