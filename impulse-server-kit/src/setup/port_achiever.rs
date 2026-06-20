//! Port achiever module.
//!
//! A small helper for deployments that learn which port to bind from a text
//! file written by an external orchestrator. Await [`port_file_watcher`] before
//! building your `protocols:` list and substitute the resolved port.

use impulse_utils::prelude::*;

/// Watch `path` until it contains a parseable `u16` port, then return it.
pub async fn port_file_watcher<P: AsRef<std::path::Path>>(path: P) -> MResult<u16> {
  use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

  let (tx, mut rx) = tokio::sync::mpsc::channel(1);
  let mut watcher = RecommendedWatcher::new(move |res| tx.blocking_send(res).unwrap(), Config::default())
    .map_err(|e| ServerError::from_private(e).with_500())?;
  watcher
    .watch(path.as_ref(), RecursiveMode::NonRecursive)
    .map_err(|e| ServerError::from_private(e).with_500())?;

  while let Some(res) = rx.recv().await {
    match res {
      Ok(event) if event.kind.is_modify() || event.kind.is_create() => {
        if let Ok(port) = std::fs::read_to_string(path.as_ref())
          && let Ok(port) = port.trim().parse::<u16>()
        {
          watcher
            .unwatch(path.as_ref())
            .map_err(|e| ServerError::from_private(e).with_500())?;
          return Ok(port);
        }
      }
      Err(e) => {
        tracing::error!("Watch error: {:?}", e);
        ServerError::from_private(e).with_500().bail()?;
      }
      _ => {}
    }
  }

  ServerError::from_private_str("Event channel is broken!")
    .with_500()
    .bail()
}
