//! Components for `impulse-client-kit`.
//!
//! ## Hiding content also hides it from the keyboard
//!
//! Whatever hides something owns hiding it completely. Nothing inside a
//! collapsed section or a closed overlay can know it is hidden — only the
//! wrapper knows — so the wrapper takes the whole subtree out of the tab order
//! and the accessibility tree. Making each child opt out instead would push the
//! problem onto every application built on the kit, which is how a spoiler ends
//! up with Tab walking through rows nobody can see.
//!
//! Two mechanisms do that, and both cascade to descendants:
//!
//! * **`visibility: hidden`** (`data-[state=closed]:invisible`) — for an overlay
//!   that is not on screen at all while closed but stays mounted: a select's
//!   options have to remain in the DOM for the chosen one's label to be
//!   resolvable, a dialog for its animation to have something to run on.
//! * **`inert`** — for content that is *still drawn* while it goes away. A
//!   collapsible and an accordion animate their height to zero, and blanking
//!   the content at the first frame would leave an empty box shrinking; `inert`
//!   changes nothing on screen and everything about interaction.
//!
//! `display: none` is neither: it cancels the animation and, in a collapsible,
//! the height measurement the animation is built on. `pointer-events: none`
//! only stops the mouse — the keyboard goes straight past it.

#![deny(warnings)]
#![recursion_limit = "256"]
#![allow(clippy::extra_unused_lifetimes)]

pub mod accordion;
pub mod alert;
pub mod alert_dialog;
pub mod aspect_ratio;
pub mod avatar;
pub mod back;
pub mod badge;
pub mod breadcrumb;
pub mod button;
pub mod button_group;
pub mod calendar;
pub mod card;
pub mod carousel;
pub mod checkbox;
pub mod collapsible;
pub mod combobox;
pub mod command;
pub mod context_menu;
pub mod cookie_consent;
pub mod data_table;
pub mod date_picker;
pub mod dialog;
pub mod dismiss;
pub mod dnd;
pub mod drawer;
pub mod dropdown_menu;
pub mod empty;
pub mod field;
pub mod form;
pub mod hover_card;
pub mod icon;
pub mod input;
pub mod input_group;
pub mod input_otp;
pub mod item;
pub mod kbd;
pub mod label;
pub mod layout;
pub mod menubar;
pub mod native_select;
pub mod navigation_menu;
pub mod pagination;
pub mod popover;
pub mod progress;
pub mod radio_group;
pub mod raw;
pub mod resizable;
pub mod scroll_area;
pub mod select;
pub mod separator;
pub mod shake;
pub mod sheet;
pub mod side_nav;
pub mod sidebar;
pub mod skeleton;
pub mod slider;
pub mod sonner;
pub mod spinner;
pub mod status_indicator;
pub mod stepper;
pub mod switch;
pub mod table;
pub mod tabs;
pub mod textarea;
pub mod theme;
pub mod timeline;
pub mod toast;
pub mod toggle;
pub mod toggle_group;
pub mod tooltip;
mod viewport;
