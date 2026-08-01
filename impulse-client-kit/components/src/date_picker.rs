#![allow(missing_docs, dead_code)]

//! Date and time pickers built entirely out of Client Kit components.
//!
//! Nothing here delegates to `<input type="date">` / `<input type="time">`. The
//! native widgets are a different control on every platform — and on Linux
//! webviews they are a poor one that, worse, *pre-fills itself*: a field the
//! user never touched still hands back a date, so "no deadline" is impossible to
//! express. These pickers hold `Option<NaiveDateTime>`, start empty unless a
//! `default_value` is given, and can always be cleared back to `None`.
//!
//! Usage:
//!
//! chrono = "0.4.42"

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use super::button::{Button, ButtonSize, ButtonVariant};
use super::calendar::{Calendar, CalendarMode, CalendarSelection};
use super::dialog::{Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle};

/// What the picker asks for.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DateTimeMode {
  /// A calendar only; the time part of the value is left at midnight.
  Date,
  /// Hours and minutes only; the date part of the value is kept as it was (or
  /// today's, when there was none).
  Time,
  /// Both, in one dialog.
  #[default]
  DateTime,
}

impl DateTimeMode {
  fn has_date(self) -> bool {
    matches!(self, Self::Date | Self::DateTime)
  }

  fn has_time(self) -> bool {
    matches!(self, Self::Time | Self::DateTime)
  }

  /// The `strftime` pattern the trigger falls back to.
  fn default_format(self) -> &'static str {
    match self {
      Self::Date => "%Y-%m-%d",
      Self::Time => "%H:%M",
      Self::DateTime => "%Y-%m-%d %H:%M",
    }
  }

  fn default_title(self) -> &'static str {
    match self {
      Self::Date => "Select a date",
      Self::Time => "Select a time",
      Self::DateTime => "Select a date and time",
    }
  }

  fn default_placeholder(self) -> &'static str {
    match self {
      Self::Date => "Pick a date",
      Self::Time => "Pick a time",
      Self::DateTime => "Pick a date and time",
    }
  }
}

/// A date/time field with its own dialog — no native picker involved.
///
/// The value is `Option<NaiveDateTime>` and starts as `None` unless
/// `default_value` says otherwise, so "nothing chosen" is a state the field can
/// both start in and return to (via the dialog's *Clear*).
///
/// * `value` / `default_value` — the signal to bind, and what it starts as.
/// * `mode` — [`DateTimeMode::DateTime`] by default; `Date` or `Time` drop the
///   half of the dialog they don't need.
/// * `on_change` — fired on every commit, including clearing (`None`).
/// * `min_date` / `max_date` — passed to the calendar.
/// * `minute_step` — how far one press of the minute stepper moves; `1` by default.
/// * `clearable` — offer *Clear* in the dialog; `true` by default.
/// * `format` — `strftime` pattern for the trigger label; per-mode default otherwise.
/// * `title`, `clear_label`, `cancel_label`, `confirm_label` — dialog wording.
///
/// ```rust,ignore
/// use chrono::NaiveDateTime;
/// use impulse_client_kit_components::date_picker::{DateTimeMode, DateTimePicker};
/// use leptos::prelude::*;
///
/// let deadline = RwSignal::new(None::<NaiveDateTime>);
///
/// view! {
///   <DateTimePicker value=deadline placeholder="No deadline" />
///   <DateTimePicker value=deadline mode=DateTimeMode::Date />
/// }
/// ```
#[component]
pub fn DateTimePicker(
  #[prop(optional)] value: Option<RwSignal<Option<NaiveDateTime>>>,
  #[prop(optional)] default_value: Option<NaiveDateTime>,
  #[prop(optional)] mode: DateTimeMode,
  #[prop(optional)] on_change: Option<Callback<Option<NaiveDateTime>>>,
  #[prop(optional)] min_date: Option<NaiveDate>,
  #[prop(optional)] max_date: Option<NaiveDate>,
  #[prop(optional)] minute_step: Option<u32>,
  #[prop(optional)] clearable: Option<bool>,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] placeholder: String,
  #[prop(optional, into)] format: String,
  #[prop(optional, into)] title: String,
  #[prop(optional, into)] clear_label: String,
  #[prop(optional, into)] cancel_label: String,
  #[prop(optional, into)] confirm_label: String,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  picker(PickerConfig {
    value,
    default_value,
    mode,
    on_change,
    min_date,
    max_date,
    minute_step,
    clearable,
    disabled,
    placeholder,
    format,
    title,
    clear_label,
    cancel_label,
    confirm_label,
    class,
  })
}

/// Everything a picker needs, so the three public components differ only in
/// which fields they let a caller set.
#[derive(Default)]
struct PickerConfig {
  value: Option<RwSignal<Option<NaiveDateTime>>>,
  default_value: Option<NaiveDateTime>,
  mode: DateTimeMode,
  on_change: Option<Callback<Option<NaiveDateTime>>>,
  min_date: Option<NaiveDate>,
  max_date: Option<NaiveDate>,
  minute_step: Option<u32>,
  clearable: Option<bool>,
  disabled: bool,
  placeholder: String,
  format: String,
  title: String,
  clear_label: String,
  cancel_label: String,
  confirm_label: String,
  class: String,
}

fn picker(config: PickerConfig) -> AnyView {
  let PickerConfig {
    value,
    default_value,
    mode,
    on_change,
    min_date,
    max_date,
    minute_step,
    clearable,
    disabled,
    placeholder,
    format,
    title,
    clear_label,
    cancel_label,
    confirm_label,
    class,
  } = config;

  let value = value.unwrap_or_else(|| RwSignal::new(default_value));
  let clearable = clearable.unwrap_or(true);
  let minute_step = minute_step.unwrap_or(1).clamp(1, 30);
  // The calendar takes plain bounds; the full `NaiveDate` range is "unbounded".
  let min_date = min_date.unwrap_or(NaiveDate::MIN);
  let max_date = max_date.unwrap_or(NaiveDate::MAX);

  let text =
    |given: String, fallback: &str| StoredValue::new(if given.is_empty() { fallback.to_string() } else { given });
  let placeholder = text(placeholder, mode.default_placeholder());
  let format = text(format, mode.default_format());
  let title = text(title, mode.default_title());
  let clear_label = text(clear_label, "Clear");
  let cancel_label = text(cancel_label, "Cancel");
  let confirm_label = text(confirm_label, "OK");

  let is_open = RwSignal::new(false);
  // Draft state: the dialog edits its own copy, so *Cancel* really cancels.
  let selection = RwSignal::new(CalendarSelection::None);
  let month = RwSignal::new(today());
  let hour = RwSignal::new(0u32);
  let minute = RwSignal::new(0u32);

  let commit = move |next: Option<NaiveDateTime>| {
    value.set(next);
    if let Some(on_change) = on_change {
      on_change.run(next);
    }
  };

  let open_dialog = move |_| {
    if disabled {
      return;
    }
    let current = value.get_untracked();
    let date = current.map(|dt| dt.date());
    selection.set(match date {
      Some(date) => CalendarSelection::Single(date),
      None => CalendarSelection::None,
    });
    month.set(date.unwrap_or_else(today));
    hour.set(current.map(|dt| dt.hour()).unwrap_or(0));
    minute.set(current.map(|dt| dt.minute()).unwrap_or(0));
    is_open.set(true);
  };

  let confirm = move |_| {
    let time = NaiveTime::from_hms_opt(hour.get_untracked(), minute.get_untracked(), 0).unwrap_or(NaiveTime::MIN);
    let picked = match selection.get_untracked() {
      CalendarSelection::Single(date) => Some(date),
      _ => None,
    };
    let next = if mode.has_date() {
      // No day picked means no value — the whole point of a nullable field.
      picked.map(|date| date.and_time(if mode.has_time() { time } else { NaiveTime::MIN }))
    } else {
      // Time-only keeps the date it already had, or lands on today.
      let date = value.get_untracked().map(|dt| dt.date()).unwrap_or_else(today);
      Some(date.and_time(time))
    };
    commit(next);
    is_open.set(false);
  };

  let clear = move |_| {
    commit(None);
    is_open.set(false);
  };

  let cancel = move |_| is_open.set(false);

  let label = move || match value.get() {
    Some(dt) => format.with_value(|format| dt.format(format).to_string()),
    None => placeholder.get_value(),
  };
  let is_empty = move || value.get().is_none();

  view! {
    <div data-slot="date-picker" class=cn(&["w-full", class.as_str()])>
      <div class="flex w-full items-center gap-1">
        <button
          type="button"
          data-slot="date-picker-trigger"
          data-empty=move || is_empty().then_some("true")
          aria-haspopup="dialog"
          disabled=disabled
          class="border-input dark:bg-input/30 focus-visible:border-ring focus-visible:ring-ring/50 data-[empty]:text-muted-foreground flex h-8 w-full min-w-0 items-center justify-start gap-2 rounded-md border bg-transparent px-3 text-left text-sm font-normal shadow-xs transition-[color,box-shadow] outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50"
          on:click=open_dialog
        >
          {if mode.has_date() {
            view! { <CalendarIcon /> }.into_any()
          } else {
            view! { <ClockIcon /> }.into_any()
          }}
          <span class="truncate">{label}</span>
        </button>

        // Clearing is the one action worth a shortcut: a field that can hold
        // "nothing" is useless if getting back there takes a round trip.
        <Show when=move || clearable && !disabled && !is_empty()>
          <Button
            variant=ButtonVariant::Ghost
            size=ButtonSize::IconSm
            attr:aria-label=clear_label.get_value()
            attr:title=clear_label.get_value()
            on:click=move |_| commit(None)
          >
            <XIcon />
          </Button>
        </Show>
      </div>

      <Dialog open=is_open>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{move || title.get_value()}</DialogTitle>
          </DialogHeader>

          <div class="flex flex-col items-center gap-4">
            {if mode.has_date() {
              view! {
                <Calendar
                  mode=CalendarMode::Single
                  selected=selection
                  month=month
                  min_date=min_date
                  max_date=max_date
                />
              }
                .into_any()
            } else {
              ().into_any()
            }}
            {if mode.has_time() {
              view! { <TimeFields hour=hour minute=minute minute_step=minute_step /> }.into_any()
            } else {
              ().into_any()
            }}
          </div>

          <DialogFooter>
            <div class="flex w-full flex-col-reverse gap-2 sm:flex-row sm:items-center">
              <Show when=move || clearable>
                <Button variant=ButtonVariant::Ghost on:click=clear>
                  {move || clear_label.get_value()}
                </Button>
              </Show>
              <div class="flex flex-col-reverse gap-2 sm:ml-auto sm:flex-row">
                <Button variant=ButtonVariant::Outline on:click=cancel>
                  {move || cancel_label.get_value()}
                </Button>
                <Button on:click=confirm>{move || confirm_label.get_value()}</Button>
              </div>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  }
  .into_any()
}

/// [`DateTimePicker`] fixed to [`DateTimeMode::Date`] — the calendar alone, with
/// the value's time left at midnight.
#[component]
pub fn DatePicker(
  #[prop(optional)] value: Option<RwSignal<Option<NaiveDateTime>>>,
  #[prop(optional)] default_value: Option<NaiveDateTime>,
  #[prop(optional)] on_change: Option<Callback<Option<NaiveDateTime>>>,
  #[prop(optional)] min_date: Option<NaiveDate>,
  #[prop(optional)] max_date: Option<NaiveDate>,
  #[prop(optional)] clearable: Option<bool>,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] placeholder: String,
  #[prop(optional, into)] format: String,
  #[prop(optional, into)] title: String,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  picker(PickerConfig {
    mode: DateTimeMode::Date,
    value,
    default_value,
    on_change,
    min_date,
    max_date,
    clearable,
    disabled,
    placeholder,
    format,
    title,
    class,
    ..Default::default()
  })
}

/// [`DateTimePicker`] fixed to [`DateTimeMode::Time`] — hour and minute steppers
/// only; the date part of the value is preserved.
#[component]
pub fn TimePicker(
  #[prop(optional)] value: Option<RwSignal<Option<NaiveDateTime>>>,
  #[prop(optional)] default_value: Option<NaiveDateTime>,
  #[prop(optional)] on_change: Option<Callback<Option<NaiveDateTime>>>,
  #[prop(optional)] minute_step: Option<u32>,
  #[prop(optional)] clearable: Option<bool>,
  #[prop(optional)] disabled: bool,
  #[prop(optional, into)] placeholder: String,
  #[prop(optional, into)] format: String,
  #[prop(optional, into)] title: String,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  picker(PickerConfig {
    mode: DateTimeMode::Time,
    value,
    default_value,
    on_change,
    minute_step,
    clearable,
    disabled,
    placeholder,
    format,
    title,
    class,
    ..Default::default()
  })
}

/// The hour/minute steppers: two columns of ▲ number ▼, wrapping at their bounds.
#[component]
fn TimeFields(hour: RwSignal<u32>, minute: RwSignal<u32>, minute_step: u32) -> impl IntoView {
  view! {
    <div data-slot="time-fields" class="flex items-center justify-center gap-2">
      <TimeField label="Hours" value=hour bound=24 step=1 />
      <span class="pb-1 text-2xl font-semibold text-muted-foreground">":"</span>
      <TimeField label="Minutes" value=minute bound=60 step=minute_step />
    </div>
  }
}

/// One stepper. `bound` is exclusive — the value wraps around it in both
/// directions, so holding ▲ past 23:00 lands back on midnight rather than
/// stopping dead.
#[component]
fn TimeField(label: &'static str, value: RwSignal<u32>, bound: u32, step: u32) -> impl IntoView {
  let shift = move |by: i64| {
    value.update(|current| {
      let bound = bound as i64;
      *current = (((*current as i64 + by) % bound + bound) % bound) as u32;
    })
  };

  let keyboard = move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
    "ArrowUp" => {
      ev.prevent_default();
      shift(step as i64);
    }
    "ArrowDown" => {
      ev.prevent_default();
      shift(-(step as i64));
    }
    _ => {}
  };

  view! {
    <div class="flex flex-col items-center gap-1">
      <Button
        variant=ButtonVariant::Ghost
        size=ButtonSize::IconSm
        attr:aria-label=format!("{label}: increase")
        on:click=move |_| shift(step as i64)
      >
        <ChevronIcon up=true />
      </Button>
      <div
        role="spinbutton"
        aria-label=label
        aria-valuemin="0"
        aria-valuemax=(bound - 1).to_string()
        aria-valuenow=move || value.get().to_string()
        tabindex="0"
        class="w-14 rounded-md border border-input py-1 text-center text-2xl font-semibold tabular-nums outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50"
        on:keydown=keyboard
      >
        {move || format!("{:02}", value.get())}
      </div>
      <Button
        variant=ButtonVariant::Ghost
        size=ButtonSize::IconSm
        attr:aria-label=format!("{label}: decrease")
        on:click=move |_| shift(-(step as i64))
      >
        <ChevronIcon up=false />
      </Button>
    </div>
  }
}

/// Today, in the machine's local time zone.
fn today() -> NaiveDate {
  Local::now().naive_local().date()
}

#[component]
fn CalendarIcon() -> impl IntoView {
  view! {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class="size-4 shrink-0 opacity-70"
      aria-hidden="true"
    >
      <path d="M8 2v4" />
      <path d="M16 2v4" />
      <rect width="18" height="18" x="3" y="4" rx="2" />
      <path d="M3 10h18" />
    </svg>
  }
}

#[component]
fn ClockIcon() -> impl IntoView {
  view! {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class="size-4 shrink-0 opacity-70"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="10" />
      <path d="M12 6v6l4 2" />
    </svg>
  }
}

#[component]
fn XIcon() -> impl IntoView {
  view! {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class="size-4"
      aria-hidden="true"
    >
      <path d="M18 6 6 18" />
      <path d="m6 6 12 12" />
    </svg>
  }
}

#[component]
fn ChevronIcon(up: bool) -> impl IntoView {
  view! {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class="size-4"
      aria-hidden="true"
    >
      <path d=if up { "m18 15-6-6-6 6" } else { "m6 9 6 6 6-6" } />
    </svg>
  }
}
