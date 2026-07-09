//! Tiny key/value persistence used by the cache and the mutation queue.
//!
//! With the `persist` feature (wasm only) values are stored as JSON in the
//! browser's LocalStorage. Everywhere else these are no-ops that report "nothing
//! stored" — the Tauri engine persists to SQLite through its own channel, and a
//! plain web build without `persist` simply keeps state in memory for the session.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Loads and JSON-decodes the value stored under `key`, if any.
pub fn load<T: DeserializeOwned>(key: &str) -> Option<T> {
  let _ = key;
  #[cfg(all(feature = "persist", any(target_arch = "wasm32", target_arch = "wasm64")))]
  {
    let raw = local_storage()?.get_item(key).ok().flatten()?;
    match serde_json::from_str(&raw) {
      Ok(v) => Some(v),
      Err(e) => {
        log::warn!("impulse-client-store: failed to decode `{key}`: {e}");
        None
      }
    }
  }
  #[cfg(not(all(feature = "persist", any(target_arch = "wasm32", target_arch = "wasm64"))))]
  {
    None
  }
}

/// JSON-encodes `value` and stores it under `key`.
pub fn save<T: Serialize>(key: &str, value: &T) {
  let _ = (key, value);
  #[cfg(all(feature = "persist", any(target_arch = "wasm32", target_arch = "wasm64")))]
  {
    match serde_json::to_string(value) {
      Ok(raw) => {
        if let Some(store) = local_storage() {
          let _ = store.set_item(key, &raw);
        }
      }
      Err(e) => log::warn!("impulse-client-store: failed to encode `{key}`: {e}"),
    }
  }
}

/// Removes any value stored under `key`.
pub fn remove(key: &str) {
  let _ = key;
  #[cfg(all(feature = "persist", any(target_arch = "wasm32", target_arch = "wasm64")))]
  {
    if let Some(store) = local_storage() {
      let _ = store.remove_item(key);
    }
  }
}

#[cfg(all(feature = "persist", any(target_arch = "wasm32", target_arch = "wasm64")))]
fn local_storage() -> Option<web_sys::Storage> {
  web_sys::window()?.local_storage().ok().flatten()
}
