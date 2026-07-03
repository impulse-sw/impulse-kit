//! Recovery hooks for pages that were frozen and later resumed.
//!
//! When a browser freezes a page into the back/forward cache (bfcache) or
//! discards a background tab, its timers stop advancing and its sockets can die
//! without ever delivering a `close` event. On resume, long-lived connections
//! and wall-clock-relative schedules are silently broken — the "CSR stops
//! working after a long idle" class of bug.
//!
//! [`on_page_restore`] centralises the small pile of window/document listeners
//! that detect such a resume, so every consumer (WebSocket, WebTransport, the
//! LBRP Security Gateway revalidation loop, …) can recover the same way instead
//! of re-deriving the event plumbing.

use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Event, PageTransitionEvent};

/// Guard owning the window/document page-lifecycle listeners installed by
/// [`on_page_restore`].
///
/// Dropping it detaches every listener, so keep it alive for as long as
/// recovery is wanted — store it next to the connection it protects, or move it
/// into the task that owns the schedule.
#[must_use = "dropping the guard immediately detaches the page-lifecycle listeners"]
pub struct PageLifecycleListeners {
  _on_pageshow: Closure<dyn FnMut(PageTransitionEvent)>,
  _on_online: Closure<dyn FnMut(Event)>,
  _on_visibility: Closure<dyn FnMut(Event)>,
}

impl Drop for PageLifecycleListeners {
  fn drop(&mut self) {
    let Some(win) = web_sys::window() else { return };
    let _ = win.remove_event_listener_with_callback("pageshow", self._on_pageshow.as_ref().unchecked_ref());
    let _ = win.remove_event_listener_with_callback("online", self._on_online.as_ref().unchecked_ref());
    if let Some(doc) = win.document() {
      let _ = doc.remove_event_listener_with_callback("visibilitychange", self._on_visibility.as_ref().unchecked_ref());
    }
  }
}

/// Attach window/document listeners that fire `on_wake` when the page may have
/// resumed from a frozen state — a bfcache restore, a discarded tab coming
/// back, the network returning, or the tab becoming visible again.
///
/// `on_wake` receives a `force` flag describing how stale the page might be:
///
/// * `true` — a bfcache restore (`pageshow` with `persisted == true`). The tab
///   may have been frozen well past any token or connection lifetime, so the
///   caller should recover unconditionally.
/// * `false` — the tab came back `online` or became visible again. The caller
///   should recover only if the thing it watches actually looks dead, leaving a
///   still-healthy connection or a not-yet-due schedule untouched.
///
/// The listeners persist until the returned [`PageLifecycleListeners`] guard is
/// dropped. Returns `None` when there is no `window` (a non-browser / SSR
/// context), in which case nothing is installed.
pub fn on_page_restore<F>(on_wake: F) -> Option<PageLifecycleListeners>
where
  F: Fn(bool) + 'static,
{
  let win = web_sys::window()?;
  let on_wake = Rc::new(on_wake);

  // A bfcache restore (`persisted`) recovers unconditionally; a normal initial
  // `pageshow` (`persisted == false`) is a no-op.
  let cb = on_wake.clone();
  let on_pageshow = Closure::<dyn FnMut(PageTransitionEvent)>::new(move |e: PageTransitionEvent| {
    if e.persisted() {
      cb(true);
    }
  });
  let _ = win.add_event_listener_with_callback("pageshow", on_pageshow.as_ref().unchecked_ref());

  let cb = on_wake.clone();
  let on_online = Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
    cb(false);
  });
  let _ = win.add_event_listener_with_callback("online", on_online.as_ref().unchecked_ref());

  // Only a transition *to* visible is a resume; ignore the page going hidden.
  let cb = on_wake;
  let on_visibility = Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
    let visible = web_sys::window().and_then(|w| w.document()).is_some_and(|d| !d.hidden());
    if visible {
      cb(false);
    }
  });
  if let Some(doc) = win.document() {
    let _ = doc.add_event_listener_with_callback("visibilitychange", on_visibility.as_ref().unchecked_ref());
  }

  Some(PageLifecycleListeners {
    _on_pageshow: on_pageshow,
    _on_online: on_online,
    _on_visibility: on_visibility,
  })
}
