//! The socket a Tauri shell opens, with the handling a phone makes necessary.
//!
//! [`WsEngine`](crate::WsEngine) only needs the [`WsSink`](crate::WsSink) and
//! [`WsStream`](crate::WsStream) traits; this is the concrete pair every app
//! here plugs into them. It lives with the engine rather than in each shell
//! because the hard parts are not app-specific — they are what a socket has to
//! do to survive a mobile OS.
//!
//! ## Why a socket needs more than `connect`
//!
//! Android freezes a backgrounded process. A connection torn down during that
//! freeze leaves no trace to come back to: the FIN never arrives, so reading the
//! socket simply never completes, and a reconnect loop waiting on that read
//! never gets another turn. Nothing retries because nothing has finished.
//!
//! Silence is the only evidence available, so this makes silence mean something.
//! A ping every [`PING_INTERVAL`] draws a pong back, which keeps a merely quiet
//! link looking alive; hearing nothing at all for [`IDLE_TIMEOUT`] ends the
//! stream, and the loop above dials again.
//!
//! [`wake`] is the fast path over the same problem. Returning to the foreground
//! is when the connection is most likely to be a corpse, and waiting out the
//! idle timeout there means the reader stares at a stale screen for the better
//! part of a minute. The signal only reaches a reader already waiting, so a
//! resume landing between loop iterations is missed — which is exactly the case
//! the timeout still covers. Both together make the common case quick without
//! making the rare one broken.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Notify};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::{WsSink, WsStream};

/// Re-exported so a shell can build its handshake request — adding gateway
/// signatures, say — without depending on tungstenite itself.
pub use tokio_tungstenite::tungstenite;

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// How often the socket pings while otherwise idle.
pub const PING_INTERVAL: Duration = Duration::from_secs(20);

/// How long the socket may hear nothing at all before it is declared dead.
///
/// Comfortably more than [`PING_INTERVAL`], so a healthy link always has a pong
/// in flight to reset it — one lost ping is not enough to trip it — and short
/// enough that a phone coming back from doze reconnects while its owner is still
/// looking at the screen.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(50);

/// Signalled when the app returns to the foreground.
fn resume_signal() -> &'static Notify {
  static RESUME: OnceLock<Notify> = OnceLock::new();
  RESUME.get_or_init(Notify::new)
}

/// Tells every live socket that the app just resumed, so it stops waiting on a
/// connection the freeze almost certainly killed.
///
/// A shell calls this from whatever its platform reports as "foreground again" —
/// a Tauri `WindowEvent::Focused(true)`, for instance. See the module docs for
/// why this does not replace [`IDLE_TIMEOUT`].
pub fn wake() {
  resume_signal().notify_waiters();
}

/// The write half, shared with the keepalive task.
pub struct SocketSink(Arc<Mutex<SplitSink<Ws, Message>>>);

/// The read half, which ends the stream on silence or on resume.
pub struct SocketStream(SplitStream<Ws>);

impl WsSink for SocketSink {
  async fn send(&mut self, frame: String) -> Result<(), String> {
    self
      .0
      .lock()
      .await
      .send(Message::Text(frame.into()))
      .await
      .map_err(|e| e.to_string())
  }
}

impl WsStream for SocketStream {
  async fn recv(&mut self) -> Option<Result<String, String>> {
    loop {
      let next = tokio::select! {
        // Checked first: on resume there is no point reading a socket that the
        // freeze almost certainly killed.
        biased;
        _ = resume_signal().notified() => {
          tracing::debug!("app resumed; dropping the socket to reconnect at once");
          return None;
        }
        read = tokio::time::timeout(IDLE_TIMEOUT, self.0.next()) => match read {
          Ok(next) => next,
          Err(_) => {
            tracing::debug!("ws idle for {IDLE_TIMEOUT:?}; treating the connection as dead");
            return None;
          }
        },
      };
      match next {
        Some(Ok(Message::Text(text))) => return Some(Ok(text.as_str().to_owned())),
        // Control and binary frames aren't part of an app protocol; keep reading.
        // Pongs land here, and reaching this point is what resets the timeout.
        Some(Ok(Message::Close(_))) | None => return None,
        Some(Ok(_)) => continue,
        Some(Err(e)) => return Some(Err(e.to_string())),
      }
    }
  }
}

/// Opens a socket and returns the halves the engine drives.
///
/// `request` is anything tungstenite accepts — a `String` URL, or a built
/// `http::Request` when the handshake needs headers of its own (a security
/// gateway's signature, say). Keepalive starts immediately.
pub async fn connect(request: impl IntoClientRequest + Unpin) -> Result<(SocketSink, SocketStream), String> {
  let (ws, _resp) = connect_async(request).await.map_err(|e| e.to_string())?;
  let (sink, stream) = ws.split();
  let sink = Arc::new(Mutex::new(sink));
  spawn_keepalive(&sink);
  Ok((SocketSink(sink), SocketStream(stream)))
}

/// Pings the socket while it is idle, so silence means something.
///
/// An app protocol that only speaks when there is news gives the read side no
/// way to tell a quiet connection from a dead one, and on mobile that difference
/// is the whole problem.
///
/// The task holds a `Weak`, so it ends when the engine drops the connection
/// rather than outliving it and pinging into a socket nobody reads.
fn spawn_keepalive(sink: &Arc<Mutex<SplitSink<Ws, Message>>>) {
  let sink = Arc::downgrade(sink);
  tokio::spawn(async move {
    loop {
      tokio::time::sleep(PING_INTERVAL).await;
      let Some(sink) = sink.upgrade() else {
        return;
      };
      if sink.lock().await.send(Message::Ping(Vec::new().into())).await.is_err() {
        return;
      }
    }
  });
}
