# Changelog

All notable changes to `impulse-kit` are documented here. The format loosely
follows [Keep a Changelog](https://keepachangelog.com/); this project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **`blocks::syntax` — languages as data, and one scanner that reads all of
  them.** A `LangDef` is a language's keywords, its comments, its strings and
  whether it has numbers; `register_lang` adds one, replacing any built-in of
  the same name, and a fence naming it is coloured from then on. Twenty-odd ship
  here (Rust, C/C++, Python, JS/TS, Go, Java, Kotlin, shell, SQL,
  JSON/YAML/TOML/INI, XML/HTML, CSS, Lua, Ruby, PHP, C#, Dockerfile, Typst) —
  chosen by what turns up between fences in an article rather than by any
  attempt at coverage.

  It covers the useful part of what a syntax file in an editor like Kate
  describes, and deliberately not the rest: context stacks, regular expressions
  and per-region embedded rules are what cost such a scanner its speed, and this
  one runs over every visible line on every keystroke.

  A rule picks a `Token`, not a colour. Tailwind emits CSS only for classes it
  can see while scanning the sources, so a class assembled at runtime — by an
  application registering a language of its own — is a class with no CSS behind
  it and no error to say so; and a palette every language shares is what keeps a
  document that switches language mid-page from switching colour scheme with it.

- **`blocks::editor::typx_syntax`**, beside `markdown_syntax`: headings on `=`,
  `#`-code read as code, comments that nest, labels and references, and raw
  blocks coloured as the language they name.

- **`blocks::editor::SourceEditor` takes a `bare` flag** — the border, rounding,
  shadow and focus ring come off, leaving only the text. An editor that owns the
  screen (a document opened into the whole area under a header) has nothing to be
  bounded against, and a frame drawn along the edge of the viewport reads as a
  box that ran out of room rather than as a page. `dark:bg-input/30` goes with
  them: it is the tint that lifts an input off the surface behind it, and a
  full-page editor *is* that surface.

  It is a flag rather than something to override through `class`, because
  [`cn`](impulse_client_kit::utils::cn) concatenates and does not merge — a
  `border-0` passed by the caller lands *beside* the `border` it meant to cancel,
  and which one wins is left to the order the stylesheet happens to emit them in.

- **`components::dnd` — drag-and-drop that works in WebKitGTK (and on touch).**
  `draggable="true"` with `dragstart`/`dragover`/`drop` is the obvious way to
  move something with the mouse and the one thing in the platform that cannot be
  relied on: WebKitGTK, the engine behind every Tauri window on Linux, never
  starts a drag for a plain element, so a Trello-style board built on it is
  simply immovable there. No mobile engine synthesises those events at all, so
  the same code is dead on a phone too.

  `DndProvider` / `Draggable` / `DropZone` rebuild the gesture on pointer
  events, which fire identically for mouse, pen and touch everywhere. Because a
  captured pointer stops firing `pointerover` on anything else, drop targets are
  found by hit-testing `Document::element_from_point` on each move and walking up
  from what it returns — which is also what makes nesting work: the walk yields
  the whole chain, so a row inside a column marks both as hovered
  (`data-dnd-over`) and `DropZone::on_drop` is offered the drop innermost-first,
  answering `false` to pass a kind it doesn't handle out to the zone around it.

  Activation is deliberately delayed: a mouse must travel `DRAG_SLOP` pixels, so
  a click on a button inside a draggable stays a click, and a finger must rest
  for `LONG_PRESS_MS` before it moves, so a swipe down a list still scrolls it.
  Nothing calls `preventDefault` until a drag has actually begun, which is what
  keeps both of those true.

- **`components::stepper::NumberStepper` — a number field with minus/plus
  buttons.** `<input type="number">` has spinners, but they are a few pixels
  tall, absent on touch, and always move by one; a value with a meaningful step
  (a priority that goes ±1, an estimate that goes ±10 minutes) wants buttons of
  its own. The value stays a `String` so a half-typed `-` or an empty box
  survives a keystroke, exactly as with a plain input.

- **`robots.txt` and `sitemap.xml` are built, not kept on disk** — a new
  `impulse_server_kit::seo` module (no feature flag, everything in the prelude).
  `RobotsTxt::new().disallow("/s/").sitemap("/sitemap.xml").into_router()` mounts
  a router at `/robots.txt`; `Sitemap` is a salvo `Writer`, so a list that
  changes is a handler returning one. Both are built rather than served from the
  static directory because both need what a file on disk cannot know: which
  routes are meant to be public, what the application has published since it
  started, and the origin it is being asked on. That last one is why
  `RobotsTxt::sitemap` accepts a rooted path and resolves it per request through
  the new `request_origin`, which reads `X-Forwarded-Proto`/`X-Forwarded-Host`
  before `Host` — behind a TLS-terminating proxy the connection the server
  accepted is plain HTTP, and believing it publishes `http://` URLs for an
  `https://` site, which a crawler files away as a separate, duplicate host.

  `RobotsTxt` and `RobotsGroup` derive `Deserialize`, so a policy can live in the
  application's YAML instead of its code. Rules that cannot mean what they say
  are dropped with a warning when the handler is built rather than rendered: the
  one worth naming is `#`, which starts a comment that runs to the end of its
  line, so `Disallow: /private#draft` bans `/private` and permits everything the
  author believed they had just closed off.

- **`CanonicalOrigin`, for an application a proxy hands more than one hostname.**
  A product domain and a vanity domain pointed at the same socket produce an app
  that serves every page at two addresses, which a crawler reads as two copies of
  one article — it picks a winner itself and splits the signals between them.
  Nothing in a request distinguishes the host you were *asked* on from the host
  you should be *found* under, so this is configuration: `CanonicalOrigin::fixed`
  or `::from_env`, unset meaning "follow the request" (right for one hostname,
  and for a laptop). `resolve(req)` then answers the same origin however the
  request arrived — build the page's `<link rel="canonical">`, the sitemap's
  `<loc>`s and the `Sitemap:` line from it — and `RobotsTxt::canonical_origin`
  emits that `Sitemap:` line only on the canonical host, because a sitemap
  listing another host's URLs is cross-submission and ignored unless both are
  verified. The alias keeps the same crawl rules deliberately: a crawler learns
  two addresses are one page by fetching both and finding the same canonical, and
  `Disallow: /` would leave it unable to see that while still free to list the
  alias bare. It also pins the scheme, which matters on one domain too — a proxy
  that terminates TLS without sending `X-Forwarded-Proto` leaves the server
  honestly reporting `http://`.

- **`request_origin` reads `X-Forwarded-Origin`** — one header carrying
  `scheme://host` whole — ahead of the `X-Forwarded-Proto`/`-Host` pair and
  `Host`. It is the only source that can be *right* for an app reachable on
  several hostnames, because it comes from the side that knows which of them is
  the site's own name; LBRP sends it for a service with
  `provide_origin_as_header`, so an app behind it publishes the right URLs with
  no configuration of its own. Taken whole so the scheme and the host cannot
  arrive from different hops and disagree, and parsed strictly — scheme, host,
  nothing else — because the value goes into every URL the app publishes. A
  `CanonicalOrigin` configured locally still wins.

- **`set_x_robots_tag` and the `RobotsTag` constants**, the other half of keeping
  something out of an index — and not a substitute for the first half.
  `Disallow` is the only one that stops the *fetch*, which is what matters when
  being fetched is itself the damage (a single-use link a crawler spends before
  its recipient opens it); the header is the only one that binds a crawler that
  fetched anyway, having ignored `robots.txt` or been handed the URL directly. It
  also reaches bodies nobody parses as HTML — a JSON read, a PDF, plain Markdown
  — which have nowhere else to carry the rule. The constants exist so a page's
  header and its `<meta name="robots">` can be given the same value instead of
  being spelled out twice and drifting apart.

- **`ButtonSize::None`** — a size that emits no height, no padding and no gap,
  for a button whose call site sizes it itself. Until now such a call site passed
  its geometry in `class` and got it *beside* a size's own, with the stylesheet's
  order picking the winner; a button that looks right until its content outgrows
  the height it was quietly given is the failure that costs the most to find. The
  calendar's day cells and month arrows are the first two users.

- **`<Calendar>` can put something in a day's cell.** The new `day_content` prop
  is a `Callback<NaiveDate, AnyView>` called for every day the grid shows, and
  whatever it returns is drawn under that day's number — a dot for "something
  happens here", a running total, a badge. It renders inside the day's button,
  which was already a `flex-col`, so the whole cell stays one click target and
  the extra content dims along with the number on the neighbouring months' days.
  The `<td>`'s `group/day` + `data-selected` let a colour of your own step aside
  on the selected day with
  `group-data-[selected=true]/day:text-primary-foreground`. This is what turns
  the calendar from a date picker into a month view — an expense planner showing
  each day's spend, a schedule showing how full a day is.

- **`<Calendar>` can be sized for that.** A square built for a bare date picker
  has no room under the number, so `cell_size` (any CSS length, default `2rem` —
  exactly what the calendar used before) and `full_width` (fill the container,
  cells sharing the width, with `cell_size` as their floor) are now props.
  Deliberately props and not classes: `cn` concatenates rather than merges, so a
  `[--cell-size:…]` passed in `class` would sit *beside* the built-in one and
  leave the stylesheet's order to pick a winner. `cell_size` is applied as an
  inline style, which always wins.

- **`<Calendar>` speaks languages other than English.** The month caption used
  to go through `%B` and the weekday headers were a hard-coded `["Mo", "Tu", …]`,
  so the calendar was English wherever it was mounted. The new `labels` prop
  takes a `CalendarLabels { months, weekdays, months_short }` — `months_short`
  being the shorter names the dropdown caption has room for. `Default` is
  exactly the English the calendar rendered before, and both props are optional,
  so no existing call site changes.

  A prop only reaches the calendars an app mounts itself, though, and the one
  inside a `<DateTimePicker>`'s dialog is mounted by the picker: a call site
  could translate every word of that dialog and still get "August 2026" over
  "Mo Tu We", with no way to reach the grid between them. So `labels` also falls
  back to a `CalendarLabels` taken from **context** — provide one at an app's
  root and every calendar under it follows, the pickers' included — and
  `<DateTimePicker>` / `<DatePicker>` now take a `labels` of their own for the
  odd one out, which they hand down the same way. The order is the prop, then
  the context, then English, so nothing that already passes labels changes and
  an app that says it once no longer has to remember which calendars it said it
  to.

- **`<SourceEditor>`: a writing surface for documents a `<textarea>` cannot
  carry.** A textarea is one layout box holding the whole document, and a browser
  re-lays out that box's text on every edit — measured in Chromium, a keystroke
  in the middle of an article costs ~3 ms at 60 KB, ~6 ms at 380 KB and ~22 ms at
  1.2 MB, with frames of 60–190 ms around it. That is the lag that makes a field
  feel heavier the longer the piece gets, and no amount of styling touches it:
  the cost is in laying out text, and a textarea gives nobody a say in how much
  of it gets laid out.

  The new block (`impulse_client_kit_blocks::editor`) keeps the document in Rust
  as a `Vec<String>` and puts only a window of lines around the viewport in the
  DOM — one `<div>` per line inside a `contenteditable`, with the rest standing
  in as padding above and below. **2–3 ms per keystroke at every document size**,
  measured the same way, and a first render of ~2 ms instead of 50–160 ms.

  Editing stays the browser's, which is the point of building it this way:
  caret, selection, IME, autocorrect, mobile keyboards, spell-check and screen
  readers work because the thing being typed into is a real editable element.
  On top of that it has its own undo stack (coalesced by word, since the
  browser's remembers DOM nodes that windowing throws away), `Tab`/`Shift+Tab`
  indentation, plain-text paste, and optional line numbers aligned to
  soft-wrapped lines. `Ctrl+A`, `Ctrl+Home` and `Ctrl+End` materialise the whole
  document and hand over to the browser, so they mean what they always mean.

  Syntax highlighting is a `fn(&str) -> Vec<HighlightSpan>` run over each line as
  it is rendered — colouring only the window, because colouring a 10 000-line
  document as spans costs 170 ms *per keystroke* and colouring the window costs
  3. `markdown_highlighter` ships with it. The line being typed in is re-coloured
  a moment after the typing stops, never during: rewriting a line under a live
  caret is how editors lose keystrokes.

  The scrollbar is honest about a document it has not laid out: the height of an
  unrendered line is fitted from the lines that *have* been measured — weighted
  by their characters, because a line that fits on one row says nothing about
  what a character costs — and every line still on an estimate is re-priced the
  moment that fit moves. Getting this wrong is not cosmetic. An estimate that
  runs high makes the end of the document retreat from the reader: each window
  they scroll into is measured, the page shortens under them, and the last line
  bobs into view and is gone again. Measured on a 4 000-line document, the height
  at first render is now within a percent or so of the truth, and scrolling to
  the end arrives at the end.

  It takes a `value: RwSignal<String>` like a `Textarea` does, so swapping one
  for the other is a line of view code — but **give it a height**
  (`class="h-full"` in a flex column, `class="h-[60vh]"` otherwise): something
  that renders only what fits has to be told what fits.

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

  **The ceiling can also be written in CSS**, and for a field holding a document
  rather than a comment that is the one to use: `class="max-h-[70vh]"` says "as
  tall as the text needs, but never taller than the screen" on a phone and a
  desktop alike, which no fixed number of rows says on both. It reads whatever
  `max-height` ends up in force — `max_rows` alongside it simply means the lower
  of the two wins — and at the ceiling the field scrolls inside itself instead of
  growing. That also keeps a very long document out of the page's own geometry:
  the field stops being a fifty-thousand-pixel box that only the browser's
  scrollbar can make sense of, so the article's scrollbar stays the article's,
  and only the lines on screen are painted.

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

- **`<BackGuard>`: going back, on a phone where that is a system button.** An
  app made of panels, sheets, editors and drill-down pages has a "back" in it
  whether or not anything on screen says so — but inside a Tauri window on
  Android the only back the user has is the system button, and left unhandled it
  does not close the panel, it closes the app. `<BackGuard when=… on_back=… />`
  renders nothing and declares what back means while a layer is open; guards
  stack, so one press closes exactly the innermost. Also available as
  `use_back_guard` for a call site that isn't a view.

  There is no Android API involved, which is the point: an open guard pushes a
  history entry, and Tauri's activity already routes the system button to the
  webview's history before it considers finishing the activity. So a press
  arrives as an ordinary `popstate`, the top guard closes, and pressing it with
  nothing open still closes the app — which is what the button is for. The
  browser's Back and the Escape key land in the same place, so the web gets it
  too. A layer closed by its own controls unwinds its entry as it goes, so a
  later press is never spent on something already closed.

  **`<Dialog>`, `<Sheet>` and `<Drawer>` honour it without being asked**, since
  each already closes on Escape and back is the same dismissal on a device with
  no Escape key to press — a sheet or drawer being precisely what a phone shows.
  `<AlertDialog>` deliberately does not: it dismisses on neither Escape nor its
  overlay, because a confirmation is there to be answered.

- **`use_click_outside`** — closes a panel when a click lands outside it, for a
  menu or popover an app builds out of plain markup rather than out of the kit's
  overlays (which have always done this themselves). Takes the wrapper that holds
  *both* the trigger and the panel, so the click that opens it isn't read as one
  that lands outside; installs one listener for the life of the call site rather
  than one per opening. A panel that only closes through its own controls is a
  trap on a touch screen — there is no Escape to reach for, the panel covers what
  you were aiming at, and tapping beside it does nothing.

- **Signing out can now take the offline copy with it.**
  `Engine::clear_local_data` (and its `WsEngine` twin) empties the app's local
  store through the new `LocalBackend::clear_local` / `WsBackend::clear_local`
  hook — a no-op by default — clears the replay queue (`Queue::clear`,
  `WsQueue::clear`), and on the socket side drops the live connection
  (`WsEngine::drop_socket`) so the old session's broadcasts stop arriving. The
  webview asks for it through `impulse_client_kit::client::clear_local_data`,
  which invokes `ik_clear_local_data` under Tauri and does nothing on the web.

  This closes a gap that no app could close on its own: the mirror is what makes
  an app work without a network, so it deliberately outlives the page — and a
  sign-out only reloads the page. Nothing stored in it says whose data it is, so
  the next person to sign in on the device was served the previous one's data
  while their own was still on its way, and their first offline writes were
  attributed to whoever the identity file still named. Queued writes are the
  sharper end of the same thing: a replay stamps the *current* credentials
  (`prepare_outgoing`), so one person's unsent work would land in another
  person's account. A session merely expiring still keeps everything, as before
  — this is only for a user saying they are done here.

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

- **`SourceEditor`: the text starts where the placeholder says it will, and
  fenced code is no longer read as prose.** What stands in for the lines above
  the window is an inline `padding-top`, which overrules the `py-2` the content
  wears — so the first line sat flush against the top edge, the last against the
  bottom, and the placeholder (an ordinary element, wearing the class) sat a
  couple of pixels below where the text it stands in for would appear. `PAD_Y`
  is now added to the padding at both ends and taken back off wherever a scroll
  position is turned into a line.

  The colouring is block-aware: a line carries a `Block` to the next one, and a
  `Syntax` is the pair of functions that reads it — `paint` for the hundred
  lines on screen, an allocation-free `advance` for every line above them.
  Inside a fence the prose rules are off and the block is coloured as the
  language the fence named, so a `*` in a shell command is no longer the start
  of a bold run and an underscore in a Python name no longer emphasises the rest
  of the paragraph. *Breaking:* `Highlighter` is now that pair — pass
  `markdown_syntax()` where `markdown_highlighter` used to go.

- **`Escape` during an IME composition stays in the editor.** It means "cancel
  what I am typing", and letting it travel on to the window handed it to
  whatever back guard was listening, which closed the document a half-typed
  Cyrillic or Japanese word was being typed into. The platform's own default is
  untouched; only the trip onwards is stopped.

- **An OTP code can be corrected on a phone.** `InputOTP` and
  `InputOTPWithSeparator` were a row of `<input maxlength="1">` boxes that moved
  the focus along per keystroke, and on a phone that field could be filled but
  never emptied: Backspace in an already-empty box deletes nothing, so the
  browser fires no `input` event, and the `keydown` a software keyboard *does*
  send carries `key` = `"Unidentified"` (`keyCode` 229) rather than the key
  pressed — an input method reports its characters only through the `input`
  event that follows. The rule the field was built on ("Backspace in an empty box
  clears the previous one and steps back") therefore asked a question the browser
  refuses to answer there. A mistyped digit was permanent until the page was
  reloaded.

  The boxes are now painted `<div>`s with one transparent `<input>` holding the
  whole code stretched across them. Nothing needs detecting: Backspace deletes
  the character before the caret because there is one, and selection, autorepeat,
  paste and undo are the browser's own. Two things follow for free — the OS's
  SMS-code autofill, which a per-digit field cannot offer at all
  (`autocomplete="one-time-code"`), and no more iOS zooming in on the field when
  it is tapped.

  Both components take an optional `value: RwSignal<String>` for the code, so a
  rejected attempt can be cleared from the outside (`value.set(String::new())`);
  writing to it does not fire `on_change` or `on_complete`. Digits outside ASCII
  are no longer accepted — `char::is_numeric` also matches the digits of other
  scripts, which look like a code and are not the one the server issued. The two
  groups either side of the separator are each rounded on their outer edge now,
  rather than the row as a whole.

- **Leaving a guarded page for somewhere else no longer snaps straight back.**
  Open a document or a board, then pick another tab: the new tab appeared for a
  frame and the app bounced back to where it had been. Pressing Back after that
  left the app entirely instead of returning to the list — on Android, it closed
  the app.

  `<BackGuard>` drove the history directly, pushing an entry as a layer opened
  and calling `history.go(-n)` as one closed. The two halves run on different
  clocks: `pushState` takes effect at once, while `go()` is a queued traversal
  whose delta is resolved against the entry that was current when it was *called*.
  Leaving a document for another tab does both in one tick — the document's guard
  closes, the tab's guard opens — so a `go(-1)` and a `pushState` went out
  together, the traversal landed one entry below the entry just pushed, and the
  `popstate` read a depth shallower than the stack. The guard that had only just
  opened was closed as though the user had asked for it, and the history was left
  an entry short. Which of the two orders a tick happened to take decided whether
  it broke, and engines disagree about where several queued traversals leave you,
  so the same build misbehaved in a Tauri window and looked fine in a browser.

  The stack is now the only truth, and the history is reconciled to it once per
  task: guards add and remove slots, and a single coalesced pass then closes the
  gap with pushes or with one `go()` — never while a traversal of its own is
  still in flight. A tick that closes one layer and opens another nets out to no
  history traffic at all, so there is no longer a race to lose. Escape closes the
  top layer directly for the same reason, rather than being one more caller
  reaching for the same entry. With `day_content`
  drawing anything under the number, the selected (or today's) colour stopped at
  the number and the content sat below it, outside the coloured square — and the
  square itself was the wrong shape, since the cell was trying to be two heights
  at once.

  The day's button asked for the geometry it wanted in `class` — a square, no
  padding, its own column gap — while still carrying a `ButtonSize`, which brings
  `h-8`, `px-3` and a `gap` of its own. `cn` concatenates rather than merges, so
  both landed on the element and the stylesheet's order decided: the fixed `h-8`
  won the height, which pinned the background to two rem and made `aspect-square`
  inert, and the padding and gap fought the same way. Anything the day held past
  those two rem simply hung out of the highlight.

  The cell is now sized in one place. The day's button takes the new
  `ButtonSize::None`, so nothing arrives to argue with, and it stretches to fill
  its `<td>` — which keeps `aspect-square` as a *floor* rather than a size, so a
  day with two lines under its number grows, its row grows with it, and the
  highlight grows with both. `day_content`'s column moved inside a wrapper of its
  own, out of reach of the button's base `gap`.

- **`<Calendar>`'s month arrows are the size they ask for.** Same conflict, same
  cause: `size-[var(--cell-size)] p-0` beside a `ButtonSize`'s `h-8 px-3`. They
  now use `ButtonSize::None` too, so a calendar given a larger `cell_size` gets
  arrows that match its cells instead of arrows stuck at two rem.

- **`<SourceEditor>` now measures the document it is given instead of guessing at
  it.** Every line nobody had scrolled to yet was priced from a fitted average —
  one row, plus what the measured lines said a character costs — and a fitted
  average is right about a document and wrong about each of its lines. The error
  did not stay still, either: every window that got rendered replaced a guess
  with the truth, so the end of the document moved a little further off with each
  screenful read. Reading an article of wrapped paragraphs end to end moved it by
  several percent.

  Which is only a twitching scrollbar until somebody drags it. A browser maps a
  thumb drag against the height it saw when the drag began, so a document that
  grows seven percent under the drag is a drag that runs out of travel seven
  percent short of the end: the thumb is at the bottom of the track, the document
  is not at its end, and the pointer pushes against nothing. Letting go and
  grabbing again worked, because the second drag was measured against a height
  that had by then been corrected.

  So the guess is now only a stand-in for the second or so it takes to replace
  it. The lines are laid out for real, a batch per frame, until every height in
  the document is a measurement — the batch sized to a ten-millisecond budget, so
  a document of headings gets thousands of lines a frame and one of long
  paragraphs gets tens, and neither drops one. It pauses while the reader is
  scrolling or typing and picks up a fifth of a second after they stop. A
  four-thousand-line article is priced within a second of appearing, and from
  then on its height does not move: measured over a full descent of the same
  document, a swing of 949 px became 2 px.

  The lines are laid out **in the editable itself**, appended past the last row
  and taken out again before anything can be painted. Measuring them in a hidden
  twin was the obvious way and it was quietly wrong — a browser does not lay two
  boxes out the same way just because they carry the same classes. Chromium's
  mobile text autosizer boosts the font in a tall block of prose and leaves a
  short hidden one alone, and the twin came back with rows a third the height of
  the real ones: a document mispriced threefold, with nothing on screen to say
  so.

- **An auto-sizing `<Textarea>` no longer throws away the scroll position of the
  page it sits on.** Measuring the text meant giving the field a height of `auto`
  for an instant — the only way to tell the content apart from the box it is
  already in — and in a field grown to hold a long document that instant took
  tens of thousands of pixels out of the page. The browser clamps a scroll offset
  the page no longer reaches, and gives nothing back when the height returns a
  microsecond later, so the reader was thrown to the top of their own article by
  their own keystroke, and only got back when the browser chased the caret again.
  Every scroll offset above the field is now snapshotted and put back in the same
  turn, before anything can be painted or scrolled from the collapsed page.

  Growing — which is what typing does — no longer collapses the field at all:
  a box already shorter than its text reports the text's full height as
  `scroll_height`, so there is nothing to find out by shrinking it first.

  **A field is also re-measured when it takes focus.** A height measured before
  the webfont arrived is a height a line or two short of its text, nothing in the
  value or the width ever says so, and a field that is short of its text scrolls
  inside itself the moment a caret is put in it — the text lurching under the
  very click that placed the caret.

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
