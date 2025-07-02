use cc_server_kit::prelude::*;
use dashmap::DashMap;
use fs_change_notifier::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// In-memory cache implementation for files based on `DashMap`.
pub struct CacheMap(DashMap<PathBuf, Vec<u8>>);

impl CacheMap {
  /// Create in-memory cache.
  pub fn new() -> Arc<Self> {
    Arc::new(Self(DashMap::new()))
  }

  /// Clear cache.
  pub fn clear(&self) {
    self.0.clear();
  }

  /// Insert or update content by its path.
  pub fn upsert(&self, path: impl AsRef<Path>, content: Vec<u8>) -> MResult<()> {
    let path = std::fs::canonicalize(path.as_ref()).map_err(ServerError::from_private)?;
    self.0.insert(path, content);
    Ok(())
  }

  /// Fetch content.
  pub fn fetch(&self, path: impl AsRef<Path>) -> MResult<Option<Vec<u8>>> {
    let path = std::fs::canonicalize(path.as_ref()).map_err(ServerError::from_private)?;
    Ok(self.0.get(&path).map(|r#ref| r#ref.value().to_vec()))
  }

  /// Remove content due to invalidation.
  pub fn invalidate(&self, path: impl AsRef<Path>) -> MResult<()> {
    let path = std::fs::canonicalize(path.as_ref()).map_err(ServerError::from_private)?;
    self.0.remove(&path);
    Ok(())
  }
}

/// Cache invalidator runner.
pub async fn cache_runner(path: impl AsRef<Path>, cache_map: Arc<CacheMap>) -> MResult<()> {
  let empty = std::collections::HashSet::new();
  loop {
    let (mut wr, rx) = create_watcher(|e| tracing::error!("{e:?}")).map_err(|e| {
      ServerError::from_private_str(e.to_string())
        .with_public("Can't create FS watcher to cache runner!")
        .with_500()
    })?;
    wr.watch(path.as_ref(), RecursiveMode::Recursive)
      .map_err(ServerError::from_private)?;

    let files = fetch_changed(path.as_ref(), rx, &empty).await;
    for file in files {
      cache_map.invalidate(file)?;
    }
  }
}
