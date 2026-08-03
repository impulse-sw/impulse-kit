#![allow(missing_docs, dead_code)]

//! Usage:
//!
//! chrono = "0.4.42"

use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use impulse_client_kit::utils::cn;
use leptos::context::Provider;
use leptos::prelude::*;
use std::collections::HashSet;

use super::button::{Button, ButtonVariant};

#[component]
pub fn Calendar(
  #[prop(optional)] mode: CalendarMode,
  #[prop(optional)] selected: Option<RwSignal<CalendarSelection>>,
  #[prop(optional)] on_select: Option<Callback<CalendarSelection>>,
  #[prop(optional)] disabled: Option<Signal<Vec<NaiveDate>>>,
  #[prop(optional)] show_week_numbers: Option<bool>,
  #[prop(optional)] caption_layout: CaptionLayout,
  #[prop(optional)] min_date: Option<NaiveDate>,
  #[prop(optional)] max_date: Option<NaiveDate>,
  /// The month on display. Pass a signal to control it from outside — a picker
  /// opening on the month of the date already chosen, say. Defaults to the
  /// current month, uncontrolled.
  #[prop(optional)]
  month: Option<RwSignal<NaiveDate>>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let selected = selected.unwrap_or_else(|| RwSignal::new(CalendarSelection::None));
  let show_week_numbers = show_week_numbers.unwrap_or(false);

  let current_month = month.unwrap_or_else(|| RwSignal::new(Local::now().naive_local().date()));
  let focused_day = RwSignal::new(None::<NaiveDate>);

  let disabled_dates = disabled.unwrap_or_else(|| Signal::derive(Vec::new));

  view! {
    <Provider value=CalendarContext {
      mode,
      selected,
      on_select,
      current_month,
      focused_day,
      disabled_dates,
      min_date,
      max_date,
      show_week_numbers,
    }>
      <div
        data-slot="calendar"
        class=cn(
          &[
            "bg-background border rounded-md group/calendar p-3 w-fit [--cell-size:theme(spacing.8)] [[data-slot=card-content]_&]:bg-transparent [[data-slot=popover-content]_&]:bg-transparent",
            class.as_str(),
          ],
        )
      >
        <div class="flex gap-4 flex-col md:flex-row relative">
          <CalendarMonth caption_layout=caption_layout />
        </div>
      </div>
    </Provider>
  }
}

#[component]
fn CalendarMonth(caption_layout: CaptionLayout) -> impl IntoView {
  let context = use_context::<CalendarContext>().expect("CalendarMonth must be used within Calendar");

  let days_in_month = Memo::new(move |_| {
    let date = context.current_month.get();
    let year = date.year();
    let month = date.month();

    let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let last_day = if month == 12 {
      NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - Duration::days(1)
    } else {
      NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - Duration::days(1)
    };

    (first_day, last_day)
  });

  let weeks = Memo::new(move |_| {
    let (first_day, last_day) = days_in_month.get();
    let mut weeks = Vec::new();
    let mut current_week = Vec::new();

    let start_weekday = first_day.weekday().num_days_from_monday();

    for i in 0..start_weekday {
      let day = first_day - Duration::days((start_weekday - i) as i64);
      current_week.push(Some(day));
    }

    let mut current = first_day;
    while current <= last_day {
      current_week.push(Some(current));

      if current.weekday() == Weekday::Sun {
        weeks.push(current_week.clone());
        current_week.clear();
      }

      current += Duration::days(1);
    }

    if !current_week.is_empty() {
      while current_week.len() < 7 {
        current_week.push(Some(current));
        current += Duration::days(1);
      }
      weeks.push(current_week);
    }

    // Always six rows. Depending on its length and which weekday it starts on, a
    // month covers four, five or six weeks, and letting the grid follow that
    // changes the calendar's height on every step through the months — the nav
    // arrows shift under the cursor between clicks, and anything laid out below
    // the calendar moves with them.
    //
    // Six is the most any month can need (31 days starting on a Sunday), so this
    // pads rather than truncates. The filler continues into the next month, the
    // same real dates the final row already shows beyond the month's end, so
    // nothing about how a row reads changes — there is just always a sixth one.
    while weeks.len() < 6 {
      let mut week = Vec::with_capacity(7);
      for _ in 0..7 {
        week.push(Some(current));
        current += Duration::days(1);
      }
      weeks.push(week);
    }

    weeks
  });

  view! {
    <div class="flex flex-col w-full gap-4">
      <CalendarNav caption_layout=caption_layout />
      <table class="w-full border-collapse">
        <thead>
          <CalendarWeekdays />
        </thead>
        <tbody>
          <For
            each=move || weeks.get()
            key=move |week| {
              format!(
                "months-{}",
                week
                  .first()
                  .and_then(|d| *d)
                  .map(|d| d.format("%Y-%m-%d").to_string())
                  .unwrap_or_default(),
              )
            }
            children=move |week| {
              view! { <CalendarWeek week=week /> }
            }
          />
        </tbody>
      </table>
    </div>
  }
}

#[component]
fn CalendarNav(caption_layout: CaptionLayout) -> impl IntoView {
  let context = use_context::<CalendarContext>().expect("CalendarNav must be used within Calendar");

  let handle_prev = move |_| {
    context.current_month.update(|date| {
      let year = date.year();
      let month = date.month();
      *date = if month == 1 {
        NaiveDate::from_ymd_opt(year - 1, 12, 1).unwrap()
      } else {
        NaiveDate::from_ymd_opt(year, month - 1, 1).unwrap()
      };
    });
  };

  let handle_next = move |_| {
    context.current_month.update(|date| {
      let year = date.year();
      let month = date.month();
      *date = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
      } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
      };
    });
  };

  let can_go_prev = Memo::new(move |_| {
    if let Some(min_date) = context.min_date {
      let current = context.current_month.get();
      let prev_month = if current.month() == 1 {
        NaiveDate::from_ymd_opt(current.year() - 1, 12, 1).unwrap()
      } else {
        NaiveDate::from_ymd_opt(current.year(), current.month() - 1, 1).unwrap()
      };
      prev_month >= min_date
    } else {
      true
    }
  });

  let can_go_next = Memo::new(move |_| {
    if let Some(max_date) = context.max_date {
      let current = context.current_month.get();
      let next_month = if current.month() == 12 {
        NaiveDate::from_ymd_opt(current.year() + 1, 1, 1).unwrap()
      } else {
        NaiveDate::from_ymd_opt(current.year(), current.month() + 1, 1).unwrap()
      };
      next_month <= max_date
    } else {
      true
    }
  });

  let month_label = move || {
    let date = context.current_month.get();
    date.format("%B %Y").to_string()
  };

  view! {
    <div class="flex items-center gap-1 w-full justify-between">
      <Button
        variant=ButtonVariant::Ghost
        class="size-[var(--cell-size)] p-0 select-none"
        attr:disabled=move || !can_go_prev.get()
        on:click=handle_prev
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="size-4"
        >
          <path d="m15 18-6-6 6-6" />
        </svg>
      </Button>

      <div class="flex items-center justify-center h-[var(--cell-size)] w-full px-[var(--cell-size)]">
        {move || {
          if matches!(caption_layout, CaptionLayout::Dropdown) {
            view! { <CalendarDropdowns /> }.into_any()
          } else {
            // A floor wide enough for the longest month label, so the caption
            // stops driving the calendar's width. The root is `w-fit`, the grid
            // below is 7 × `--cell-size`, and the nav row's intrinsic width is
            // the arrows plus this caption — so without a floor "May 2026" and
            // "September 2026" size the whole calendar differently, and every
            // step through the months moves the arrow you are clicking.
            view! {
              <span class="select-none font-medium text-sm min-w-[7rem] text-center">
                {month_label}
              </span>
            }
              .into_any()
          }
        }}
      </div>

      <Button
        variant=ButtonVariant::Ghost
        class="size-[var(--cell-size)] p-0 select-none"
        attr:disabled=move || !can_go_next.get()
        on:click=handle_next
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="size-4"
        >
          <path d="m9 18 6-6-6-6" />
        </svg>
      </Button>
    </div>
  }
}

#[component]
fn CalendarDropdowns() -> impl IntoView {
  let context = use_context::<CalendarContext>().expect("CalendarDropdowns must be used within Calendar");

  let months = vec![
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
  ];

  let current_year = context.current_month.get().year();
  let years: Vec<i32> = ((current_year - 10)..=(current_year + 10)).collect();

  let handle_month_change = move |ev: leptos::ev::Event| {
    let target = event_target::<leptos::web_sys::HtmlSelectElement>(&ev);
    if let Ok(month) = target.value().parse::<u32>() {
      context.current_month.update(|date| {
        *date = NaiveDate::from_ymd_opt(date.year(), month, 1).unwrap();
      });
    }
  };

  let handle_year_change = move |ev: leptos::ev::Event| {
    let target = event_target::<leptos::web_sys::HtmlSelectElement>(&ev);
    if let Ok(year) = target.value().parse::<i32>() {
      context.current_month.update(|date| {
        *date = NaiveDate::from_ymd_opt(year, date.month(), 1).unwrap();
      });
    }
  };

  let _months = months.clone();
  view! {
    <div class="w-full flex items-center text-sm font-medium justify-center h-[var(--cell-size)] gap-1.5">
      <div class="relative has-[:focus]:border-ring border border-input shadow-xs has-[:focus]:ring-ring/50 has-[:focus]:ring-[3px] rounded-md">
        <select
          class="absolute bg-popover inset-0 opacity-0 cursor-pointer"
          on:change=handle_month_change
          prop:value=move || context.current_month.get().month()
        >
          <For
            each=move || _months.clone().into_iter().enumerate()
            key=move |(i, _)| format!("month-{}", *i)
            children=move |(i, name)| {
              view! { <option value=i + 1>{name}</option> }
            }
          />
        </select>
        <div class="rounded-md pl-2 pr-1 flex items-center gap-1 text-sm h-8 pointer-events-none">
          <span>{move || months[context.current_month.get().month() as usize - 1]}</span>
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="text-muted-foreground size-3.5"
          >
            <path d="m6 9 6 6 6-6" />
          </svg>
        </div>
      </div>

      <div class="relative has-[:focus]:border-ring border border-input shadow-xs has-[:focus]:ring-ring/50 has-[:focus]:ring-[3px] rounded-md">
        <select
          class="absolute bg-popover inset-0 opacity-0 cursor-pointer"
          on:change=handle_year_change
          prop:value=move || context.current_month.get().year()
        >
          <For
            each=move || years.clone()
            key=move |y| format!("year-{}", *y)
            children=move |year| {
              view! { <option value=year>{year}</option> }
            }
          />
        </select>
        <div class="rounded-md pl-2 pr-1 flex items-center gap-1 text-sm h-8 pointer-events-none">
          <span>{move || context.current_month.get().year()}</span>
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="text-muted-foreground size-3.5"
          >
            <path d="m6 9 6 6 6-6" />
          </svg>
        </div>
      </div>
    </div>
  }
}

#[component]
fn CalendarWeekdays() -> impl IntoView {
  let context = use_context::<CalendarContext>().expect("CalendarWeekdays must be used within Calendar");

  let weekdays = vec!["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

  view! {
    <tr class="flex">
      <Show when=move || context.show_week_numbers>
        <th class="text-muted-foreground rounded-md flex-1 font-normal text-[0.8rem] select-none w-[var(--cell-size)]">
          ""
        </th>
      </Show>
      <For
        each=move || weekdays.clone()
        key=move |day| format!("weekdays-{}", day)
        children=move |day| {
          view! {
            <th class="text-muted-foreground rounded-md flex-1 font-normal text-[0.8rem] select-none">
              {day}
            </th>
          }
        }
      />
    </tr>
  }
}

#[component]
fn CalendarWeek(week: Vec<Option<NaiveDate>>) -> impl IntoView {
  let context = use_context::<CalendarContext>().expect("CalendarWeek must be used within Calendar");

  let week_number = week.first().and_then(|d| *d).map(|d| d.iso_week().week());

  view! {
    <tr class="flex w-full mt-2">
      <Show when=move || context.show_week_numbers>
        <td class="text-[0.8rem] select-none text-muted-foreground">
          <div class="flex size-[var(--cell-size)] items-center justify-center text-center">
            {week_number.unwrap_or(0)}
          </div>
        </td>
      </Show>
      <For
        each=move || week.clone()
        key=move |day| {
          format!("day-{}", day.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default())
        }
        children=move |day| {
          view! { <CalendarDay day=day /> }
        }
      />
    </tr>
  }
}

#[component]
fn CalendarDay(day: Option<NaiveDate>) -> impl IntoView {
  let context = use_context::<CalendarContext>().expect("CalendarDay must be used within Calendar");

  let day = match day {
    Some(d) => d,
    None => {
      return view! { <td class="relative w-full h-full p-0" /> }.into_any();
    }
  };

  let is_outside = Memo::new(move |_| {
    let current_month = context.current_month.get();
    day.month() != current_month.month() || day.year() != current_month.year()
  });
  let is_today = day == Local::now().naive_local().date();

  let is_disabled = Memo::new(move |_| {
    let disabled = context.disabled_dates.get();
    if disabled.contains(&day) {
      return true;
    }
    if let Some(min) = context.min_date
      && day < min
    {
      return true;
    }
    if let Some(max) = context.max_date
      && day > max
    {
      return true;
    }
    false
  });

  let selection_state = Signal::derive(move || match context.selected.get() {
    CalendarSelection::None => DaySelectionState::None,
    CalendarSelection::Single(selected) => {
      if selected == day {
        DaySelectionState::SelectedSingle
      } else {
        DaySelectionState::None
      }
    }
    CalendarSelection::Multiple(ref selected) => {
      if selected.contains(&day) {
        DaySelectionState::SelectedSingle
      } else {
        DaySelectionState::None
      }
    }
    CalendarSelection::Range { start, end } => {
      if let (Some(s), Some(e)) = (start, end) {
        if day == s && day == e {
          DaySelectionState::SelectedSingle
        } else if day == s {
          DaySelectionState::RangeStart
        } else if day == e {
          DaySelectionState::RangeEnd
        } else if day > s && day < e {
          DaySelectionState::RangeMiddle
        } else {
          DaySelectionState::None
        }
      } else if let Some(s) = start {
        if day == s {
          DaySelectionState::RangeStart
        } else {
          DaySelectionState::None
        }
      } else {
        DaySelectionState::None
      }
    }
  });

  let is_focused = Memo::new(move |_| context.focused_day.get() == Some(day));

  let handle_click = move |_| {
    if is_disabled.get() {
      return;
    }

    let new_selection = match context.mode {
      CalendarMode::Single => CalendarSelection::Single(day),
      CalendarMode::Multiple => {
        let mut selected = match context.selected.get() {
          CalendarSelection::Multiple(s) => s,
          _ => HashSet::new(),
        };
        if selected.contains(&day) {
          selected.remove(&day);
        } else {
          selected.insert(day);
        }
        CalendarSelection::Multiple(selected)
      }
      CalendarMode::Range => match context.selected.get() {
        CalendarSelection::Range { start: None, end: _ } => CalendarSelection::Range {
          start: Some(day),
          end: None,
        },
        CalendarSelection::Range {
          start: Some(s),
          end: None,
        } => {
          if day < s {
            CalendarSelection::Range {
              start: Some(day),
              end: Some(s),
            }
          } else {
            CalendarSelection::Range {
              start: Some(s),
              end: Some(day),
            }
          }
        }
        _ => CalendarSelection::Range {
          start: Some(day),
          end: None,
        },
      },
    };

    context.selected.set(new_selection.clone());
    if let Some(on_select) = context.on_select {
      on_select.run(new_selection);
    }
  };

  let first_col_class = if context.show_week_numbers {
    "[&:nth-child(2)[data-selected=true]_button]:rounded-l-md"
  } else {
    "[&:first-child[data-selected=true]_button]:rounded-l-md"
  };

  let button_class = Signal::derive(move || {
    let state = selection_state.get();
    cn(&[
      "flex aspect-square size-auto w-full min-w-[var(--cell-size)] flex-col gap-1 leading-none font-normal rounded-md",
      if matches!(state, DaySelectionState::SelectedSingle) {
        "bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground"
      } else if matches!(state, DaySelectionState::RangeStart) {
        "bg-primary text-primary-foreground rounded-r-none hover:bg-primary"
      } else if matches!(state, DaySelectionState::RangeEnd) {
        "bg-primary text-primary-foreground rounded-l-none hover:bg-primary"
      } else if matches!(state, DaySelectionState::RangeMiddle) {
        "bg-accent text-accent-foreground rounded-none hover:bg-accent"
      } else {
        ""
      },
      if is_today
        && !matches!(
          state,
          DaySelectionState::SelectedSingle
            | DaySelectionState::RangeStart
            | DaySelectionState::RangeEnd
            | DaySelectionState::RangeMiddle
        )
      {
        "bg-accent text-accent-foreground"
      } else {
        ""
      },
      if is_outside.get() {
        "text-muted-foreground opacity-50"
      } else {
        ""
      },
    ])
  });

  view! {
    <td
      class=cn(
        &[
          "relative w-full h-full p-0 text-center group/day aspect-square select-none [&:last-child[data-selected=true]_button]:rounded-r-md",
          first_col_class,
        ],
      )
      data-selected=move || selection_state.get().is_selected()
      data-focused=move || is_focused.get()
    >
      <Button
        variant=ButtonVariant::Ghost
        class=button_class
        attr:data-selected-single=move || {
          matches!(selection_state.get(), DaySelectionState::SelectedSingle)
        }
        attr:data-range-start=move || matches!(selection_state.get(), DaySelectionState::RangeStart)
        attr:data-range-end=move || matches!(selection_state.get(), DaySelectionState::RangeEnd)
        attr:data-range-middle=move || {
          matches!(selection_state.get(), DaySelectionState::RangeMiddle)
        }
        attr:disabled=move || is_disabled.get()
        on:click=handle_click
      >
        {day.day()}
      </Button>
    </td>
  }
    .into_any()
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum CalendarMode {
  #[default]
  Single,
  Multiple,
  Range,
}

#[derive(Clone, PartialEq)]
pub enum CalendarSelection {
  None,
  Single(NaiveDate),
  Multiple(HashSet<NaiveDate>),
  Range {
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
  },
}

impl std::fmt::Display for CalendarSelection {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::None => write!(f, "none"),
      Self::Single(day) => write!(f, "{day}"),
      Self::Multiple(days) => write!(
        f,
        "{}",
        days.iter().map(|d| format!("{d}")).collect::<Vec<_>>().join(", ")
      ),
      Self::Range { start, end } => match (start, end) {
        (Some(start), Some(end)) => write!(f, "from {start} until {end}"),
        (Some(start), None) => write!(f, "from {start}"),
        (None, Some(end)) => write!(f, "until {end}"),
        (None, None) => write!(f, "none"),
      },
    }
  }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum CaptionLayout {
  #[default]
  Label,
  Dropdown,
}

#[derive(Clone, Copy, PartialEq)]
enum DaySelectionState {
  None,
  SelectedSingle,
  RangeStart,
  RangeEnd,
  RangeMiddle,
}

impl DaySelectionState {
  fn is_selected(&self) -> bool {
    !matches!(self, DaySelectionState::None)
  }
}

#[derive(Clone, Copy)]
struct CalendarContext {
  mode: CalendarMode,
  selected: RwSignal<CalendarSelection>,
  on_select: Option<Callback<CalendarSelection>>,
  current_month: RwSignal<NaiveDate>,
  focused_day: RwSignal<Option<NaiveDate>>,
  disabled_dates: Signal<Vec<NaiveDate>>,
  min_date: Option<NaiveDate>,
  max_date: Option<NaiveDate>,
  show_week_numbers: bool,
}
