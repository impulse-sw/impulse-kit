#![allow(missing_docs, dead_code)]

// Date Picker component that combines Calendar with Popover
// This is a simplified implementation that can be expanded

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

use super::button::ButtonVariant;
use super::popover::{Popover, PopoverContent, PopoverTrigger};

#[component]
pub fn DatePicker(
  #[prop(optional, into)] value: Option<RwSignal<Option<String>>>,
  #[prop(into, optional)] class: String,
  #[prop(into, optional)] placeholder: String,
) -> impl IntoView {
  let value = value.unwrap_or_else(|| RwSignal::new(None));
  let is_open = RwSignal::new(false);

  let placeholder = if placeholder.is_empty() {
    "Pick a date".to_string()
  } else {
    placeholder
  };

  view! {
    <Popover open=is_open>
      <PopoverTrigger variant=ButtonVariant::Outline class=cn(&["w-[280px] justify-start text-left font-normal", if value.get().is_none() { "text-muted-foreground" } else { "" }, class.as_str()])>
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
          class="mr-2 h-4 w-4"
        >
          <path d="M8 2v4" />
          <path d="M16 2v4" />
          <rect width="18" height="18" x="3" y="4" rx="2" />
          <path d="M3 10h18" />
        </svg>
        {move || value.get().unwrap_or(placeholder.clone())}
      </PopoverTrigger>
      <PopoverContent class="w-auto p-0">
        <div class="p-4 text-center text-sm text-muted-foreground">
          "Calendar component integration coming soon"
        </div>
      </PopoverContent>
    </Popover>
  }
}
