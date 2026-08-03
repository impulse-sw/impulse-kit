//! Shared automatic-reconnection policy for [`ws`](crate::ws) and
//! [`wt`](crate::wt) handles.
//!
//! When a connection drops unexpectedly the handle can re-establish it on its
//! own, waiting between attempts according to [`ReconnectOptions`]. The delay
//! grows by [`ReconnectOptions::backoff_factor`] after each failed attempt and
//! is capped at [`ReconnectOptions::max_delay`]; the number of consecutive
//! attempts can be bounded with [`ReconnectOptions::max_attempts`] or left
//! unbounded.
//!
//! ## Every wait is a few seconds at most
//!
//! The defaults are deliberately impatient: no single wait here exceeds five
//! seconds, and the usual one is well under two. A reconnect is cheap — a
//! ticket, a handshake, a snapshot — while a user looking at a screen that has
//! quietly stopped updating is not, and on a phone the two are separated only by
//! how long the connection is given to prove itself. When a wait has to be
//! traded off, it is traded towards noticing sooner.
//!
//! Reconnection is **disabled by default** so existing call sites keep their
//! previous one-shot behaviour.
//!
//! ```rust,ignore
//! use std::time::Duration;
//! use impulse_client_kit::reconnect::ReconnectOptions;
//!
//! // Retry forever, starting at 250ms and doubling up to 2s.
//! let policy = ReconnectOptions::enabled()
//!   .with_initial_delay(Duration::from_millis(250))
//!   .with_max_delay(Duration::from_secs(2))
//!   .with_backoff_factor(2.0);
//!
//! // Or: at most five attempts with a constant 1s delay.
//! let bounded = ReconnectOptions::enabled()
//!   .with_backoff_factor(1.0)
//!   .with_max_attempts(Some(5));
//! ```

use std::time::Duration;

/// Policy controlling automatic reconnection after an unexpected disconnect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReconnectOptions {
  /// Whether to reconnect when the connection drops unexpectedly. A graceful
  /// close requested by the application (or the remote peer) never triggers a
  /// reconnect regardless of this flag.
  pub enabled: bool,
  /// Delay before the first reconnect attempt.
  pub initial_delay: Duration,
  /// Upper bound applied to the (possibly backed-off) delay between attempts.
  pub max_delay: Duration,
  /// Multiplier applied to the delay after each failed attempt. `1.0` keeps the
  /// delay constant; values above `1.0` produce exponential backoff. Values
  /// below `1.0` are clamped to `1.0`.
  pub backoff_factor: f64,
  /// Maximum number of consecutive reconnect attempts. `None` retries forever;
  /// the counter resets once a connection successfully reopens.
  pub max_attempts: Option<u32>,
  /// Maximum time a single attempt may spend reaching the open state before it
  /// is abandoned and retried. This bounds two failure modes the browser never
  /// reports on its own — a URL-provider request or a socket handshake that
  /// stalls indefinitely after the network drops out from under a suspended
  /// mobile tab — so a stuck attempt can no longer wedge the connection forever.
  /// `None` disables the watchdog (the previous behaviour); [`enabled`](Self::enabled)
  /// turns it on at 5s, while the one-shot [`default`](Self::default) leaves it off.
  pub connect_timeout: Option<Duration>,
  /// How long a socket that still *reports* itself open has to answer a liveness
  /// probe after the page comes back from a freeze, before it is treated as dead
  /// and replaced.
  ///
  /// This is the one failure the other bounds cannot see. A tab that was frozen
  /// wakes holding a socket the browser still calls `OPEN` — the TCP connection
  /// underneath it died while the page was not running, and no `close` event
  /// survived to say so. Nothing is stalled, so no watchdog fires; the socket is
  /// simply mute, and stays that way. The probe forces the question and this is
  /// the deadline on the answer. `None` skips the probe entirely — see
  /// [`WebSocketOptions::liveness_probe`](crate::ws::WebSocketOptions::liveness_probe)
  /// for what happens then.
  pub liveness_timeout: Option<Duration>,
}

impl Default for ReconnectOptions {
  /// Disabled, preserving the previous one-shot connection behaviour.
  fn default() -> Self {
    Self {
      enabled: false,
      initial_delay: Duration::from_millis(500),
      max_delay: Duration::from_secs(3),
      backoff_factor: 2.0,
      max_attempts: None,
      // Off for the one-shot default, so a plain socket keeps its previous
      // "stay in Connecting until the browser says otherwise" behaviour.
      connect_timeout: None,
      liveness_timeout: None,
    }
  }
}

impl ReconnectOptions {
  /// Reconnection enabled with sensible defaults: a 500ms initial delay doubling
  /// up to 3s, retrying indefinitely, with a 5s per-attempt connect watchdog and
  /// a 3s deadline on the liveness probe.
  pub fn enabled() -> Self {
    Self {
      enabled: true,
      connect_timeout: Some(Duration::from_secs(5)),
      liveness_timeout: Some(Duration::from_secs(3)),
      ..Self::default()
    }
  }

  /// Set the delay before the first reconnect attempt.
  pub fn with_initial_delay(mut self, delay: Duration) -> Self {
    self.initial_delay = delay;
    self
  }

  /// Set the upper bound on the delay between attempts.
  pub fn with_max_delay(mut self, delay: Duration) -> Self {
    self.max_delay = delay;
    self
  }

  /// Set the backoff multiplier applied after each failed attempt.
  pub fn with_backoff_factor(mut self, factor: f64) -> Self {
    self.backoff_factor = factor;
    self
  }

  /// Set the maximum number of consecutive attempts (`None` for unlimited).
  pub fn with_max_attempts(mut self, attempts: Option<u32>) -> Self {
    self.max_attempts = attempts;
    self
  }

  /// Set the per-attempt watchdog timeout (`None` to disable it). See
  /// [`connect_timeout`](Self::connect_timeout).
  pub fn with_connect_timeout(mut self, timeout: Option<Duration>) -> Self {
    self.connect_timeout = timeout;
    self
  }

  /// Set the deadline on the liveness probe (`None` to skip probing). See
  /// [`liveness_timeout`](Self::liveness_timeout).
  pub fn with_liveness_timeout(mut self, timeout: Option<Duration>) -> Self {
    self.liveness_timeout = timeout;
    self
  }

  /// Whether another attempt is permitted given the number of attempts already
  /// made since the last successful connection.
  pub(crate) fn should_retry(&self, attempts_made: u32) -> bool {
    self.enabled && self.max_attempts.is_none_or(|max| attempts_made < max)
  }

  /// Delay before the retry identified by `retry_index` (0 for the first retry
  /// after a disconnect), with backoff applied and capped at [`Self::max_delay`].
  pub(crate) fn delay_for_attempt(&self, retry_index: u32) -> Duration {
    let base = self.initial_delay.as_secs_f64();
    let factor = self.backoff_factor.max(1.0).powi(retry_index as i32);
    let capped = (base * factor).min(self.max_delay.as_secs_f64());
    // `as_secs_f64` round-trips finite, non-negative values safely here.
    Duration::from_secs_f64(capped.max(0.0))
  }
}
