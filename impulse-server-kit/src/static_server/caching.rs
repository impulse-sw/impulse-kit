//! In-memory caching layer for the static-file handlers.
//!
//! Tracks file bytes alongside cached `ETag` / `Last-Modified` metadata and a
//! filesystem watcher (`cache_runner`) that invalidates entries when the
//! underlying file changes.
//!
//! Every response carries `Cache-Control: public, max-age=0, must-revalidate`
//! so browsers always revalidate against the served `ETag` instead of holding a
//! heuristically-fresh copy — without it, a redeployed CSR/WASM bundle is not
//! picked up until a forced refresh.

use fs_change_notifier::*;
use headers::{
  AcceptRanges, ContentLength, ContentRange, ContentType, ETag, HeaderMapExt, IfMatch, IfModifiedSince, IfNoneMatch,
  IfUnmodifiedSince, LastModified,
};
use salvo::fs::NamedFile;
use salvo::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_TYPE, IF_NONE_MATCH, RANGE};
use salvo::http::{HeaderMap, HeaderValue};
use std::cmp;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use salvo::http::HttpRange;

use crate::prelude::*;

/// `Cache-Control` applied to every cached response.
///
/// `max-age=0, must-revalidate` forces the browser to revalidate with the
/// server on every use (via `ETag` / `Last-Modified`) instead of serving a
/// heuristically-fresh copy without contacting us. Combined with the strong
/// `ETag`, revalidation is cheap — an unchanged file yields an empty `304` —
/// while a redeploy is picked up immediately rather than after a hard refresh.
/// This matches the streaming `file_upload!` path so cached and streamed
/// responses behave identically.
const CACHE_CONTROL_REVALIDATE: HeaderValue = HeaderValue::from_static("public, max-age=0, must-revalidate");

/// Single cached file entry.
#[derive(Clone)]
pub struct CachedFile {
  /// Strong `ETag` derived from file metadata.
  pub etag: Option<ETag>,
  /// Last-modified timestamp.
  pub last_modified: Option<std::time::SystemTime>,
  /// MIME type guessed from the filename.
  pub content_type: mime::Mime,
  /// Optional `Content-Encoding` value (gzip/br/etc.).
  pub content_encoding: Option<HeaderValue>,
  /// File length in bytes (matches `bytes.len()`).
  pub length: u64,
  /// Pre-built `Content-Disposition` header, when applicable.
  pub content_disposition: Option<String>,
  /// File contents loaded into memory.
  pub bytes: Vec<u8>,
}

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
  /// Read a file from disk and build a cache entry.
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

  /// Write the cached file to the response, respecting conditional and range headers.
  pub async fn send(&self, req_headers: &HeaderMap, res: &mut Response) {
    let precondition_failed = if !any_match(self.etag.as_ref(), req_headers) {
      true
    } else if let (Some(last_modified), Some(since)) =
      (&self.last_modified, req_headers.typed_get::<IfUnmodifiedSince>())
    {
      !since.precondition_passes(*last_modified)
    } else {
      false
    };

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
    res.headers_mut().insert(CACHE_CONTROL, CACHE_CONTROL_REVALIDATE);

    let mut length = self.length;
    if let Some(content_encoding) = &self.content_encoding {
      res.headers_mut().insert(CONTENT_ENCODING, content_encoding.clone());
    }
    let mut offset = 0;

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

      // Send exactly the requested range from the in-memory bytes. Capping the
      // body at a fixed chunk size while advertising the full range in
      // Content-Length would leave clients hanging on the missing bytes; cached
      // files are already fully buffered, so there is nothing to stream in
      // chunks.
      let start = offset as usize;
      let end = (start + total_size as usize).min(self.bytes.len());
      let buf = self.bytes[start..end].to_vec();

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

/// Send a file from cache or disk, falling back to streaming for large files.
pub async fn send_file(
  cacher: &Option<Arc<Mutex<CacheMap>>>,
  req: &mut Request,
  depot: &mut Depot,
  res: &mut Response,
  filename: &str,
  path: &Path,
) -> MResult<()> {
  use salvo::Writer;

  if let Some(cacher) = cacher.as_ref() {
    {
      let guard = cacher.lock().await;
      if let Ok(Some(cached)) = guard.fetch(path) {
        tracing::debug!(file = %filename, "static file served from cache");
        cached.send(req.headers(), res).await;
        return Ok(());
      }
    }
    let length = tokio::fs::metadata(path)
      .await
      .map_err(|e| ServerError::from_private(e).with_404())?
      .len();
    if length > 16 * 1024 * 1024 {
      tracing::debug!(file = %filename, length, "static file streamed from disk (too large to cache)");
      file_upload!(path.to_path_buf(), filename.to_string())
        .write(req, depot, res)
        .await;
    } else {
      tracing::debug!(file = %filename, length, "static file read from disk and cached");
      let cached = CachedFile::construct_from(filename, path, length).await?;
      {
        let mut guard = cacher.lock().await;
        guard.upsert(path, cached.clone())?;
      }
      cached.send(req.headers(), res).await;
    }
  } else {
    tracing::debug!(file = %filename, "static file streamed from disk (no cacher)");
    file_upload!(path.to_path_buf(), filename.to_string())
      .write(req, depot, res)
      .await;
  }
  Ok(())
}

/// Send an HTML page from cache or disk.
pub async fn send_html(
  cacher: &Option<Arc<Mutex<CacheMap>>>,
  req: &mut Request,
  depot: &mut Depot,
  res: &mut Response,
  filename: &str,
  path: &Path,
) -> MResult<()> {
  use salvo::Writer;

  // The HTML shell must always be revalidated so a redeploy is picked up without
  // a forced refresh; `html!` renders the body but never touches this header.
  res.headers_mut().insert(CACHE_CONTROL, CACHE_CONTROL_REVALIDATE);

  if let Some(cacher) = cacher.as_ref() {
    let cached = {
      let guard = cacher.lock().await;
      guard.fetch(path)
    };
    if let Ok(Some(cached)) = cached {
      tracing::debug!(file = %filename, "html served from cache");
      let site = String::from_utf8_lossy(&cached.bytes).into_owned();
      html!(site).unwrap().write(req, depot, res).await;
      return Ok(());
    }
    let length = tokio::fs::metadata(path)
      .await
      .map_err(|e| ServerError::from_private(e).with_404())?
      .len();
    if length > 16 * 1024 * 1024 {
      tracing::debug!(file = %filename, length, "html streamed from disk (too large to cache)");
      let site = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| ServerError::from_private(e).with_404())?;
      html!(site).unwrap().write(req, depot, res).await;
    } else {
      tracing::debug!(file = %filename, length, "html read from disk and cached");
      let cached = CachedFile::construct_from(filename, path, length).await?;
      {
        let mut guard = cacher.lock().await;
        guard.upsert(path, cached.clone())?;
      }
      let site = String::from_utf8_lossy(&cached.bytes).into_owned();
      html!(site).unwrap().write(req, depot, res).await;
    }
  } else {
    tracing::debug!(file = %filename, "html streamed from disk (no cacher)");
    let site = tokio::fs::read_to_string(path)
      .await
      .map_err(|e| ServerError::from_private(e).with_404())?;
    html!(site).unwrap().write(req, depot, res).await;
  }
  Ok(())
}

/// In-memory cache for static files, keyed by canonicalised path.
pub struct CacheMap(HashMap<PathBuf, CachedFile>);

impl CacheMap {
  /// Create a new shared cache.
  pub fn new() -> Arc<Mutex<Self>> {
    Arc::new(Mutex::new(Self(HashMap::new())))
  }

  /// Drop all cached entries.
  pub fn clear(&mut self) {
    self.0.clear();
  }

  /// Insert or replace the cached entry for `path`.
  pub fn upsert(&mut self, path: impl AsRef<Path>, content: CachedFile) -> MResult<()> {
    let path = std::fs::canonicalize(path.as_ref()).map_err(ServerError::from_private)?;
    self.0.insert(path, content);
    Ok(())
  }

  /// Look up a cached entry.
  pub fn fetch(&self, path: impl AsRef<Path>) -> MResult<Option<CachedFile>> {
    let path = std::fs::canonicalize(path.as_ref()).map_err(ServerError::from_private)?;
    Ok(self.0.get(&path).cloned())
  }

  /// Remove a cached entry after detecting a filesystem change.
  pub fn invalidate(&mut self, path: impl AsRef<Path>) -> MResult<()> {
    let path = std::fs::canonicalize(path.as_ref()).map_err(ServerError::from_private)?;
    self.0.remove(&path);
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn dummy_entry() -> CachedFile {
    CachedFile {
      etag: "\"abc\"".parse::<ETag>().ok(),
      last_modified: Some(std::time::SystemTime::UNIX_EPOCH),
      content_type: mime::TEXT_JAVASCRIPT,
      content_encoding: None,
      length: 3,
      content_disposition: None,
      bytes: b"foo".to_vec(),
    }
  }

  #[tokio::test]
  async fn full_response_carries_revalidate_cache_control() {
    let entry = dummy_entry();
    let mut res = Response::new();
    entry.send(&HeaderMap::new(), &mut res).await;

    assert_eq!(res.status_code, Some(StatusCode::OK));
    assert_eq!(
      res.headers().get(CACHE_CONTROL).and_then(|v| v.to_str().ok()),
      Some("public, max-age=0, must-revalidate"),
    );
  }

  #[tokio::test]
  async fn not_modified_response_still_carries_cache_control() {
    // A matching `If-None-Match` yields `304`; the browser must still learn how
    // long it may keep serving the copy before revalidating again.
    let entry = dummy_entry();
    let mut req_headers = HeaderMap::new();
    req_headers.typed_insert(IfNoneMatch::from(entry.etag.clone().unwrap()));

    let mut res = Response::new();
    entry.send(&req_headers, &mut res).await;

    assert_eq!(res.status_code, Some(StatusCode::NOT_MODIFIED));
    assert_eq!(
      res.headers().get(CACHE_CONTROL).and_then(|v| v.to_str().ok()),
      Some("public, max-age=0, must-revalidate"),
    );
  }
}

/// Background task that invalidates cache entries on filesystem changes.
pub async fn cache_runner(path: impl AsRef<Path>, cache_map: Arc<Mutex<CacheMap>>) -> MResult<()> {
  // `fetch_changed` only collects a changed path while iterating `exclude`, so
  // an empty exclude set makes its inner loop run zero times: nothing is ever
  // collected and the future never resolves, leaving the cache permanently
  // stale. Express "exclude nothing" with a single sentinel that can never
  // match a real relative path (filenames cannot contain a NUL byte) so the
  // outer loop still runs once per changed file and every change is reported.
  let exclude = std::collections::HashSet::from([PathBuf::from("\0")]);
  loop {
    let (mut wr, rx) = create_watcher(|e| tracing::error!("{e:?}")).map_err(|e| {
      ServerError::from_private_str(e.to_string())
        .with_public("Can't create FS watcher to cache runner!")
        .with_500()
    })?;
    wr.watch(path.as_ref(), RecursiveMode::Recursive)
      .map_err(ServerError::from_private)?;

    let files = fetch_changed(path.as_ref(), rx, &exclude).await;
    {
      let mut guard = cache_map.lock().await;
      for file in &files {
        tracing::info!(?file, "invalidating cached static asset");
        guard.invalidate(file)?;
      }
    }
  }
}
