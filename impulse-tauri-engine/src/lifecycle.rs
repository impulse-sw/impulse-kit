//! The one app-lifecycle signal everything waiting on a connection listens to.
//!
//! A mobile OS freezes a backgrounded process, and the connection it froze is
//! usually a corpse by the time the app comes back — with no FIN to say so.
//! Returning to the foreground is therefore the single most useful hint the
//! platform gives, and it is useful to more than one waiter: the socket's read
//! side, a connect attempt still in its handshake, and the backoff sleep between
//! attempts are all, at that moment, waiting on something that will never
//! arrive.
//!
//! So the signal lives here rather than inside the socket, and a shell reports
//! the resume exactly once ([`wake`](crate::lifecycle::wake)) for all of them.
//!
//! It is a fast path, never a substitute for a timeout. [`wake`](crate::lifecycle::wake)
//! only reaches a
//! waiter already parked in [`resumed`](crate::lifecycle::resumed), so a resume
//! landing between two waits
//! is missed by design — which is precisely the case the timeouts around it
//! cover. Both together make the common case quick without making the rare one
//! broken.

use std::sync::OnceLock;

use tokio::sync::Notify;

fn resume_signal() -> &'static Notify {
  static RESUME: OnceLock<Notify> = OnceLock::new();
  RESUME.get_or_init(Notify::new)
}

/// Tells everything currently waiting on a possibly-dead connection that the
/// app just returned to the foreground.
///
/// A shell calls this from whatever its platform reports as "foreground again"
/// — a Tauri `WindowEvent::Focused(true)`, for instance. Nothing else in the
/// engine needs to know what a window is, which is why this is the whole of the
/// platform's side of the contract.
pub fn wake() {
  resume_signal().notify_waiters();
}

/// Completes on the next [`wake`].
///
/// Only a waiter already parked here hears it; see the module docs for why that
/// is deliberate.
pub async fn resumed() {
  resume_signal().notified().await;
}
