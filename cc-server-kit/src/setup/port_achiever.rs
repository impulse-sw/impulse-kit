//! Port achiever module.

use cc_utils::prelude::*;

pub(crate) async fn port_file_watcher<P: AsRef<std::path::Path>>(path: P) -> MResult<u16> {
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
