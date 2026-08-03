# Changelog

All notable changes to `impulse-kit` are documented here. The format loosely
follows [Keep a Changelog](https://keepachangelog.com/); this project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **`<Textarea>` can let the text set its height.** The `resizable` prop is now
  a `TextareaSizing` — `Grabber` (the pill handle, still the default), `Fixed`
  (no handle at all), or the new `Auto`, where the field grows as lines are
  added, shrinks as they go, and never scrolls inside itself. A comment box that
  starts as one line and opens up as it is written is `rows=1
  resizable=TextareaSizing::Auto`; add `max_rows=10` to stop the growth
  somewhere and hand the rest back to a scrollbar, or leave it out and let the
  page do the scrolling. `rows` and any `min-h-…` stay meaningful as the floor.
  The height is re-measured after every change to the value — a `value.set` from
  elsewhere resizes the field exactly as typing does — and whenever the field's
  width changes, since the same text wraps into more lines in a narrower box.
  `resizable=true` / `resizable=false` still compile and still mean `Grabber` /
  `Fixed`, which is every call the old `Option<bool>` prop ever accepted — so no
  existing call site has to change.

- **The Tauri engine reports a no-connection state.** Every response
  `impulse-tauri-engine` produces without reaching the server — a locally-served
  success as much as a "not available offline" error — now carries the
  `x-ik-offline` header (`impulse_endpoint::OFFLINE_HEADER`,
  `HttpResponse::is_offline`). A frontend can finally tell "the server rejected
  this" from "the server was never asked", which is what an auth gate needs to
  keep an offline-first app usable instead of bouncing the user to a login
  screen no network can complete.
- **`LocalBackend` gained three hooks** for the same reason:
  `observe_write` (see successful online *writes*, e.g. an auth check answering
  `POST` with the signed-in identity — `cache_read` only ever sees reads),
  `should_queue` (keep verb-writes that change nothing on the server out of the
  replay queue), and `prepare_outgoing` (stamp current credentials on a request
  the engine sends itself, so a write made weeks ago doesn't replay with a
  long-rotated token). All have defaults, so existing backends are unaffected.
- **`Engine::prefetch` + `LocalBackend::prefetch_requests`.** Caching only what
  the user happened to open leaves an app half-usable offline — you find out
  which data you *didn't* read at exactly the wrong moment. A backend can now
  name the requests worth running purely to fill the local store; the engine runs
  them while online, authenticating each through `prepare_outgoing` (they never
  passed through the UI, so nothing else would).
- **Date and time pickers that are actually nullable.** `DatePicker` gained a
  working implementation and two siblings, `TimePicker` and `DateTimePicker`,
  all built out of `Dialog`, `Calendar` and custom hour/minute steppers — no
  `<input type="date">` / `<input type="time">` anywhere. The native widgets are
  a different control on every platform, and on Linux webviews they pre-fill
  themselves: a field the user never touched still handed back a date, so "no
  deadline" could not be expressed at all. The value is `Option<NaiveDateTime>`,
  starts `None` unless `default_value` says otherwise, and an ✕ next to the
  field (or *Clear* in the dialog) puts it back. Wording — title, placeholder,
  button labels, `strftime` format — is settable, and `Calendar` took an
  optional `month` signal so the dialog opens on the month already chosen.
- **`ThemeToggle` can label itself.** Six new optional props — `system_icon`,
  `light_icon`, `dark_icon` and `system_text`, `light_text`, `dark_text` — make
  the button render the face of the mode it is *currently in*, icon first and
  text after. A three-state control that never says which state it is in leaves
  the user to click and find out; each face is optional, and with none of them
  set the toggle renders its children exactly as before. The three icons now
  **default to a built-in sun, moon and monitor**, so `<ThemeToggle/>` on its own
  is a finished icon button — every app that had one was re-drawing the same
  glyphs (or pulling in an icon set to name them) to get there. An icon prop
  overrides one mode and leaves the rest built-in; `light_icon=|| ()` drops a
  mode's icon entirely, and children still replace the label wholesale.
- **`impulse_client_kit::client::ipc::command`** — the wasm ⇄ Tauri IPC bridge
  for an app's own commands, so a frontend no longer re-declares the `invoke`
  binding to ask the native side something that isn't an HTTP request.

### Changed

- **Selects are button-sized.** `SelectTriggerSize` now mirrors `ButtonSize` one
  for one — `Sm` (`h-8`, the default), `Middle` (`h-9`), `Lg` (`h-10`) — so a
  select in a row of buttons no longer stands a step taller than everything
  around it. `NativeSelect` takes the same `size` prop and follows the same
  scale. *Breaking:* `SelectTriggerSize::Default` is gone; it was `h-9`, so pass
  `SelectTriggerSize::Middle` to keep the old height, or drop the prop to get the
  button-matching one.
- **The engine reports where a request's time went** — network versus local store
  — at debug level, and as a `slow request: …` warning past 750 ms. A local
  backend is written while the UI waits for the response, so a slow store is
  indistinguishable from a slow server from the user's seat unless the two are
  named separately.
- **A failed remote attempt flips the engine offline immediately** instead of
  letting each following request pay for its own timeout until the shell's
  connectivity probe notices.
- `Engine::sync` documents what it already guaranteed: a queued write is only
  dropped when the server *accepted* it, so offline work survives an expired
  session and lands after the next sign-in.

### Fixed

- **A socket restored from a frozen page is now made to prove it is alive.** The
  browser handle's page-lifecycle recovery only reconnected a socket that already
  looked dead, and after a freeze a dead socket does not look dead: the
  connection died while the page was not running, so no `close` event was
  delivered and `readyState` stays `OPEN`. Sending into it does not fail either.
  For an app whose protocol only speaks when there is news, nothing ever
  contradicts the illusion — the tab shows itself connected and receives nothing,
  for as long as it is left open.

  On a wake the handle now sends `WebSocketOptions::liveness_probe` — a frame the
  app knows the server answers — and gives it
  `ReconnectOptions::liveness_timeout` to produce anything at all; silence
  replaces the socket. An app opts in by naming the frame (its "give me a
  snapshot" message is usually the right one). Configure no probe and a wake
  reconnects unconditionally instead: costlier, but never silently mute.

  **`WebTransport` had the identical hole and gets the identical fix.** Its
  supervisor woke only on a session that was no longer `Open`/`Connecting`, and a
  session restored from a freeze reports `Open` with its `closed` promise pending
  forever — so the wake never fired on exactly the sessions that needed it.
  `WebTransportHandle::set_liveness_probe` registers a probe datagram, and the
  answer is observed through the datagram reader, so it works alongside a
  registered `datagram_signal`; without a probe, a reader, or a deadline the
  session is rebuilt on wake rather than trusted.

- **Nothing in a reconnect now takes longer than about five seconds.** The old
  numbers were sized for a desktop that reconnects once a day, not a phone whose
  connection dies every time its owner checks a message; a 20-second backoff or a
  30-second idle timeout is time spent showing someone a screen that has quietly
  stopped being true. Across both kits: the browser policy's connect watchdog
  15s → 5s, initial delay 1s → 500ms, max delay 30s → 3s; the Tauri engine's
  connect timeout 15s → 5s, write timeout 10s → 3s, initial delay 1s → 500ms,
  max delay 20s → 3s, id-reconciliation wait 5s → 3s; the socket's ping 10s → 1s
  and idle timeout 30s → 3s (still three pings, so a single lost one is still not
  enough to trip it); the shared HTTP client's connect timeout 6s → 3s, read
  timeout 30s → 4s and pool idle timeout 20s → 5s. A reconnect is a ticket and a
  handshake; discovering too late that you needed one is the expensive outcome.

- **A Tauri app's socket comes back after the phone does.** On Android an app
  that had been in the background could return to a permanently disconnected
  socket that never retried — despite keepalive pings and an idle timeout, which
  is what made it so hard to place. The stall was *before* the socket: the
  process comes back holding pooled HTTP connections whose peer is gone, and a
  request over one of those never answers and never fails. The reconnect loop,
  parked on the ws-token fetch it makes per attempt, simply stopped — nothing to
  see in a log, and no keepalive to save a connection that was never opened.

  Every wait on that path is now bounded, in the library rather than in each
  shell:

  - the shared `executor::client` gained a `read_timeout` (30s) and dropped its
    pool idle timeout from 300s to 20s, with TCP keepalive on — a stale pooled
    connection is now closed rather than handed to the next request. This
    applies to *everything* native an app sends through it, including offline
    upload queues draining;
  - `WsEngine::connect_and_run` bounds the whole connect attempt
    (`ReconnectPolicy::connect_timeout`, 15s) — ticket fetch and handshake
    alike — and bounds every socket write (`write_timeout`, 10s). A wedged write
    used to hold the sink and take the *next* connect attempt down with it;
  - `shell`'s socket bounds its own writes too, so a stalled keepalive ping
    can't freeze the write half it shares with the app's frames;
  - the socket is now dropped when a connection ends, instead of being left for
    its keepalive task to ping and for the next attempt to wait on.

- **`WsEngine::run_reconnecting` is the reconnect loop, so apps stop writing
  one.** It connects, serves until the connection drops, backs off and dials
  again, forever; `run_reconnecting_with` adds a hook for a shell with its own
  queue to drain each cycle. It needs no connectivity pre-check — a bounded
  connect attempt *is* the probe — and a resume collapses the backoff, so coming
  back to the foreground reconnects at once rather than serving out a wait
  measured against a network the app may no longer be on. The loop each app kept
  its own copy of is where this bug lived; there is now one copy, with tests for
  a connect that never answers and a write that never completes.

- **The resume signal moved to `impulse_tauri_engine::lifecycle`.** It was
  private to the socket, so only the read side could hear it — a resume that
  landed while a connect attempt or a backoff was in flight did nothing. Those
  now listen too. `shell::wake` still exists and is still what a shell calls
  from `WindowEvent::Focused(true)`.

- **The socket's keepalive is easier on a phone.** `PING_INTERVAL` 3s → 10s and
  `IDLE_TIMEOUT` 5s → 30s (three pings, not one-and-a-bit). A five-second stall
  is ordinary on a mobile network, and treating it as a dead connection cost a
  full reconnect — ticket, TLS, handshake, and whatever snapshot the server
  rebuilds — several times an hour. The case the short timeout was really there
  for, a resume, is handled at once by `wake` instead.

- **`<Textarea>` no longer relies on the platform's resize grip.** On Android the
  native grip is a barely-visible white speck in the corner; every other
  platform draws its own. The native one is now off (`resize-none`) and the
  field carries a pill-shaped grabber centred underneath it (`mt-2` below the
  field) that looks the same everywhere — dragged with mouse, touch or pen, or
  moved with ↑/↓ when focused. `resizable=false` drops it for a fixed-height
  field.
- **`<Textarea class="min-h-…">` was ignored.** The base styling carried
  `min-h-[80px]`, and since `cn` only concatenates, both classes reached the
  same layer — where Tailwind orders candidates of one utility by name, not by
  the order they appear in the attribute. `min-h-[80px]` therefore came out
  after `min-h-[28rem]` and won, and a field asked to be an editor pane stayed
  four rows tall. The base no longer sets a `min-height` at all: the opening
  height is `rows` (still four by default), so a caller's `min-h-…` / `h-…` is
  the only thing setting it. Fields that never passed one look exactly as
  before.
- **`<Markdown>` padded itself out of every tight box it was put in.** A
  document's block rhythm was set a step too generously (`my-4` paragraphs,
  `mt-8` headings, `leading-7` lines), and — worse — the first block's top and
  the last block's bottom margins were rendered too, so a two-line description in
  a task card floated in the middle of it. Spacing is now one step tighter and
  the leading/trailing margins are trimmed. `MarkdownClasses::compact()` is a
  ready-made preview density for cards, list rows and tooltips.

- **SK DSL type parser silently ignored trailing input.** `TypeParser::parse`
  did not require the whole input to be consumed, so malformed types such as
  `HashMap String>` parsed as the bare `HashMap` instead of erroring. It now
  rejects trailing characters. (Surfaced by wiring the test suites into CI —
  the failing `test_error_cases` had never run in the pipeline before.)

### Security / robustness

- **`immediate-abort` is now WASM-only.** The shared `[profile.release]` applied
  `panic = "immediate-abort"` to *native* binaries too, turning every reachable
  panic into an `abort()` of the whole process — a remote-DoS surface for a
  long-running server (`iks`, `ring-server`, any Server Kit backend). Native
  release builds now unwind; the aggressive strategy moved to a dedicated
  `wasm-release` profile (inherits `release`) used by the wasm build tooling.
  See the README "Build profiles" section.

### CI / tooling

- The `quality` pipeline now **runs the native test suites** (`cargo test` for
  `impulse-utils`, `impulse-server-kit`, `impulse-server-kit-dsl`,
  `impulse-static-server`), a **format check** (`cargo fmt --check`) and a
  **dependency audit** (`cargo deny check`, config in `deny.toml`) in addition
  to the existing clippy matrix and doc tests.

### Docs

- Added a "Build profiles" section to the README, `SECURITY.md`, and this
  `CHANGELOG.md`.
