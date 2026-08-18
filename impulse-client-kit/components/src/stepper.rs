#![allow(missing_docs, dead_code)]

//! A number field with its own minus/plus buttons.
//!
//! `<input type="number">` already has spinners, and they are the wrong control
//! for a value with a meaningful step: they are a few pixels tall, absent
//! entirely on touch, and always move by one. A priority that goes ±1 and an
//! estimate that goes ±10 minutes want the same widget with different arithmetic
//! — which is what [`NumberStepper`] is.
//!
//! The value is carried as a `String`, not a number, because the field is still
//! typeable: a half-entered `-` or an empty box has to survive a keystroke, and
//! a parsed number cannot represent either. Everything that reads it parses on
//! use, exactly as it would with a plain input.

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

/// Height, matching [`SelectTriggerSize`](crate::select::SelectTriggerSize) and
/// [`ButtonSize`](crate::button::ButtonSize) so a stepper in a row of fields
/// lines up with them.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum StepperSize {
  /// `h-8` — same as [`Input`](crate::input::Input).
  #[default]
  Sm,
  /// `h-9`.
  Middle,
  /// `h-10`.
  Lg,
}

impl StepperSize {
  pub fn as_str(&self) -> &'static str {
    match self {
      StepperSize::Sm => "sm",
      StepperSize::Middle => "middle",
      StepperSize::Lg => "lg",
    }
  }
}

const BUTTON_CLASSES: &str = "border-input bg-muted text-muted-foreground hover:bg-accent hover:text-accent-foreground focus-visible:ring-ring/50 inline-flex shrink-0 items-center justify-center border transition-colors outline-none focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50 group-data-[size=sm]/stepper:size-8 group-data-[size=middle]/stepper:size-9 group-data-[size=lg]/stepper:size-10";

const INPUT_CLASSES: &str = "border-input dark:bg-input/30 w-full min-w-0 border-y bg-transparent px-2 py-1 text-center text-base tabular-nums shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive group-data-[size=sm]/stepper:h-8 group-data-[size=middle]/stepper:h-9 group-data-[size=lg]/stepper:h-10 [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none";

#[component]
pub fn NumberStepper(
  /// The edited value. A `String` so a partially typed number survives.
  value: RwSignal<String>,
  /// How much a button press moves the value. Defaults to `1`.
  #[prop(optional)]
  step: Option<i64>,
  #[prop(optional)] min: Option<i64>,
  #[prop(optional)] max: Option<i64>,
  #[prop(optional)] size: StepperSize,
  #[prop(optional)] disabled: bool,
  /// `aria-label` / tooltip for the minus button.
  #[prop(optional, into)]
  decrement_label: String,
  /// `aria-label` / tooltip for the plus button.
  #[prop(optional, into)]
  increment_label: String,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let step = step.unwrap_or(1);
  let decrement_label = if decrement_label.is_empty() {
    format!("-{step}")
  } else {
    decrement_label
  };
  let increment_label = if increment_label.is_empty() {
    format!("+{step}")
  } else {
    increment_label
  };

  // An unparseable box (empty, or mid-typing) steps from zero rather than
  // refusing to move: pressing "+" on an empty field should give you a number.
  let nudge = move |by: i64| {
    if disabled {
      return;
    }
    let current = value.get_untracked().trim().parse::<i64>().unwrap_or(0);
    let mut next = current.saturating_add(by);
    if let Some(min) = min {
      next = next.max(min);
    }
    if let Some(max) = max {
      next = next.min(max);
    }
    value.set(next.to_string());
  };

  view! {
    <div
      data-slot="number-stepper"
      data-size=size.as_str()
      class=cn(&["group/stepper flex w-full items-stretch", class.as_str()])
    >
      <button
        type="button"
        data-slot="number-stepper-decrement"
        aria-label=decrement_label.clone()
        title=decrement_label
        disabled=disabled
        class=cn(&[BUTTON_CLASSES, "rounded-l-md border-r-0"])
        on:click=move |_| nudge(-step)
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
          aria-hidden="true"
        >
          <path d="M5 12h14" />
        </svg>
      </button>
      <input
        data-slot="number-stepper-input"
        type="number"
        inputmode="numeric"
        step=step.to_string()
        min=min.map(|m| m.to_string())
        max=max.map(|m| m.to_string())
        disabled=disabled
        class=INPUT_CLASSES
        prop:value=value
        on:input:target=move |ev| value.set(ev.target().value())
      />
      <button
        type="button"
        data-slot="number-stepper-increment"
        aria-label=increment_label.clone()
        title=increment_label
        disabled=disabled
        class=cn(&[BUTTON_CLASSES, "rounded-r-md border-l-0"])
        on:click=move |_| nudge(step)
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
          aria-hidden="true"
        >
          <path d="M5 12h14" />
          <path d="M12 5v14" />
        </svg>
      </button>
    </div>
  }
}
