#![allow(missing_docs, dead_code)]

//! A small pulsing dot that communicates a live status (a service being up,
//! down, recovering, or idle). The `Active`, `Down` and `Fixing` states pulse
//! via Tailwind's `animate-ping`; `Idle` stays static.
//!
//! `state` and `label` accept `Signal`s, so the indicator can be driven
//! reactively — e.g. wired straight to a WebSocket connection state.

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

/// Visual status conveyed by the indicator.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusState {
  /// Healthy / connected — green, pulsing.
  Active,
  /// Failed / disconnected — red, pulsing.
  Down,
  /// Recovering / reconnecting — yellow, pulsing.
  Fixing,
  /// No activity — slate, static.
  #[default]
  Idle,
}

impl StatusState {
  fn dot(&self) -> &'static str {
    match self {
      Self::Active => "bg-green-500",
      Self::Down => "bg-red-500",
      Self::Fixing => "bg-yellow-500",
      Self::Idle => "bg-slate-700",
    }
  }

  fn ping(&self) -> &'static str {
    match self {
      Self::Active => "bg-green-300",
      Self::Down => "bg-red-300",
      Self::Fixing => "bg-yellow-300",
      Self::Idle => "bg-slate-400",
    }
  }

  /// Whether the state should pulse. Idle is the only static state.
  fn animated(&self) -> bool {
    matches!(self, Self::Active | Self::Down | Self::Fixing)
  }
}

/// Diameter of the dot (and its pulse).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusIndicatorSize {
  Sm,
  #[default]
  Md,
  Lg,
}

impl StatusIndicatorSize {
  fn class(&self) -> &'static str {
    match self {
      Self::Sm => "h-2 w-2",
      Self::Md => "h-3 w-3",
      Self::Lg => "h-4 w-4",
    }
  }
}

#[component]
pub fn StatusIndicator(
  /// The status to display. Accepts a plain value or a reactive `Signal`.
  #[prop(into, optional)]
  state: Signal<StatusState>,
  /// Optional label rendered next to the dot.
  #[prop(into, optional)]
  label: Option<Signal<String>>,
  /// Dot size.
  #[prop(optional)]
  size: StatusIndicatorSize,
  /// Extra classes for the outer wrapper.
  #[prop(into, optional)]
  class: String,
  /// Extra classes for the label.
  #[prop(into, optional)]
  label_class: String,
) -> impl IntoView {
  let label_class = StoredValue::new(label_class);

  view! {
    <div data-slot="status-indicator" class=cn(&["flex items-center gap-2", class.as_str()])>
      <div class="relative flex items-center">
        {move || {
          let state = state.get();
          state
            .animated()
            .then(|| {
              view! {
                <span class=cn(
                  &[
                    "absolute inline-flex rounded-full opacity-75 animate-ping",
                    size.class(),
                    state.ping(),
                  ],
                ) />
              }
            })
        }}
        <span class=move || {
          cn(&["relative inline-flex rounded-full", size.class(), state.get().dot()])
        }></span>
      </div>
      {label
        .map(|label| {
          view! {
            <p class=cn(
              &["text-sm text-slate-700 dark:text-slate-300", label_class.get_value().as_str()],
            )>{move || label.get()}</p>
          }
        })}
    </div>
  }
}

/// The connection dot for an offline-first app: three states, not two.
///
/// A plain [`StatusIndicator`] answers "is there a socket". For an app that
/// accepts a write locally and hands it over later, that leaves the more useful
/// question unanswered — does the server have what this device holds. So:
///
/// * no connection — red, the only state that asks anything of the reader;
/// * connected with work still owed — yellow, which says the app is busy handing
///   it over. That is a normal thing to be doing, not a problem to report;
/// * connected and owing nothing — green.
///
/// Green therefore means what people assume it means: everything here is also
/// there. That is what makes a separate "you are offline" banner unnecessary —
/// three states in one dot say what the banner would have, without a row that
/// appears and shoves the layout down.
///
/// `pending` is the app's queue depth. In a browser build there is no queue, so
/// pass `0` and the dot behaves exactly like the two-state one.
#[component]
pub fn SyncIndicator(
  /// Whether a connection to the server is currently held.
  #[prop(into)]
  connected: Signal<bool>,
  /// How much local work is still owed to the server.
  #[prop(into)]
  pending: Signal<usize>,
  /// Dot size.
  #[prop(optional)]
  size: StatusIndicatorSize,
  /// Tooltip wording, in the app's language. Defaults to English.
  #[prop(optional)]
  labels: SyncLabels,
  /// Extra classes for the outer wrapper.
  #[prop(into, optional)]
  class: String,
) -> impl IntoView {
  let state = Signal::derive(move || {
    if !connected.get() {
      StatusState::Down
    } else if pending.get() > 0 {
      StatusState::Fixing
    } else {
      StatusState::Active
    }
  });
  let labels = StoredValue::new(labels);
  let title = move || {
    labels.with_value(|l| {
      if !connected.get() {
        l.disconnected.clone()
      } else {
        match pending.get() {
          0 => l.synced.clone(),
          n => format!("{} {n}", l.syncing),
        }
      }
    })
  };

  view! {
    <span class=cn(&["inline-flex", class.as_str()]) title=title>
      <StatusIndicator state=state size=size />
    </span>
  }
}

/// Tooltip wording for [`SyncIndicator`].
///
/// `syncing` is a prefix: the count of outstanding work is appended to it, so
/// write it to read naturally before a number.
#[derive(Clone)]
pub struct SyncLabels {
  /// Shown while there is no connection.
  pub disconnected: String,
  /// Shown when the server has everything this device holds.
  pub synced: String,
  /// Prefix shown while work is still being handed over.
  pub syncing: String,
}

impl Default for SyncLabels {
  fn default() -> Self {
    Self {
      disconnected: "No connection".into(),
      synced: "Everything is synced".into(),
      syncing: "Syncing… remaining:".into(),
    }
  }
}
