use cc_server_kit::prelude::*;
use cc_server_kit::salvo::http::HttpRange;
use dashmap::DashMap;
use fs_change_notifier::*;
use headers::{
  AcceptRanges, ContentLength, ContentRange, ContentType, ETag, HeaderMapExt, IfMatch, IfModifiedSince, IfNoneMatch,
  IfUnmodifiedSince, LastModified,
};
use salvo::fs::NamedFile;
use salvo::http::header::{CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_TYPE, IF_NONE_MATCH, RANGE};
use salvo::http::{HeaderMap, HeaderValue};
use std::cmp;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const CHUNK_SIZE: u64 = 1024 * 1024;

#[derive(Clone)]
pub struct CachedFile {
  pub etag: Option<ETag>,
  pub last_modified: Option<std::time::SystemTime>,
  pub content_type: mime::Mime,
  pub content_encoding: Option<HeaderValue>,
  pub length: u64,
  pub content_disposition: Option<String>,
  pub bytes: Vec<u8>,
}

/// Returns true if `req_headers` has no `If-Match` header or one which matches `etag`.
fn any_match(etag: Option<&ETag>, req_headers: &HeaderMap) -> bool {
  match req_headers.typed_get::<IfMatch>() {
    None => true,
    Some(if_match) => {
      if if_match == IfMatch::any() {
        true
      } else if let Some(etag) = etag {
        if_match.precondition_passes(etag)
      } else {
        false
      }
    }
  }
}

/// Returns true if `req_headers` doesn't have an `If-None-Match` header matching `req`.
fn none_match(etag: Option<&ETag>, req_headers: &HeaderMap) -> bool {
  match req_headers.typed_get::<IfNoneMatch>() {
    None => true,
    Some(if_none_match) => {
      if if_none_match == IfNoneMatch::any() {
        false
      } else if let Some(etag) = etag {
        if_none_match.precondition_passes(etag)
      } else {
        true
      }
    }
  }
}

impl CachedFile {
  pub async fn construct_from(filename: &str, path: &Path, known_length: u64) -> MResult<Self> {
    let named_file = NamedFile::builder(path)
      .attached_name(filename)
      .use_etag(true)
      .use_last_modified(true)
      .build()
      .await
      .map_err(|e| ServerError::from_private(e).with_404())?;

    let etag = named_file.etag();
    let last_modified = named_file.last_modified();
    let content_type = named_file.content_type().clone();
    let content_encoding = named_file.content_encoding().cloned();

    Ok(Self {
      etag,
      last_modified,
      content_type,
      content_encoding,
      content_disposition: Some(format!(r#"attachment; filename="{filename}""#)),
      length: known_length,
      bytes: tokio::fs::read(path)
        .await
        .map_err(|e| ServerError::from_private(e).with_404())?,
    })
  }

  pub async fn send(&self, req_headers: &HeaderMap, res: &mut Response) {
    // check preconditions
    let precondition_failed = if !any_match(self.etag.as_ref(), req_headers) {
      true
    } else if let (Some(last_modified), Some(since)) =
      (&self.last_modified, req_headers.typed_get::<IfUnmodifiedSince>())
    {
      !since.precondition_passes(*last_modified)
    } else {
      false
    };

    // check last modified
    let not_modified = if !none_match(self.etag.as_ref(), req_headers) {
      true
    } else if req_headers.contains_key(IF_NONE_MATCH) {
      false
    } else if let (Some(last_modified), Some(since)) = (&self.last_modified, req_headers.typed_get::<IfModifiedSince>())
    {
      !since.is_modified(*last_modified)
    } else {
      false
    };

    if let Some(content_disposition) = self.content_disposition.as_deref()
      && let Ok(content_disposition) = content_disposition.parse::<HeaderValue>()
    {
      res.headers_mut().insert(CONTENT_DISPOSITION, content_disposition);
    } else {
      res
        .headers_mut()
        .insert(CONTENT_DISPOSITION, HeaderValue::from_static("inline"));
    }
    if !res.headers().contains_key(CONTENT_TYPE) {
      res
        .headers_mut()
        .typed_insert(ContentType::from(self.content_type.clone()));
    }
    if let Some(lm) = self.last_modified {
      res.headers_mut().typed_insert(LastModified::from(lm));
    }
    if let Some(etag) = self.etag.clone() {
      res.headers_mut().typed_insert(etag);
    }
    res.headers_mut().typed_insert(AcceptRanges::bytes());

    let mut length = self.length;
    if let Some(content_encoding) = &self.content_encoding {
      res.headers_mut().insert(CONTENT_ENCODING, content_encoding.clone());
    }
    let mut offset = 0;

    // check for range header
    let range = req_headers.get(RANGE);
    if let Some(range) = range {
      if let Ok(range) = range.to_str() {
        if let Ok(range) = HttpRange::parse(range, length) {
          length = range[0].length;
          offset = range[0].start;
        } else {
          res.headers_mut().typed_insert(ContentRange::unsatisfied_bytes(length));
          res.status_code(StatusCode::RANGE_NOT_SATISFIABLE);
          return;
        };
      } else {
        res.status_code(StatusCode::BAD_REQUEST);
        return;
      };
    }

    if precondition_failed {
      res.status_code(StatusCode::PRECONDITION_FAILED);
      return;
    } else if not_modified {
      res.status_code(StatusCode::NOT_MODIFIED);
      return;
    }

    if offset != 0 || length != self.length || range.is_some() {
      res.status_code(StatusCode::PARTIAL_CONTENT);
      match ContentRange::bytes(offset..offset + length, self.length) {
        Ok(content_range) => {
          res.headers_mut().typed_insert(content_range);
        }
        Err(e) => {
          tracing::error!(error = ?e, "Set file's content range failed");
        }
      }
      let total_size = cmp::min(length, self.length);
      res.headers_mut().typed_insert(ContentLength(total_size));

      let max_bytes = cmp::min(total_size, CHUNK_SIZE) as usize;
      let mut buf = Vec::<u8>::with_capacity(max_bytes);
      let end = (offset as usize + max_bytes).min(self.bytes.len());
      buf.extend_from_slice(&self.bytes[offset as usize..end]);

      if let Err(e) = res.write_body(buf) {
        tracing::error!(error = ?e, "Failed to send file part!");
      }
    } else {
      res.status_code(StatusCode::OK);
      res.headers_mut().typed_insert(ContentLength(length));

      if let Err(e) = res.write_body(self.bytes.clone()) {
        tracing::error!(error = ?e, "Failed to send file part!");
      }
    }
  }
}

/// In-memory cache implementation for files based on `DashMap`.
pub struct CacheMap(DashMap<PathBuf, CachedFile>);

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
  pub fn upsert(&self, path: impl AsRef<Path>, content: CachedFile) -> MResult<()> {
    let path = std::fs::canonicalize(path.as_ref()).map_err(ServerError::from_private)?;
    self.0.insert(path, content);
    Ok(())
  }

  /// Fetch content.
  pub fn fetch(&self, path: impl AsRef<Path>) -> MResult<Option<CachedFile>> {
    let path = std::fs::canonicalize(path.as_ref()).map_err(ServerError::from_private)?;
    Ok(self.0.get(&path).map(|r#ref| r#ref.value().clone()))
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
