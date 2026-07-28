# Changelog

All notable changes to `impulse-kit` are documented here. The format loosely
follows [Keep a Changelog](https://keepachangelog.com/); this project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

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
  replay queue), and `prepare_replay` (stamp current credentials on a queued
  request, so a write made weeks ago doesn't replay with a long-rotated token).
  All three have defaults, so existing backends are unaffected.
- **`impulse_client_kit::client::ipc::command`** — the wasm ⇄ Tauri IPC bridge
  for an app's own commands, so a frontend no longer re-declares the `invoke`
  binding to ask the native side something that isn't an HTTP request.

### Changed

- **A failed remote attempt flips the engine offline immediately** instead of
  letting each following request pay for its own timeout until the shell's
  connectivity probe notices.
- `Engine::sync` documents what it already guaranteed: a queued write is only
  dropped when the server *accepted* it, so offline work survives an expired
  session and lands after the next sign-in.

### Fixed

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
