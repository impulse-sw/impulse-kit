//! Streaming and connection-upgrade transport over Ring channels.
//!
//! The base Ring bus is a single-shot RPC (`RingHttpRequest` → `RingHttpResponse`).
//! SSE, WebSocket and WebTransport need a continuous flow, so they are layered on
//! top of Ring **channels** (`publish_channel`/`subscribe`): the initial RPC only
//! negotiates the session and names the channels, and the bytes themselves travel
//! as [`RingStreamFrame`]s.
//!
//! This module provides the shared plumbing reused by the client API (and by the
//! LBRP `impring://` connector):
//!
//! - [`RingDuplex`] — an `AsyncRead + AsyncWrite` byte stream bridging a Ring
//!   channel pair to ordinary async IO (so a standard HTTP/WebSocket codec can run
//!   over it; this is how WebSocket works end-to-end).
//! - [`RingEventStream`] — a `Stream` of server→client byte chunks (used for SSE,
//!   where the listener streams the response body straight onto one channel).
//! - channel helpers ([`publish_stream_channel`], [`subscribe_by_name`]).

use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use bytes::{Buf, Bytes};
use futures_core::Stream;
use impulse_ring_connector::{Connection, Publisher, Subscriber};
use impulse_ring_http::{RingStreamFrame, STREAM_SCHEMA, opcode};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;

/// How long a channel-draining thread waits per `recv` before re-checking that
/// its consumer is still alive (so a dropped duplex tears the thread down).
const RECV_TICK: Duration = Duration::from_millis(250);

/// How long [`subscribe_by_name`] waits for a freshly published channel to appear.
const SUBSCRIBE_WAIT: Duration = Duration::from_secs(5);

/// Publish a streaming channel carrying [`RingStreamFrame`]s.
pub fn publish_stream_channel(conn: &Connection, name: &str, key: Option<&str>) -> io::Result<Publisher> {
  conn.publish_channel(name, STREAM_SCHEMA, key)
}

/// Resolve a channel by name and subscribe to it.
///
/// The publisher and subscriber race on startup, so this retries briefly while
/// the channel is being registered on the broker.
pub fn subscribe_by_name(conn: &Connection, name: &str, key: Option<&str>) -> io::Result<Subscriber> {
  let deadline = Instant::now() + SUBSCRIBE_WAIT;
  loop {
    for ci in conn.list_channels()? {
      if ci.name == name {
        return conn.subscribe(ci.channel_id, key, STREAM_SCHEMA);
      }
    }
    if Instant::now() >= deadline {
      return Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("ring stream channel '{name}' did not appear"),
      ));
    }
    std::thread::sleep(Duration::from_millis(50));
  }
}

/// Drain a [`Subscriber`] of [`RingStreamFrame`]s on a dedicated thread, forwarding
/// `DATA`/`STREAM_DATA` payloads to `tx` and stopping on `CLOSE`/error/`tx` drop.
fn spawn_inbound_pump(subscriber: Subscriber, tx: mpsc::Sender<io::Result<Bytes>>) {
  std::thread::spawn(move || {
    loop {
      match subscriber.recv::<RingStreamFrame>(RECV_TICK) {
        Ok(Some(frame)) => match frame.opcode {
          opcode::DATA | opcode::STREAM_DATA | opcode::DATAGRAM => {
            if tx.blocking_send(Ok(Bytes::from(frame.payload))).is_err() {
              break;
            }
          }
          opcode::CLOSE | opcode::STREAM_CLOSE => break,
          _ => {}
        },
        // Timeout: nothing arrived this tick. Stop if the consumer went away.
        Ok(None) => {
          if tx.is_closed() {
            break;
          }
        }
        Err(e) => {
          let _ = tx.blocking_send(Err(e));
          break;
        }
      }
    }
    // Dropping `tx` here signals EOF to the consumer.
  });
}

/// A bidirectional raw-byte stream carried over a pair of Ring channels.
///
/// Writes are framed as [`opcode::DATA`] [`RingStreamFrame`]s on the outbound
/// channel; inbound `DATA` frames are surfaced as readable bytes. A clean
/// shutdown emits an [`opcode::CLOSE`] frame. Because it is `AsyncRead +
/// AsyncWrite`, a standard HTTP/1.1 or WebSocket codec can be driven over it —
/// which is exactly how the WebSocket path works: salvo on the server side and a
/// WebSocket client on this side both speak their normal protocol over this
/// "virtual socket", while Ring just relays the bytes.
pub struct RingDuplex {
  // Keep the bus connection alive for as long as the duplex (and its channels) live.
  _conn: Arc<Connection>,
  inbound: mpsc::Receiver<io::Result<Bytes>>,
  read_rem: Bytes,
  inbound_done: bool,
  outbound: Option<mpsc::UnboundedSender<Vec<u8>>>,
}

impl RingDuplex {
  /// Bridge `publisher` (our outbound direction) and `subscriber` (inbound) into
  /// an async byte stream. `conn` is retained so the bus outlives the channels.
  pub fn new(conn: Arc<Connection>, publisher: Publisher, subscriber: Subscriber) -> Self {
    let (in_tx, in_rx) = mpsc::channel::<io::Result<Bytes>>(16);
    spawn_inbound_pump(subscriber, in_tx);

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    std::thread::spawn(move || {
      while let Some(chunk) = out_rx.blocking_recv() {
        if publisher.publish(&RingStreamFrame::data(chunk)).is_err() {
          return;
        }
      }
      // Sender dropped (shutdown): announce a clean close to the peer.
      let _ = publisher.publish(&RingStreamFrame::close());
    });

    RingDuplex {
      _conn: conn,
      inbound: in_rx,
      read_rem: Bytes::new(),
      inbound_done: false,
      outbound: Some(out_tx),
    }
  }
}

impl AsyncRead for RingDuplex {
  fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
    if !self.read_rem.is_empty() {
      let n = self.read_rem.len().min(buf.remaining());
      buf.put_slice(&self.read_rem[..n]);
      self.read_rem.advance(n);
      return Poll::Ready(Ok(()));
    }
    if self.inbound_done {
      return Poll::Ready(Ok(())); // EOF
    }
    match self.inbound.poll_recv(cx) {
      Poll::Ready(Some(Ok(mut chunk))) => {
        let n = chunk.len().min(buf.remaining());
        buf.put_slice(&chunk[..n]);
        chunk.advance(n);
        self.read_rem = chunk;
        Poll::Ready(Ok(()))
      }
      Poll::Ready(Some(Err(e))) => {
        self.inbound_done = true;
        Poll::Ready(Err(e))
      }
      Poll::Ready(None) => {
        self.inbound_done = true;
        Poll::Ready(Ok(())) // EOF
      }
      Poll::Pending => Poll::Pending,
    }
  }
}

impl AsyncWrite for RingDuplex {
  fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
    match &self.outbound {
      Some(tx) => match tx.send(buf.to_vec()) {
        Ok(()) => Poll::Ready(Ok(buf.len())),
        Err(_) => Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "ring outbound closed"))),
      },
      None => Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "ring duplex shut down"))),
    }
  }

  fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    Poll::Ready(Ok(()))
  }

  fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    // Drop the sender so the outbound pump emits CLOSE and exits.
    self.outbound = None;
    Poll::Ready(Ok(()))
  }
}

/// Publish frames coming from `rx` on `publisher` until the channel drains.
fn spawn_frame_publisher(publisher: Publisher, mut rx: mpsc::UnboundedReceiver<RingStreamFrame>) {
  std::thread::spawn(move || {
    while let Some(frame) = rx.blocking_recv() {
      if publisher.publish(&frame).is_err() {
        return;
      }
    }
    let _ = publisher.publish(&RingStreamFrame::close());
  });
}

// ===========================================================================
// WebTransport over Ring
// ===========================================================================
//
// A WebTransport session multiplexes datagrams and bidirectional streams over a
// single Ring channel pair, framed by [`RingStreamFrame`]'s `opcode`/`stream_id`:
// `DATAGRAM` for datagrams, and `STREAM_OPEN`/`STREAM_DATA`/`STREAM_CLOSE`
// (keyed by `stream_id`) for streams. Real WebTransport (QUIC/HTTP3) is
// terminated by salvo at the edge (e.g. LBRP); this is the Ring-side mirror of
// its session API used by the impring service and by direct clients.
//
// Stream ids use QUIC-style parity to avoid collisions: the session `initiator`
// opens even ids, the peer opens odd ids.

/// One bidirectional WebTransport stream within a [`RingWebTransport`] session.
///
/// Besides the explicit [`send`](RingWtStream::send)/[`recv`](RingWtStream::recv)
/// API it also implements [`AsyncRead`]/[`AsyncWrite`], so it can be relayed to
/// another byte stream (e.g. a QUIC WebTransport stream at the LBRP edge) with
/// the usual tunnel helpers.
pub struct RingWtStream {
  stream_id: i64,
  out: mpsc::UnboundedSender<RingStreamFrame>,
  inbound: mpsc::Receiver<Bytes>,
  read_rem: Bytes,
  closed: bool,
}

impl RingWtStream {
  /// This stream's id within the session.
  pub fn id(&self) -> i64 {
    self.stream_id
  }

  /// Send bytes on this stream.
  pub fn send(&self, data: impl Into<Vec<u8>>) -> io::Result<()> {
    self
      .out
      .send(RingStreamFrame {
        opcode: opcode::STREAM_DATA,
        stream_id: self.stream_id,
        payload: data.into(),
      })
      .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "webtransport session closed"))
  }

  /// Receive the next chunk, or `None` once the peer closes the stream.
  pub async fn recv(&mut self) -> Option<Bytes> {
    self.inbound.recv().await
  }

  /// Close this stream, notifying the peer.
  pub fn close(&mut self) {
    if !self.closed {
      self.closed = true;
      let _ = self.out.send(RingStreamFrame {
        opcode: opcode::STREAM_CLOSE,
        stream_id: self.stream_id,
        payload: Vec::new(),
      });
    }
  }
}

impl Drop for RingWtStream {
  fn drop(&mut self) {
    self.close();
  }
}

impl AsyncRead for RingWtStream {
  fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
    if !self.read_rem.is_empty() {
      let n = self.read_rem.len().min(buf.remaining());
      buf.put_slice(&self.read_rem[..n]);
      self.read_rem.advance(n);
      return Poll::Ready(Ok(()));
    }
    match self.inbound.poll_recv(cx) {
      Poll::Ready(Some(mut chunk)) => {
        let n = chunk.len().min(buf.remaining());
        buf.put_slice(&chunk[..n]);
        chunk.advance(n);
        self.read_rem = chunk;
        Poll::Ready(Ok(()))
      }
      Poll::Ready(None) => Poll::Ready(Ok(())), // peer closed → EOF
      Poll::Pending => Poll::Pending,
    }
  }
}

impl AsyncWrite for RingWtStream {
  fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
    match self.send(buf.to_vec()) {
      Ok(()) => Poll::Ready(Ok(buf.len())),
      Err(e) => Poll::Ready(Err(e)),
    }
  }

  fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    Poll::Ready(Ok(()))
  }

  fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    self.close();
    Poll::Ready(Ok(()))
  }
}

type StreamRegistry = Arc<Mutex<HashMap<i64, mpsc::Sender<Bytes>>>>;

/// The cheap-clone send half of a [`RingWebTransport`] session: send datagrams
/// and open new bidirectional streams. Obtained via [`RingWebTransport::split`].
#[derive(Clone)]
pub struct RingWtSender {
  out: mpsc::UnboundedSender<RingStreamFrame>,
  next_stream_id: Arc<AtomicI64>,
  registry: StreamRegistry,
}

impl RingWtSender {
  /// Send a datagram.
  pub fn send_datagram(&self, data: impl Into<Vec<u8>>) -> io::Result<()> {
    self
      .out
      .send(RingStreamFrame::datagram(data.into()))
      .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "webtransport session closed"))
  }

  /// Open a new bidirectional stream.
  pub fn open_bi(&self) -> io::Result<RingWtStream> {
    let stream_id = self.next_stream_id.fetch_add(2, Ordering::Relaxed);
    let (in_tx, in_rx) = mpsc::channel::<Bytes>(32);
    self.registry.lock().unwrap().insert(stream_id, in_tx);
    self
      .out
      .send(RingStreamFrame {
        opcode: opcode::STREAM_OPEN,
        stream_id,
        payload: Vec::new(),
      })
      .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "webtransport session closed"))?;
    Ok(RingWtStream {
      stream_id,
      out: self.out.clone(),
      inbound: in_rx,
      read_rem: Bytes::new(),
      closed: false,
    })
  }
}

/// A WebTransport session over a Ring channel pair (datagrams + bidi streams).
///
/// Symmetric: either end can send/receive datagrams and open/accept streams.
pub struct RingWebTransport {
  _conn: Arc<Connection>,
  out: mpsc::UnboundedSender<RingStreamFrame>,
  datagrams: mpsc::Receiver<Bytes>,
  incoming: mpsc::Receiver<RingWtStream>,
  next_stream_id: Arc<AtomicI64>,
  registry: StreamRegistry,
}

impl RingWebTransport {
  /// Build a session from a publisher (outbound) and subscriber (inbound).
  ///
  /// `initiator` selects the stream-id parity (even when `true`, odd otherwise),
  /// mirroring QUIC so the two ends never pick the same id.
  pub fn new(conn: Arc<Connection>, publisher: Publisher, subscriber: Subscriber, initiator: bool) -> Self {
    let (out_tx, out_rx) = mpsc::unbounded_channel::<RingStreamFrame>();
    spawn_frame_publisher(publisher, out_rx);

    let (dgram_tx, dgram_rx) = mpsc::channel::<Bytes>(64);
    let (incoming_tx, incoming_rx) = mpsc::channel::<RingWtStream>(16);
    let registry: StreamRegistry = Arc::new(Mutex::new(HashMap::new()));

    spawn_wt_demux(subscriber, dgram_tx, incoming_tx, registry.clone(), out_tx.clone());

    RingWebTransport {
      _conn: conn,
      out: out_tx,
      datagrams: dgram_rx,
      incoming: incoming_rx,
      next_stream_id: Arc::new(AtomicI64::new(if initiator { 0 } else { 1 })),
      registry,
    }
  }

  /// A cheap-clone handle for sending datagrams and opening streams.
  pub fn sender(&self) -> RingWtSender {
    RingWtSender {
      out: self.out.clone(),
      next_stream_id: self.next_stream_id.clone(),
      registry: self.registry.clone(),
    }
  }

  /// Split the session into its send half and the two receive halves
  /// (datagrams, peer-opened streams), so the directions can be driven
  /// concurrently — used by the LBRP WebTransport relay.
  pub fn split(self) -> (RingWtSender, mpsc::Receiver<Bytes>, mpsc::Receiver<RingWtStream>) {
    let sender = self.sender();
    (sender, self.datagrams, self.incoming)
  }

  /// Send a datagram.
  pub fn send_datagram(&self, data: impl Into<Vec<u8>>) -> io::Result<()> {
    self.sender().send_datagram(data)
  }

  /// Receive the next datagram, or `None` once the session ends.
  pub async fn recv_datagram(&mut self) -> Option<Bytes> {
    self.datagrams.recv().await
  }

  /// Open a new bidirectional stream.
  pub fn open_bi(&self) -> io::Result<RingWtStream> {
    self.sender().open_bi()
  }

  /// Accept the next stream opened by the peer, or `None` once the session ends.
  pub async fn accept_bi(&mut self) -> Option<RingWtStream> {
    self.incoming.recv().await
  }
}

/// Demultiplex an inbound WebTransport channel into datagrams, accepted streams
/// and per-stream data.
fn spawn_wt_demux(
  subscriber: Subscriber,
  dgram_tx: mpsc::Sender<Bytes>,
  incoming_tx: mpsc::Sender<RingWtStream>,
  registry: StreamRegistry,
  out: mpsc::UnboundedSender<RingStreamFrame>,
) {
  std::thread::spawn(move || {
    loop {
      match subscriber.recv::<RingStreamFrame>(RECV_TICK) {
        Ok(Some(frame)) => match frame.opcode {
          opcode::DATAGRAM => {
            if dgram_tx.blocking_send(Bytes::from(frame.payload)).is_err() {
              break;
            }
          }
          opcode::STREAM_OPEN => {
            let (in_tx, in_rx) = mpsc::channel::<Bytes>(32);
            registry.lock().unwrap().insert(frame.stream_id, in_tx);
            let stream = RingWtStream {
              stream_id: frame.stream_id,
              out: out.clone(),
              inbound: in_rx,
              read_rem: Bytes::new(),
              closed: false,
            };
            if incoming_tx.blocking_send(stream).is_err() {
              break;
            }
          }
          opcode::STREAM_DATA => {
            let sender = registry.lock().unwrap().get(&frame.stream_id).cloned();
            if let Some(tx) = sender {
              let _ = tx.blocking_send(Bytes::from(frame.payload));
            }
          }
          opcode::STREAM_CLOSE => {
            registry.lock().unwrap().remove(&frame.stream_id);
          }
          opcode::CLOSE => break,
          _ => {}
        },
        Ok(None) => {
          if dgram_tx.is_closed() && incoming_tx.is_closed() {
            break;
          }
        }
        Err(_) => break,
      }
    }
  });
}

/// A server→client stream of byte chunks over a single Ring channel.
///
/// Used for SSE: the listener streams the response body straight onto the
/// down-channel as [`opcode::DATA`] frames, and this surfaces those chunks as a
/// `Stream`. Each item is one frame's payload (already the raw SSE bytes).
pub struct RingEventStream {
  _conn: Arc<Connection>,
  inbound: mpsc::Receiver<io::Result<Bytes>>,
}

impl RingEventStream {
  /// Wrap a down-channel `subscriber` as a byte stream. `conn` is retained so the
  /// bus outlives the channel.
  pub fn new(conn: Arc<Connection>, subscriber: Subscriber) -> Self {
    let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(16);
    spawn_inbound_pump(subscriber, tx);
    RingEventStream { _conn: conn, inbound: rx }
  }

  /// Await the next chunk, or `None` at end of stream.
  pub async fn recv(&mut self) -> Option<io::Result<Bytes>> {
    self.inbound.recv().await
  }
}

impl Stream for RingEventStream {
  type Item = io::Result<Bytes>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    self.inbound.poll_recv(cx)
  }
}
