//! Offline-first client store for Leptos apps that must keep working without a
//! server and reconcile when it returns.
//!
//! Four small pieces, deliberately unopinionated so both the WASM (browser) and
//! Tauri builds of an app can share them:
//!
//! * [`Connectivity`] — a reactive online/offline flag.
//! * [`Cache`] — a reactive, optionally-persisted key→value cache of server data
//!   so reads render instantly (and offline) from last-known state.
//! * [`MutationQueue`] — an ordered, persisted journal of writes made while
//!   offline, each with a stable id for idempotent replay.
//! * [`drain_queue`] / [`on_reconnect`] — the sync driver: when the connection
//!   returns, replay the queue against the server, then reconcile the cache.
//!
//! The store holds no transport of its own. An app wires it to a server via
//! `impulse-client-kit`'s unified client (browser/native or Tauri IPC), so the
//! same store code runs in every build.

#![deny(warnings, clippy::todo, clippy::unimplemented)]

pub mod persist;

use std::collections::BTreeMap;
use std::collections::HashMap;

use leptos::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Reactive online/offline flag. Cheap to `Copy` and share via `provide_context`.
///
/// On a plain web build call [`Connectivity::track_browser`] once to mirror the
/// browser's `online`/`offline` events; under Tauri the engine flips it via
/// [`Connectivity::set`] as its link to the server goes up and down.
#[derive(Clone, Copy)]
pub struct Connectivity {
  online: RwSignal<bool>,
}

impl Default for Connectivity {
  fn default() -> Self {
    Self::new(true)
  }
}

impl Connectivity {
  /// Creates a flag with an initial state.
  pub fn new(online: bool) -> Self {
    Self {
      online: RwSignal::new(online),
    }
  }

  /// The reactive state (tracks in a reactive context).
  pub fn is_online(&self) -> bool {
    self.online.get()
  }

  /// The current state without tracking.
  pub fn is_online_untracked(&self) -> bool {
    self.online.get_untracked()
  }

  /// The underlying signal, e.g. to drive a "you're offline" banner.
  pub fn signal(&self) -> RwSignal<bool> {
    self.online
  }

  /// Sets the state (used by the Tauri engine bridge).
  pub fn set(&self, online: bool) {
    self.online.set(online);
  }

  /// Subscribes to the browser's `online`/`offline` events (wasm, no-op elsewhere).
  #[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
  pub fn track_browser(&self) {
    use wasm_bindgen::JsCast;
    // Seed from navigator.onLine, then keep in sync via window events.
    if let Some(win) = web_sys::window() {
      self.online.set(win.navigator().on_line());
      let online = self.online;
      let on = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || online.set(true));
      let off = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || online.set(false));
      let _ = win.add_event_listener_with_callback("online", on.as_ref().unchecked_ref());
      let _ = win.add_event_listener_with_callback("offline", off.as_ref().unchecked_ref());
      // Leak the closures: they live for the whole page, like the listeners.
      on.forget();
      off.forget();
    }
  }

  /// No-op on native; the engine drives state via [`set`](Self::set).
  #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
  pub fn track_browser(&self) {}
}

/// A reactive, optionally-persisted key→value cache of server-owned data.
///
/// Reads come straight from signals so the UI renders instantly and offline from
/// the last-known snapshot; writes update the signal and (with `persist`) the
/// backing store. `V` must be serialisable so it can be persisted and hydrated.
#[derive(Clone, Copy)]
pub struct Cache<V>
where
  V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
  namespace: &'static str,
  entries: RwSignal<HashMap<String, V>>,
}

impl<V> Cache<V>
where
  V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
  /// Creates a cache under a storage `namespace`, hydrating any persisted state.
  pub fn new(namespace: &'static str) -> Self {
    let entries: HashMap<String, V> = persist::load(namespace).unwrap_or_default();
    Self {
      namespace,
      entries: RwSignal::new(entries),
    }
  }

  /// The whole map (tracks).
  pub fn snapshot(&self) -> HashMap<String, V> {
    self.entries.get()
  }

  /// The reactive backing signal, for fine-grained subscriptions.
  pub fn signal(&self) -> RwSignal<HashMap<String, V>> {
    self.entries
  }

  /// Looks up one entry (tracks).
  pub fn get(&self, key: &str) -> Option<V> {
    self.entries.with(|m| m.get(key).cloned())
  }

  /// Inserts or replaces one entry and persists the map.
  pub fn set(&self, key: impl Into<String>, value: V) {
    self.entries.update(|m| {
      m.insert(key.into(), value);
    });
    self.flush();
  }

  /// Replaces the entire cache (e.g. from a fresh server snapshot) and persists.
  pub fn replace_all(&self, entries: HashMap<String, V>) {
    self.entries.set(entries);
    self.flush();
  }

  /// Removes one entry and persists.
  pub fn remove(&self, key: &str) {
    self.entries.update(|m| {
      m.remove(key);
    });
    self.flush();
  }

  /// Clears the cache and its persisted copy.
  pub fn clear(&self) {
    self.entries.update(|m| m.clear());
    persist::remove(self.namespace);
  }

  fn flush(&self) {
    self.entries.with_untracked(|m| persist::save(self.namespace, m));
  }
}

/// An ordered, persisted journal of writes made while offline.
///
/// Each entry gets a monotonic id so replay is idempotent: the sync driver walks
/// entries oldest-first, replays each against the server, and [`ack`](Self::ack)s
/// it on success. A crash mid-flush leaves un-acked entries to be retried.
#[derive(Clone, Copy)]
pub struct MutationQueue<M>
where
  M: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
  namespace: &'static str,
  next_id: RwSignal<u64>,
  entries: RwSignal<BTreeMap<u64, M>>,
}

impl<M> MutationQueue<M>
where
  M: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
  /// Creates a queue under a storage `namespace`, hydrating any persisted entries.
  pub fn new(namespace: &'static str) -> Self {
    let entries: BTreeMap<u64, M> = persist::load(namespace).unwrap_or_default();
    let next_id = entries.keys().next_back().copied().map(|k| k + 1).unwrap_or(0);
    Self {
      namespace,
      next_id: RwSignal::new(next_id),
      entries: RwSignal::new(entries),
    }
  }

  /// Appends a mutation, returning its id.
  pub fn enqueue(&self, mutation: M) -> u64 {
    let id = self.next_id.get_untracked();
    self.next_id.set(id + 1);
    self.entries.update(|m| {
      m.insert(id, mutation);
    });
    self.flush();
    id
  }

  /// Pending mutations, oldest first.
  pub fn pending(&self) -> Vec<(u64, M)> {
    self
      .entries
      .with(|m| m.iter().map(|(id, v)| (*id, v.clone())).collect())
  }

  /// Number of pending mutations (tracks) — handy for a "N unsynced" badge.
  pub fn len(&self) -> usize {
    self.entries.with(|m| m.len())
  }

  /// Whether the queue is empty (tracks).
  pub fn is_empty(&self) -> bool {
    self.entries.with(|m| m.is_empty())
  }

  /// The reactive backing signal.
  pub fn signal(&self) -> RwSignal<BTreeMap<u64, M>> {
    self.entries
  }

  /// Marks a mutation replayed and drops it.
  pub fn ack(&self, id: u64) {
    self.entries.update(|m| {
      m.remove(&id);
    });
    self.flush();
  }

  /// Empties the queue and its persisted copy.
  pub fn clear(&self) {
    self.entries.update(|m| m.clear());
    persist::remove(self.namespace);
  }

  fn flush(&self) {
    self.entries.with_untracked(|m| persist::save(self.namespace, m));
  }
}

/// Replays every pending mutation against `replay`, oldest first, ack-ing each on
/// success. Stops at the first failure and returns its message, leaving that
/// mutation and the rest queued for the next attempt.
///
/// Transport-agnostic and target-agnostic: pass a closure that sends the mutation
/// through the app's client. Spawn it however the target expects (`spawn_local`
/// on wasm, a Tokio task under Tauri).
pub async fn drain_queue<M, F, Fut>(queue: &MutationQueue<M>, mut replay: F) -> Result<(), String>
where
  M: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
  F: FnMut(M) -> Fut,
  Fut: std::future::Future<Output = Result<(), String>>,
{
  for (id, mutation) in queue.pending() {
    replay(mutation).await?;
    queue.ack(id);
  }
  Ok(())
}

/// Runs `on_online` whenever connectivity transitions offline→online (not on the
/// initial state). The typical body drains the queue and reconciles the cache.
///
/// Must be called inside a reactive owner (e.g. a component body).
pub fn on_reconnect(connectivity: Connectivity, mut on_online: impl FnMut() + 'static) {
  let signal = connectivity.signal();
  let mut was_online = signal.get_untracked();
  Effect::new(move |_| {
    let now = signal.get();
    if now && !was_online {
      on_online();
    }
    was_online = now;
  });
}
