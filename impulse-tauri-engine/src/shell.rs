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
//! A ping every [`PING_INTERVAL`](crate::shell::PING_INTERVAL) draws a pong back, which keeps a merely quiet
//! link looking alive; hearing nothing at all for
//! [`IDLE_TIMEOUT`](crate::shell::IDLE_TIMEOUT) ends the
//! stream, and the loop above dials again.
//!
//! [`wake`](crate::lifecycle::wake) is the fast path over the same problem.
//! Returning to the foreground is when the connection is most likely to be a
//! corpse, and there is no reason to spend even the idle timeout rediscovering
//! that. It is not a replacement for the timeout — see
//! [`lifecycle`](crate::lifecycle) for why both are needed.
//!
//! ## Writes are bounded too
//!
//! Reading is not the only side that can wait forever. A write to a peer that
//! stopped listening fills the kernel's send buffer and then never completes,
//! and since the write half is shared with the keepalive, one such write freezes
//! every other user of this socket. [`WRITE_TIMEOUT`](crate::shell::WRITE_TIMEOUT)
//! caps that: a frame that
//! cannot be handed to the socket in time is a lost connection, reported as an
//! error so the engine drops the socket and reconnects.

use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::lifecycle;
use crate::{WsSink, WsStream};

/// Re-exported so a shell can build its handshake request — adding gateway
/// signatures, say — without depending on tungstenite itself.
pub use tokio_tungstenite::tungstenite;

/// Signals a resume to every live socket. Re-exported from the crate's
/// `lifecycle` module, where the reconnect loop reads it too.
pub use crate::lifecycle::wake;

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// How often the socket pings while otherwise idle.
///
/// Fast, because the timeout below is: probing every second is what makes three
/// seconds of silence *mean* something, rather than being an ordinary gap in an
/// app protocol that only speaks when there is news. The cost is a radio wake-up
/// a second while the app is in the foreground, and none at all once the OS
/// freezes it.
pub const PING_INTERVAL: Duration = Duration::from_secs(1);

/// How long the socket may hear nothing at all before it is declared dead.
///
/// Three whole [`PING_INTERVAL`]s, so a single lost ping is never enough to trip
/// it, and short enough that a link which died while its owner was looking at
/// the screen is noticed in about the time it takes to wonder why nothing has
/// moved. A resume — the other case where the connection is usually dead — does
/// not wait for this at all: [`wake`] ends it immediately.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(3);

/// How long a single frame may take to reach the socket before the connection is
/// declared lost. See the module docs.
///
/// Deliberately tighter than the engine's own
/// [`ReconnectPolicy::write_timeout`](crate::ReconnectPolicy::write_timeout),
/// which wraps this one: the inner bound should be what fires, so a stalled
/// write is reported as an error rather than cancelled mid-frame by the layer
/// above.
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

/// The write half, shared with the keepalive task.
pub struct SocketSink(Arc<Mutex<SplitSink<Ws, Message>>>);

/// The read half, which ends the stream on silence or on resume.
pub struct SocketStream(SplitStream<Ws>);

impl WsSink for SocketSink {
  async fn send(&mut self, frame: String) -> Result<(), String> {
    write_bounded(&self.0, Message::Text(frame)).await
  }
}

/// Writes one message, treating "still not written after [`WRITE_TIMEOUT`]" as a
/// broken socket. Taking the lock is inside the bound on purpose: waiting for
/// another wedged writer is the same stall as being wedged.
async fn write_bounded(sink: &Mutex<SplitSink<Ws, Message>>, msg: Message) -> Result<(), String> {
  let write = async { sink.lock().await.send(msg).await.map_err(|e| e.to_string()) };
  match tokio::time::timeout(WRITE_TIMEOUT, write).await {
    Ok(result) => result,
    Err(_) => Err(format!("socket write stalled for {WRITE_TIMEOUT:?}")),
  }
}

impl WsStream for SocketStream {
  async fn recv(&mut self) -> Option<Result<String, String>> {
    loop {
      let next = tokio::select! {
        // Checked first: on resume there is no point reading a socket that the
        // freeze almost certainly killed.
        biased;
        _ = lifecycle::resumed() => {
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
///
/// The handshake itself is not bounded here: the caller that owns the retry owns
/// the deadline, and that is
/// [`ReconnectPolicy::connect_timeout`](crate::ReconnectPolicy::connect_timeout),
/// which covers whatever else a shell does per attempt (minting a ticket, say)
/// as well as this.
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
      // A ping that can't be written means the socket is gone (or wedged, which
      // amounts to the same thing). Stop: the read side's idle timeout is already
      // running the connection down, and a keepalive that keeps retrying would
      // only hold the write lock the next attempt needs.
      if write_bounded(&sink, Message::Ping(Vec::new())).await.is_err() {
        return;
      }
    }
  });
}
