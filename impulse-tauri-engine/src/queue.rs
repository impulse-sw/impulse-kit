//! A tiny persistent journal of offline writes awaiting replay to the server.
//!
//! Each entry is the original [`HttpRequest`] plus a monotonic id and, for a
//! create, the temporary id it minted (its `provisional_id`). The whole journal
//! is rewritten to a JSON file on every change (write-to-temp + rename), so a
//! crash mid-flush never leaves a half-written file. Ordering is preserved so
//! writes replay in the sequence they were made.

use std::path::PathBuf;
use std::sync::Mutex;

use impulse_endpoint::HttpRequest;
use serde::{Deserialize, Serialize};

/// One queued offline write.
#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
  /// Monotonic queue id (for [`Queue::ack`]).
  pub id: u64,
  /// The request to replay.
  pub req: HttpRequest,
  /// For a create, the temporary id it assigned locally. On replay the server
  /// returns the real id, which the engine maps back onto this.
  pub provisional_id: Option<i64>,
}

#[derive(Serialize, Deserialize)]
struct State {
  next_id: u64,
  /// Next temporary id to hand out; decrements, staying negative so it never
  /// collides with a server's positive autoincrement ids.
  next_provisional: i64,
  entries: Vec<Entry>,
}

impl Default for State {
  fn default() -> Self {
    Self {
      next_id: 0,
      next_provisional: -1,
      entries: Vec::new(),
    }
  }
}

/// File-backed FIFO queue of pending offline writes.
pub struct Queue {
  path: PathBuf,
  state: Mutex<State>,
}

impl Queue {
  /// Opens (or creates) the queue at `path`, loading any persisted entries.
  pub fn open(path: PathBuf) -> std::io::Result<Self> {
    let state = match std::fs::read(&path) {
      Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => State::default(),
      Err(e) => return Err(e),
    };
    Ok(Self {
      path,
      state: Mutex::new(state),
    })
  }

  /// Hands out the next temporary (negative) id for an offline create.
  pub fn next_provisional_id(&self) -> i64 {
    let mut state = self.state.lock().expect("queue mutex");
    let id = state.next_provisional;
    state.next_provisional -= 1;
    self.persist(&state);
    id
  }

  /// Appends a request to replay later, tagging a create with the id it minted.
  pub fn enqueue(&self, req: &HttpRequest, provisional_id: Option<i64>) {
    let mut state = self.state.lock().expect("queue mutex");
    let id = state.next_id;
    state.next_id += 1;
    state.entries.push(Entry {
      id,
      req: req.clone(),
      provisional_id,
    });
    self.persist(&state);
  }

  /// Pending entries, oldest first.
  pub fn pending(&self) -> Vec<Entry> {
    self.state.lock().expect("queue mutex").entries.clone()
  }

  /// Drops a replayed entry.
  pub fn ack(&self, id: u64) {
    let mut state = self.state.lock().expect("queue mutex");
    state.entries.retain(|e| e.id != id);
    self.persist(&state);
  }

  /// Drops every pending entry without replaying it.
  ///
  /// For sign-out, and only for sign-out: a queued write belongs to the session
  /// that made it, so replaying it after somebody else signs in on this device
  /// would send one person's work under another person's credentials. The two
  /// counters are deliberately *not* reset — ids stay monotonic, so an entry
  /// enqueued after this can never be confused with one acked before it.
  pub fn clear(&self) {
    let mut state = self.state.lock().expect("queue mutex");
    state.entries.clear();
    self.persist(&state);
  }

  /// Number of pending entries.
  pub fn len(&self) -> usize {
    self.state.lock().expect("queue mutex").entries.len()
  }

  /// Whether the queue is empty.
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  fn persist(&self, state: &State) {
    if let Some(parent) = self.path.parent() {
      let _ = std::fs::create_dir_all(parent);
    }
    let Ok(bytes) = serde_json::to_vec(state) else {
      return;
    };
    let tmp = self.path.with_extension("tmp");
    if std::fs::write(&tmp, &bytes).is_ok() {
      let _ = std::fs::rename(&tmp, &self.path);
    }
  }
}
