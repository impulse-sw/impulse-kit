//! A plain-text source editor that keeps only the visible part of the document
//! in the DOM.
//!
//! # Why not a `<textarea>`
//!
//! A `<textarea>` is one layout box holding a whole document, and a browser
//! re-lays out that box's text on every edit. Measured in Chromium, a keystroke
//! in the middle of an article costs about 3 ms at 60 KB, 6 ms at 380 KB and
//! 22 ms at 1.2 MB — with frames of 60–190 ms around the edit, which is the lag
//! that makes a field feel heavier the longer the piece gets. Nothing about the
//! field's *height* changes it: a 40-pixel textarea holding the same text costs
//! the same as one grown to fifty thousand pixels, because the cost is in laying
//! out text, not in painting a box. There is no way to ask a textarea to lay out
//! only what is on screen.
//!
//! # What this does instead
//!
//! The document lives in Rust as a `Vec<String>`, and only a window of lines
//! around the viewport is ever put in the DOM — one `<div>` per line inside a
//! `contenteditable`, with the rest of the document standing in as padding above
//! and below. Editing is the browser's: caret, selection, IME, autocorrect,
//! mobile keyboards, spell-check and screen readers all work, because the thing
//! being typed into is a real editable element. After each edit the window is
//! read back into the model, and that is the whole of the bookkeeping.
//!
//! The same measurement with an 80-line window: **2–3 ms per keystroke at every
//! document size**, and a first render of about 2 ms instead of 50–160 ms. The
//! cost stops depending on the length of the piece, which is the entire point.
//!
//! Highlighting is per rendered line, so it costs nothing on the lines nobody is
//! looking at — colouring a 10 000-line document as spans costs 170 ms *per
//! keystroke*, colouring the window costs 3.
//!
//! # What it is not
//!
//! Not a code IDE: no multi-cursor, no bracket matching, no folding, no
//! find-and-replace, and the highlighter sees one line at a time (a Markdown
//! fence marks its own line; it does not colour everything between two of them).
//!
//! A selection can only span what the DOM holds — so while one is being dragged
//! the window grows with it rather than moving, and shrinks back to the viewport
//! when the mouse comes up. The whole-document keys people actually use —
//! `Ctrl+A`, `Ctrl+Home`, `Ctrl+End` and their `Shift` forms — put the whole
//! document in the DOM first and hand over to the browser, so they work at any
//! size, at the cost of one hitch of a few tens of milliseconds on a very long
//! document. The window returns on the next scroll or edit.

use impulse_client_kit::utils::cn;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, Node, Range, Selection};

/// Classes on the scroll box the editor lives in. It takes its height from the
/// caller (`class="h-full"` in a flex column, `class="h-[60vh]"` otherwise):
/// unlike a textarea, something that renders a window has to be told how big the
/// window is.
const ROOT_CLASSES: &str = "relative w-full overflow-auto rounded-md border border-input bg-transparent text-base shadow-xs transition-[color,box-shadow] outline-none dark:bg-input/30 focus-within:border-ring focus-within:ring-ring/50 focus-within:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive md:text-sm";

/// Classes on the editable itself. `overflow-anchor: none` keeps the browser's
/// own scroll anchoring from fighting the padding that stands in for the lines
/// off screen — it would otherwise "correct" the scroll every time that padding
/// changed, which is every time the window moves.
const CONTENT_CLASSES: &str = "min-h-full whitespace-pre-wrap break-words outline-none [overflow-anchor:none]";

/// How much beyond the viewport is kept in the DOM, in viewports.
const OVERSCAN: f64 = 1.0;

/// ...and never fewer than this many lines on either side. A viewport is a poor
/// unit for a margin when the lines are tall: on a phone a wrapped paragraph can
/// be a third of the screen, so one viewport of margin comes to three lines, and
/// a flick clears it long before the next frame is drawn. On a screen that
/// already shows more lines than this, the viewport is the wider of the two and
/// nothing changes.
const MIN_OVERSCAN_LINES: usize = 8;

/// How much further the window reaches in the direction the reader is going. A
/// fling travels much further than a frame of scrolling does, and it travels one
/// way — so the margin is worth spending where the reader is heading rather than
/// evenly around them.
const LEAD: f64 = 2.0;
const TRAIL: f64 = 0.5;

/// How long a run of single-character edits keeps folding into one undo entry.
const UNDO_COALESCE_MS: f64 = 700.0;

/// How much of the fit survives each measured line. Low enough to follow a
/// document that changes shape halfway through, high enough that one odd
/// paragraph does not reprice the rest of it.
const FIT_DECAY: f64 = 0.995;

/// What the font probe is worth in the fit, in characters — about a line.
const PROBE_WEIGHT: f64 = 120.0;

/// How long pricing may spend laying lines out in any one frame.
///
/// A budget rather than a line count, because what a line costs to lay out is
/// the one thing this cannot know in advance: a heading is free and a paragraph
/// that wraps into twenty rows is not, and a document is usually made of both.
/// Fixed at three hundred lines a frame this took sixty milliseconds a frame on
/// an article and half a millisecond on a changelog.
const PRICE_BUDGET_MS: f64 = 10.0;

/// How many lines pricing tries on its first frame, before it has any idea what
/// they cost.
const PRICE_BATCH: usize = 64;

/// How long after the last scroll the window is drawn back in to the viewport.
const SETTLE_AFTER: std::time::Duration = std::time::Duration::from_millis(220);

/// How long after the last keystroke the caret's line is re-coloured.
const RELIGHT_AFTER: std::time::Duration = std::time::Duration::from_millis(250);

/// A run of one line the highlighter wants coloured: byte offsets into that
/// line, and the classes to colour it with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighlightSpan {
  /// Byte offset of the first byte, into the line the span came from.
  pub from: usize,
  /// Byte offset one past the last byte.
  pub to: usize,
  /// Tailwind classes for the run.
  pub class: &'static str,
}

impl HighlightSpan {
  /// A span over `from..to`, coloured with `class`.
  pub fn new(from: usize, to: usize, class: &'static str) -> Self {
    Self { from, to, class }
  }
}

/// Turns one line into the runs to colour in it.
///
/// It sees a single line and nothing else, which is what makes it affordable: it
/// runs on the lines in the window and on no others. Spans must be sorted, must
/// not overlap and must fall on character boundaries — anything else is dropped
/// rather than trusted.
pub type Highlighter = fn(&str) -> Vec<HighlightSpan>;

/// Where the caret is, in the document's terms: a line, and a byte offset into
/// that line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Pos {
  line: usize,
  col: usize,
}

/// One undoable change: the lines `at..at + removed.len()` became `inserted`.
struct Edit {
  at: usize,
  removed: Vec<String>,
  inserted: Vec<String>,
  before: Pos,
  after: Pos,
}

/// Everything the editor knows that is not in the DOM.
struct State {
  /// The document. Never empty — an empty document is one empty line.
  lines: Vec<String>,
  /// What each line is worth in pixels: measured once it has been rendered,
  /// estimated from its length until then.
  heights: Vec<f64>,
  measured: Vec<bool>,
  /// The height of one unwrapped row, measured from the real font.
  row_h: f64,
  /// The fit that prices a line nobody has rendered: how many characters have
  /// been measured, and how many pixels of *extra* height beyond the first row
  /// those characters came to. Their ratio is the cost of a character, carrying
  /// the real font, the real width and the way real words wrap.
  ///
  /// Weighted by characters rather than by lines, and that is the whole trick: a
  /// line that fits on one row says nothing about what a character costs — only
  /// that this many of them still fit — so a screenful of short lines must not
  /// be allowed to talk a document of long ones down to one row each.
  fit_chars: f64,
  fit_extra: f64,
  /// The cost of a character the current estimates were computed with. When the
  /// fit moves away from it, the estimates are worth redoing.
  applied_slope: f64,
  /// How far down the document pricing has got. Every line above it has been
  /// laid out for real, so its height is a fact rather than a guess.
  priced: usize,
  /// A pricing frame is already booked, so nothing books a second one.
  pricing: bool,
  /// How many lines pricing took last frame, retuned each time to whatever fits
  /// in [`PRICE_BUDGET_MS`] of this document, on this machine.
  price_batch: usize,
  /// The half-open range of lines currently in the DOM.
  first: usize,
  last: usize,
  /// An IME is composing: until it says otherwise, the DOM is the browser's and
  /// touching it destroys the composition.
  composing: bool,
  /// The range of lines the DOM actually holds. Kept apart from the window
  /// itself so a render can tell what is already there from what it has to make.
  painted: (usize, usize),
  /// Where the scroll was at the last render, so the next one can tell which way
  /// the reader is going.
  last_scroll: f64,
  /// When the reader last actually moved. Re-pricing changes the height of the
  /// document, and the height of the document is what a scrollbar drag is
  /// measured against — so it waits until they have stopped.
  last_moved_ms: f64,
  /// Which way they were last actually going. A render that happens while the
  /// scroll is still — the one that runs a fifth of a second after they stop —
  /// sees no movement at all, and "no movement" must not be read as "not on
  /// their way up": that is how a reader creeping up off the last line gets
  /// posted back down to it.
  heading: f64,
  /// Where the end of the document was at the last look, so a render can tell
  /// whether it has moved — and by how much — since.
  last_max: f64,
  /// The last string this editor put into `value`, so its own writes can be told
  /// from somebody else setting the document.
  emitted: String,
  undo: Vec<Edit>,
  redo: Vec<Edit>,
  /// When the top of the undo stack was last added to.
  last_edit_ms: f64,
  /// `Escape` was the last key pressed, so the next `Tab` moves focus instead of
  /// typing indentation — the way out for anyone driving this from a keyboard.
  tab_escapes: bool,
}

impl Default for State {
  fn default() -> Self {
    Self {
      lines: vec![String::new()],
      heights: vec![20.0],
      measured: vec![false],
      row_h: 20.0,
      fit_chars: 0.0,
      fit_extra: 0.0,
      applied_slope: f64::NAN,
      priced: 0,
      pricing: false,
      price_batch: PRICE_BATCH,
      first: 0,
      last: 0,
      composing: false,
      painted: (0, 0),
      last_scroll: 0.0,
      last_moved_ms: 0.0,
      heading: 0.0,
      last_max: 0.0,
      emitted: String::new(),
      undo: Vec::new(),
      redo: Vec::new(),
      last_edit_ms: 0.0,
      tab_escapes: false,
    }
  }
}

impl State {
  /// Where a line's top edge sits, in pixels from the top of the document.
  fn offset_of(&self, line: usize) -> f64 {
    self.heights[..line.min(self.heights.len())].iter().sum()
  }

  /// The height of the whole document, measured lines and estimates alike.
  fn total(&self) -> f64 {
    self.heights.iter().sum()
  }

  /// The line a pixel offset falls in.
  fn line_at(&self, offset: f64) -> usize {
    let mut acc = 0.0;
    for (i, height) in self.heights.iter().enumerate() {
      acc += height;
      if acc > offset {
        return i;
      }
    }
    self.lines.len().saturating_sub(1)
  }

  /// What a line is worth before anybody has laid it out: one row, plus what the
  /// measured lines say each character beyond that costs.
  ///
  /// Deliberately a straight line rather than `rows = chars / width`, which is
  /// the obvious formula and is systematically wrong: it steps to two rows the
  /// moment a line is one character past the width, and real text wraps on word
  /// boundaries at a width no character count knows. On a document of 4 000
  /// ordinary paragraphs that guess came out **twice** the real height — and an
  /// over-estimate is not a cosmetic problem, it is the bottom of the document
  /// running away from the reader, because every window that gets measured
  /// shortens the page under them. A fitted average is wrong in both directions
  /// instead of one, so the errors cancel over a document instead of piling up.
  fn estimate(&self, text: &str) -> f64 {
    self.row_h + self.slope() * text.chars().count() as f64
  }

  /// What one character of a line costs beyond its first row.
  fn slope(&self) -> f64 {
    if self.fit_chars < 1.0 {
      return 0.0;
    }
    (self.fit_extra / self.fit_chars).max(0.0)
  }

  /// Folds one measured line into the fit.
  ///
  /// Decaying sums rather than exact totals: no bookkeeping is needed when lines
  /// are edited, re-measured or replaced, the fit follows a document whose
  /// paragraphs change shape halfway through, and one window's worth of real
  /// lines outweighs the opening guess.
  fn observe(&mut self, text: &str, height: f64) {
    let chars = text.chars().count() as f64;
    if chars < 1.0 || height <= 0.0 {
      return;
    }
    self.fit_chars = self.fit_chars * FIT_DECAY + chars;
    self.fit_extra = self.fit_extra * FIT_DECAY + (height - self.row_h).max(0.0);
  }

  /// Whether the reader is in the middle of a gesture — scrolling now, or having
  /// scrolled a moment ago. Nothing that changes the document's height or the
  /// scroll position may happen while this is true.
  ///
  /// `last_moved_ms > 0` matters: it is zero before they have ever moved, and at
  /// that point the page is young enough that a plain "was it less than a fifth
  /// of a second ago" reads as yes and holds up the first re-pricing of all.
  fn is_moving(&self) -> bool {
    self.last_moved_ms > 0.0 && now_ms() - self.last_moved_ms <= SETTLE_AFTER.as_millis() as f64
  }

  /// Whether the reader is doing anything at all — scrolling, or typing. Work
  /// that can wait waits for this: laying lines out invalidates the layout of
  /// the page they are on, and a keystroke that has to pay for a re-layout is a
  /// keystroke they can feel.
  fn is_busy(&self) -> bool {
    self.is_moving()
      || (self.last_edit_ms > 0.0
        && now_ms() - self.last_edit_ms <= SETTLE_AFTER.as_millis() as f64)
  }

  /// Whether the fit has moved far enough from the estimates on the books to be
  /// worth redoing them.
  ///
  /// The bar is deliberately low. Re-pricing four thousand lines is a few
  /// hundred microseconds of arithmetic, and it only happens when the fit
  /// actually moves — while a fit left a tenth of a percent stale is a document
  /// whose end sits in the wrong place, which is the one thing a reader notices.
  fn estimates_are_stale(&self) -> bool {
    !self.applied_slope.is_finite()
      || (self.slope() - self.applied_slope).abs() > self.applied_slope.abs() * 0.01 + 0.0002
  }

  /// Swallows `drift` pixels into the guesses above the window.
  ///
  /// Rows are rendered above the viewport as well as below it, and measuring one
  /// that was until now a guess moves every line under it — which moves the
  /// reader. The obvious repair is to move the scroll position back by as much;
  /// the trouble is that writing the scroll position on Android cancels the
  /// gesture the platform is animating, so the repair is felt as a jolt, or as a
  /// slow drag that will not move at all.
  ///
  /// So the correction goes where it costs nothing instead: the lines above the
  /// window have never been laid out and their heights are guesses, and a guess
  /// is exactly the thing that can absorb a few pixels. The region above keeps
  /// the total it had, the reader keeps the line they were reading, and nobody
  /// writes a scroll position. `false` if the guesses could not swallow all of
  /// it — near the top of a document there may be nothing above to adjust.
  fn absorb_above(&mut self, drift: f64) -> bool {
    let mut left = drift;
    for i in (0..self.first.min(self.heights.len())).rev() {
      if left.abs() < 0.5 {
        return true;
      }
      if self.measured[i] {
        continue;
      }
      // A guess may be nudged, not turned into a nonsense: one row is the floor
      // and a screenful of rows the ceiling.
      let was = self.heights[i];
      let want = (was - left).clamp(self.row_h, self.row_h * 40.0);
      self.heights[i] = want;
      left -= was - want;
    }
    left.abs() < 0.5
  }

  /// Re-prices the unmeasured lines from `from` onwards, on the current fit.
  ///
  /// **From `from` onwards, and never above it.** Everything above the window is
  /// ground the reader is standing on: re-pricing a line up there moves every
  /// line below it, which moves the reader, and the only way to put them back is
  /// to write the scroll position — mid-gesture, against a fling the platform is
  /// animating. That is what "it keeps throwing me back" is. Below the window
  /// nothing has been painted yet, so re-pricing there costs a scrollbar that
  /// twitches and nothing else.
  fn reprice(&mut self, from: usize) {
    self.applied_slope = self.slope();
    for i in from..self.lines.len() {
      if !self.measured[i] {
        self.heights[i] = self.estimate(&self.lines[i]);
      }
    }
  }

  /// Re-derives the document from `text`, keeping nothing.
  fn load(&mut self, text: &str) {
    self.lines = text.split('\n').map(str::to_string).collect();
    if self.lines.is_empty() {
      self.lines.push(String::new());
    }
    self.heights = self.lines.iter().map(|line| self.estimate(line)).collect();
    self.measured = vec![false; self.lines.len()];
    self.priced = 0;
    self.painted = (0, 0);
    self.undo.clear();
    self.redo.clear();
    self.first = 0;
    self.last = 0;
  }

  /// The document as one string — what the outside world calls the value.
  fn text(&self) -> String {
    self.lines.join("\n")
  }

  /// Whether the document is one empty line, which is the only thing worth
  /// telling the view about.
  fn is_blank(&self) -> bool {
    self.lines.len() == 1 && self.lines[0].is_empty()
  }
}

/// Everything an operation needs to reach: all of it `Copy`, so handlers can
/// take it by value instead of each closure capturing five things.
#[derive(Clone, Copy)]
struct Ctx {
  state: StoredValue<State>,
  root: NodeRef<leptos::html::Div>,
  content: NodeRef<leptos::html::Div>,
  gutter: NodeRef<leptos::html::Div>,
  highlight: Option<Highlighter>,
  value: RwSignal<String>,
  blank: RwSignal<bool>,
  /// The pending "colour the line being typed in" timer, if any.
  relight: StoredValue<Option<TimeoutHandle>>,
  /// The pending "the scrolling has stopped" timer, if any.
  settle: StoredValue<Option<TimeoutHandle>>,
}

impl Ctx {
  /// The scroll box and the editable, once they exist.
  fn elements(&self) -> Option<(Element, Element)> {
    let root = self.root.get_untracked()?;
    let content = self.content.get_untracked()?;
    Some((root.unchecked_into(), content.unchecked_into()))
  }

  fn gutter_element(&self) -> Option<Element> {
    self.gutter.get_untracked().map(JsCast::unchecked_into)
  }
}

/// A source editor for documents too long for a `<textarea>`.
///
/// * `value` — the document, read and written as one string. Set it from outside
///   and the editor reloads; type in it and it is rewritten on every keystroke,
///   exactly like a `Textarea`.
/// * `class` — goes on the scroll box. **Give it a height** (`h-full` inside a
///   flex column, or `h-[60vh]`): something that renders only what fits has to be
///   told what fits.
/// * `highlight` — a [`Highlighter`] run over each line as it is rendered;
///   [`markdown_highlighter`] is one. Left out, the text stays plain.
/// * `line_numbers` — a gutter down the left, aligned to soft-wrapped lines.
/// * `tab_size` — how many spaces `Tab` inserts (two by default). `Tab` types
///   indentation here rather than moving focus, as it does in every editor;
///   `Shift+Tab` takes it back out.
///
/// ```ignore
/// let content = RwSignal::new(String::new());
/// view! {
///   <SourceEditor value=content class="h-full font-mono" highlight=markdown_highlighter />
/// }
/// ```
#[component]
pub fn SourceEditor(
  #[prop(optional)] value: RwSignal<String>,
  #[prop(into, optional)] class: String,
  #[prop(into, optional)] placeholder: String,
  #[prop(optional)] disabled: bool,
  #[prop(optional)] readonly: bool,
  #[prop(optional)] line_numbers: bool,
  #[prop(optional)] spellcheck: bool,
  #[prop(optional)] tab_size: Option<usize>,
  #[prop(optional)] highlight: Option<Highlighter>,
) -> impl IntoView {
  let ctx = Ctx {
    state: StoredValue::new(State::default()),
    root: NodeRef::new(),
    content: NodeRef::new(),
    gutter: NodeRef::new(),
    highlight,
    value,
    blank: RwSignal::new(true),
    relight: StoredValue::new(None),
    settle: StoredValue::new(None),
  };
  let editable = !disabled && !readonly;
  let indent = StoredValue::new(" ".repeat(tab_size.unwrap_or(2).clamp(1, 8)));
  let has_placeholder = !placeholder.is_empty();

  // Mount, and anybody setting the document from outside. Both are the same
  // thing — a value this editor did not write — so both take the same path.
  Effect::new(move |_| {
    let incoming = ctx.value.get();
    // Tracked on purpose: the refs are empty until the view is mounted, and this
    // has to run again when they land.
    let (Some(root), Some(content)) = (ctx.root.get(), ctx.content.get()) else {
      return;
    };
    let (root, content): (Element, Element) = (root.unchecked_into(), content.unchecked_into());
    if ctx.state.with_value(|st| st.emitted == incoming && st.last > st.first) {
      return;
    }
    // Where `plaintext-only` is not supported the attribute reads back as
    // something else; `true` plus the paste handler is the fallback, and the
    // difference between them is markup nobody asked for.
    if editable && content.get_attribute("contenteditable").as_deref() != Some("true") {
      let supported = content
        .dyn_ref::<HtmlElement>()
        .is_some_and(|el| el.content_editable() == "plaintext-only");
      if !supported {
        let _ = content.set_attribute("contenteditable", "true");
      }
    }
    ctx.state.update_value(|st| {
      probe_metrics(st, &content);
      st.load(&incoming);
      st.emitted = incoming.clone();
      ctx.blank.set(st.is_blank());
    });
    // A document that arrived from outside is a different document: it is read
    // from the top, and `render` — not `draw` — is what works out which lines
    // that means.
    root.set_scroll_top(0);
    render(ctx, true);
    // Everything below the first window is still a guess. Go and find out.
    schedule_pricing(ctx);
  });

  // The same text wraps into more rows in a narrower editor, so a width change
  // invalidates the probe and every estimate that came from it.
  {
    use leptos_use::use_resize_observer;
    let last_width = StoredValue::new(-1);
    use_resize_observer(ctx.root, move |_, _| {
      let Some((root, content)) = ctx.elements() else {
        return;
      };
      let width = root.client_width();
      if width == last_width.get_value() {
        return;
      }
      last_width.set_value(width);
      ctx.state.update_value(|st| {
        // Every height on the books was measured at the old width, and the same
        // text wraps into a different number of rows at the new one — so none of
        // them is worth keeping. The window is re-measured on the way out.
        probe_metrics(st, &content);
        st.measured.iter_mut().for_each(|measured| *measured = false);
        st.priced = 0;
        st.reprice(0);
      });
      render(ctx, true);
      schedule_pricing(ctx);
    });
  }

  let on_keydown = move |ev: web_sys::KeyboardEvent| {
    let key = ev.key();
    let ctrl = ev.ctrl_key() || ev.meta_key();
    // The whole-document keys work on the whole document: materialise it and let
    // the browser do exactly what it does everywhere else. The window comes back
    // on the next scroll or edit.
    if ctrl && matches!(key.as_str(), "a" | "A" | "ф" | "Ф" | "Home" | "End") {
      materialise(ctx);
      // `Ctrl+End` is the browser's to carry out once the document is there, and
      // it carries it out by showing the *caret* — which on a soft-wrapped last
      // line leaves the rest of that line below the fold. Somebody who asked for
      // the end of the document meant the end of the document.
      if matches!(key.as_str(), "Home" | "End") {
        request_animation_frame(move || {
          if let Some((root, content)) = ctx.elements() {
            scroll_caret_into_view(&root, &content);
          }
        });
      }
      return;
    }
    // Firefox does not always raise `beforeinput` for undo, so the shortcut is
    // caught here as well as there.
    if ctrl && matches!(key.as_str(), "z" | "Z" | "я" | "Я") {
      ev.prevent_default();
      history(ctx, !ev.shift_key());
      return;
    }
    if ctrl && matches!(key.as_str(), "y" | "Y" | "н" | "Н") {
      ev.prevent_default();
      history(ctx, false);
      return;
    }
    // `Escape` then `Tab` leaves the editor, because an editor that traps `Tab`
    // is an editor a keyboard user cannot get out of. Any other key puts the
    // trap back.
    if key == "Escape" {
      ctx.state.update_value(|st| st.tab_escapes = true);
      return;
    }
    let escaping = ctx
      .state
      .try_update_value(|st| std::mem::replace(&mut st.tab_escapes, false));
    if key == "Tab" && editable && escaping != Some(true) {
      ev.prevent_default();
      if ev.shift_key() {
        outdent(ctx, &indent.get_value());
      } else {
        exec("insertText", &indent.get_value());
      }
      return;
    }
    // Arrows and paging can walk the caret to the edge of what is rendered. Let
    // the browser move it first, then give the window a chance to follow.
    if matches!(
      key.as_str(),
      "ArrowUp" | "ArrowDown" | "PageUp" | "PageDown" | "Home" | "End" | "Enter"
    ) {
      request_animation_frame(move || render(ctx, false));
    }
  };

  let on_before_input = move |ev: web_sys::InputEvent| {
    // Our own undo stack, because the browser's remembers DOM nodes and we throw
    // those away every time the window moves.
    match ev.input_type().as_str() {
      "historyUndo" => {
        ev.prevent_default();
        history(ctx, true);
      }
      "historyRedo" => {
        ev.prevent_default();
        history(ctx, false);
      }
      _ => {}
    }
  };

  let on_paste = move |ev: web_sys::ClipboardEvent| {
    // Pasted as text by us rather than by the browser: under the
    // `contenteditable=true` fallback the browser would paste markup into a
    // document that is not markup.
    let Some(text) = ev.clipboard_data().and_then(|data| data.get_data("text/plain").ok()) else {
      return;
    };
    ev.prevent_default();
    exec("insertText", &text.replace("\r\n", "\n").replace('\r', "\n"));
  };

  view! {
    <div
      node_ref=ctx.root
      data-slot="source-editor"
      class=cn(&[ROOT_CLASSES, class.as_str()])
      aria-disabled=disabled.then_some("true")
      on:scroll=move |_| {
        render(ctx, false);
        // The window leans the way the reader is going while they are going;
        // once they stop, one more render draws it back to the viewport, so a
        // field being *typed* in is not carrying three screens of margin.
        if let Some(pending) = ctx.settle.get_value() {
          pending.clear();
        }
        let handle = set_timeout_with_handle(move || render(ctx, false), SETTLE_AFTER);
        ctx.settle.set_value(handle.ok());
      }
      // A drag that ends is a selection that stops growing: the window is free
      // to shrink back to the viewport again.
      on:pointerup=move |_| request_animation_frame(move || render(ctx, false))
    >
      {line_numbers
        .then(|| {
          view! {
            <div
              node_ref=ctx.gutter
              aria-hidden="true"
              class="pointer-events-none absolute top-0 left-0 w-12 select-none text-right font-mono text-xs text-muted-foreground/60"
            />
          }
        })}
      <div
        node_ref=ctx.content
        data-slot="source-editor-content"
        // `plaintext-only` is what makes an editable element behave like a text
        // field: no markup, and none of the rich-text keyboard shortcuts.
        contenteditable=editable.then_some("plaintext-only")
        spellcheck=spellcheck.then_some("true")
        role="textbox"
        aria-multiline="true"
        aria-readonly=readonly.then_some("true")
        class=cn(&[CONTENT_CLASSES, if line_numbers { "py-2 pr-3 pl-14" } else { "px-3 py-2" }])
        on:beforeinput=on_before_input
        on:input=move |_| sync(ctx)
        on:keydown=on_keydown
        on:paste=on_paste
        on:compositionstart=move |_| ctx.state.update_value(|st| st.composing = true)
        on:compositionend=move |_| {
          ctx.state.update_value(|st| st.composing = false);
          sync(ctx);
        }
      />
      <Show when=move || has_placeholder && ctx.blank.get()>
        <div class=cn(
          &[
            "pointer-events-none absolute top-0 py-2 text-muted-foreground",
            if line_numbers { "left-14" } else { "left-3" },
          ],
        )>{placeholder.clone()}</div>
      </Show>
    </div>
  }
}

// -- The two directions the document moves in -------------------------------

/// Model to DOM: renders the window the current scroll position calls for, then
/// measures what it rendered and puts the caret back.
///
/// `force` renders even when the range has not moved — after a reload, a resize,
/// or an edit that changed the shape of the DOM.
fn render(ctx: Ctx, force: bool) {
  let Some((root, content)) = ctx.elements() else {
    return;
  };
  if ctx.state.with_value(|st| st.composing) {
    return;
  }
  let caret = ctx.state.with_value(|st| dom_caret(&content, st.first));
  // A selection that is not a bare caret is one somebody is using, and it lives
  // in the nodes on screen: the rows it is anchored in have to stay. The window
  // may still *grow* under it — that is what lets a drag run past the edge of
  // what was rendered instead of stopping at it — it just may not let go of
  // anything.
  let held = selection_is_ranged(&content);
  let (going, heading) = ctx
    .state
    .try_update_value(|st| {
      let scroll_top = f64::from(root.scroll_top());
      let going = scroll_top - st.last_scroll;
      st.last_scroll = scroll_top;
      if going.abs() > 1.0 {
        st.heading = going;
        st.last_moved_ms = now_ms();
      }
      (going, st.heading)
    })
    .unwrap_or((0.0, 0.0));
  let moved = ctx.state.try_update_value(|st| {
    let view_h = f64::from(root.client_height());
    let scroll_top = f64::from(root.scroll_top());
    let (mut first, mut last) = window_for(st, scroll_top, view_h, caret, going);
    if held && st.last > st.first {
      first = first.min(st.first);
      last = last.max(st.last);
    }
    if !force && first == st.first && last == st.last {
      return false;
    }
    st.first = first;
    st.last = last;
    true
  });
  if moved != Some(true) {
    return;
  }
  // Only a bare caret is worth putting back. A live selection is already in the
  // rows that were kept — and "putting the caret back" would collapse it, which
  // is how a drag used to die the moment the mouse came up.
  // The window leans towards where the scroll is going *now*; whether the end
  // may be followed depends on where the reader was last actually going.
  draw(ctx, &root, &content, (!held).then_some(caret).flatten(), heading);
}

/// Paints the window, measures it, and keeps the line the reader is looking at
/// where they are looking at it.
///
/// The heights of unrendered lines are estimates, and rendering replaces an
/// estimate with the truth — which moves everything below it. Anchoring on the
/// topmost visible line is what keeps that correction from yanking the text
/// around as the reader scrolls.
fn draw(ctx: Ctx, root: &Element, content: &Element, caret: Option<Pos>, going: f64) {
  let scroll_top = f64::from(root.scroll_top());
  // Somebody who scrolled to the end meant the end, not the pixel the end used
  // to be at. Measuring the last window replaces estimates with the truth and
  // the end moves — a couple of lines further off, typically, which is exactly
  // far enough to leave a reader who flicked at it stranded just short. So:
  // anyone who had effectively arrived is taken to wherever the end turned out
  // to be. The slack is two lines, wide enough to cover the error and narrow
  // enough that somebody deliberately stopping short is left where they are.
  let gutter = ctx.gutter_element();
  // A reader at the end is anchored to the end, not to a line: swallowing a
  // measurement into the guesses above would keep their *line* still and push
  // the end away from them, which is the whole complaint — arriving at the end
  // and finding it has moved on again.
  let near_end = ctx.state.with_value(|st| {
    let reach = (st.heights.last().copied().unwrap_or(st.row_h) * 2.0).max(st.row_h * 3.0);
    let max = f64::from(root.scroll_height() - root.client_height());
    scroll_top > 0.0 && scroll_top >= max - reach
  });
  // Hold the document's height up while the rows are swapped. Between taking the
  // old rows out and putting the new padding in, the content is briefly shorter
  // than it was — and a browser will not keep a scroll position that no longer
  // exists, so it clamps, and the reader is thrown back by whatever the document
  // momentarily lost. Nothing in this code moves them; the damage is done before
  // it gets to look. Padding the bottom out to the height the document already
  // has means it never shrinks in between, so there is nothing to clamp.
  let moving = ctx.state.with_value(State::is_moving);
  hold_height(content, f64::from(root.scroll_height()));
  let corrected = ctx.state.try_update_value(|st| {
    let anchor = st.line_at(scroll_top);
    let was = st.offset_of(anchor);
    paint(st, content, ctx.highlight);
    measure(st, content);
    // Measuring rows that were guesses has moved the anchor line. Take the
    // difference out of the guesses above rather than out of the reader's scroll
    // position; only when there is nothing up there to take it from does the
    // scroll have to move.
    let drift = st.offset_of(anchor) - was;
    let absorbed = !near_end && (drift.abs() < 0.5 || st.absorb_above(drift));
    pad(st, content);
    if let Some(gutter) = &gutter {
      paint_gutter(st, gutter, content);
    }
    if let Some(caret) = caret {
      set_dom_caret(content, st.first, caret);
    }
    if absorbed {
      None
    } else {
      Some(st.offset_of(anchor) + (scroll_top - was))
    }
  });
  if let Some(Some(corrected)) = corrected
    && (corrected - scroll_top).abs() > 0.5
    && !moving
  {
    // Never while the reader is moving. This correction keeps the line they are
    // looking at still when a measurement changes the geometry above it — worth
    // having when they are at rest, and not worth anything at all mid-gesture:
    // writing the scroll position while a scrollbar thumb is being dragged
    // fights the drag, and the drag feels like a wall the reader is pushing
    // against. Better a line that shifts a little than a document that will not
    // move. What is left settles a fifth of a second after they stop.
    set_scroll(ctx, root, corrected);
  }
  follow_end(ctx, root, going);
}

// -- Pricing the document ----------------------------------------------------

/// Books the next frame of the walk down the document, if one is due.
///
/// Idempotent: everything that can invalidate a height calls it, and only the
/// first call in flight books anything.
fn schedule_pricing(ctx: Ctx) {
  let due = ctx.state.try_update_value(|st| {
    if st.pricing || st.priced >= st.lines.len() {
      return false;
    }
    st.pricing = true;
    true
  });
  if due != Some(true) {
    return;
  }
  request_animation_frame(move || price_chunk(ctx));
}

/// Lays the next batch of unmeasured lines out and writes down what they are
/// really worth.
///
/// This is what stops the document's height from moving under a reader who is
/// already in it. The alternative — pricing lines from a fitted average and
/// correcting each one as it is scrolled into view — is right on average and
/// wrong everywhere in particular, and the error does not stay put: every
/// window that gets measured shifts the end of the document by the difference.
/// A browser dragging a scrollbar thumb maps the pointer against the height it
/// saw when the drag began, so a document that grows a few percent under the
/// drag is a drag that runs out of travel a few percent short of the end — the
/// invisible wall, felt as "let go and grab it again and it works".
///
/// So the guess is only ever a stand-in for the second or two before the real
/// answer arrives. Laying a few dozen paragraphs out is a couple of
/// milliseconds, and it is spread over frames so nothing stutters. Once the
/// walk is done every height is a measurement, the end of the document stops
/// moving for good, and none of the machinery that exists to paper over a
/// moving end has anything left to do.
///
/// The lines are laid out **in the editable itself**, appended past the last
/// row and taken out again before anything can be painted. A twin element off
/// to one side was the obvious way to do it and it was quietly wrong: a browser
/// does not lay out two boxes the same way just because they carry the same
/// classes. Chromium's mobile text autosizer boosts the font in a tall block of
/// prose and leaves a short hidden one alone, and the twin came back with rows
/// a third the height of the real ones — a document mispriced threefold, with
/// nothing on screen to show for it. Measured where the text actually lives,
/// there is no second box to disagree.
fn price_chunk(ctx: Ctx) {
  ctx.state.update_value(|st| st.pricing = false);
  let Some((root, content)) = ctx.elements() else {
    return;
  };
  // Never mid-gesture and never mid-composition: this moves the ground the
  // reader is standing on, and putting them back means writing the scroll
  // position, which on a phone cancels the fling the platform is animating.
  // Never mid-keystroke either — for a duller reason, that laying lines out
  // dirties the layout the next keystroke has to measure against.
  if ctx.state.with_value(|st| st.composing || st.is_busy()) {
    // Stays booked across the wait, so a scroll event cannot queue a second
    // walk down the same document.
    ctx.state.update_value(|st| st.pricing = true);
    let _ = set_timeout_with_handle(
      move || {
        ctx.state.update_value(|st| st.pricing = false);
        schedule_pricing(ctx);
      },
      SETTLE_AFTER,
    );
    return;
  }
  let Some(doc) = content.owner_document() else {
    return;
  };

  // Skip past everything already measured — a line rendered on screen has
  // already told us the truth — and collect the next chunk that has not.
  let started = now_ms();
  let mut todo: Vec<usize> = Vec::new();
  let fragment = doc.create_document_fragment();
  let batch = ctx.state.with_value(|st| st.price_batch);
  ctx.state.update_value(|st| {
    while st.priced < st.lines.len() && todo.len() < batch {
      let line = st.priced;
      st.priced += 1;
      if st.measured[line] {
        continue;
      }
      // No highlighting: spans are inline and wrap exactly as the bare text
      // does, so colouring a line the reader will never see is work for
      // nothing.
      if let Some(div) = make_line(&doc, &st.lines[line], None) {
        let _ = fragment.append_child(&div);
        todo.push(line);
      }
    }
  });

  // Appended past the last row, measured, and taken out again — all in this one
  // turn, with no chance for the browser to paint in between, so nothing of it
  // is ever on screen. The document is only ever *longer* while they are there,
  // and a scroll position that was valid before a block grows is still valid
  // after, so nothing is clamped and the reader does not move.
  let rows = content.children().length();
  let _ = content.append_child(&fragment);
  let children = content.children();
  let heights: Vec<f64> = (0..todo.len() as u32)
    .map(|i| {
      children
        .item(rows + i)
        .map_or(0.0, |child| child.get_bounding_client_rect().height())
    })
    .collect();
  while content.children().length() > rows {
    if let Some(extra) = content.last_element_child() {
      let _ = content.remove_child(&extra);
    } else {
      break;
    }
  }
  // What that chunk actually cost decides the size of the next one. A document
  // of headings gets thousands of lines a frame; one of long paragraphs gets
  // tens; and a slow phone gets fewer of either, without being told which it is.
  let spent = now_ms() - started;
  ctx.state.update_value(|st| {
    let done = todo.len() as f64;
    st.price_batch = if done < 1.0 {
      st.price_batch
    } else if spent < 0.5 {
      (st.price_batch * 4).min(4096)
    } else {
      ((PRICE_BUDGET_MS * done / spent) as usize).clamp(8, 4096)
    };
  });

  let scroll_top = f64::from(root.scroll_top());
  let gutter = ctx.gutter_element();
  let drift = ctx.state.try_update_value(|st| {
    let anchor = st.line_at(scroll_top);
    let was = st.offset_of(anchor);
    for (&line, &height) in todo.iter().zip(heights.iter()) {
      if height <= 0.0 || line >= st.heights.len() {
        continue;
      }
      st.heights[line] = height;
      st.measured[line] = true;
      let text = st.lines[line].clone();
      st.observe(&text, height);
    }
    // The lines just priced are the best evidence there is about the ones below
    // them, so the stand-in the rest of the document is still carrying is
    // redone on it. Only below where pricing has reached: above that nothing is
    // a guess any more.
    if st.estimates_are_stale() {
      let from = st.priced;
      st.reprice(from);
    }
    pad(st, &content);
    st.offset_of(anchor) - was
  });
  // Pricing lines *above* the reader moves every line below them, including the
  // one they are looking at. Nothing is being dragged or flung — that was
  // checked above — so the scroll position is free to move, and moving it is
  // what keeps their place.
  //
  // The numbers in the gutter move with it, and only with it: the lines in the
  // window were measured for real as they were painted and are never re-priced
  // here, so the padding above them is the only thing that can put a number
  // beside the wrong line. No drift, no repaint.
  if let Some(drift) = drift
    && drift.abs() > 0.5
  {
    set_scroll(ctx, &root, scroll_top + drift);
    if let Some(gutter) = &gutter {
      ctx.state.with_value(|st| paint_gutter(st, gutter, &content));
    }
  }
  // The end of the document has just moved on purpose. That is not the end
  // running away from a reader who was standing on it, so it is not something
  // to chase.
  ctx.state.update_value(|st| {
    st.last_max = f64::from(root.scroll_height() - root.client_height());
  });
  schedule_pricing(ctx);
}

/// Carries a reader who is at the end of the document along when the end moves.
///
/// The end moves for one reason: measuring the last window replaces guesses with
/// the truth, and the truth is a couple of lines further down. That is exactly
/// far enough to leave somebody who flicked at the end stranded short of the
/// last line, for a reason no reader can see.
///
/// So this fires on one condition — **the end moved away from them**, and they
/// were at it before it did — and carries them by exactly as far as it moved.
/// Never because *they* moved away from it: a reader creeping up off the last
/// line has left, and posting them back down on the next frame is every bit as
/// bad as never letting them arrive. Nothing is remembered here between calls
/// except where the end was, because a rule with no memory cannot strand anybody
/// in a state it forgot to leave.
fn follow_end(ctx: Ctx, root: &Element, going: f64) {
  let scroll_top = f64::from(root.scroll_top());
  let max = f64::from(root.scroll_height() - root.client_height());
  let view_h = f64::from(root.client_height());
  let Some((growth, reach)) = ctx.state.try_update_value(|st| {
    let growth = max - st.last_max;
    st.last_max = max;
    // "At the end" measured in the lines this document actually has, not in
    // rows: on a phone a wrapped paragraph is a third of the screen, and a
    // couple of rows would be no slack at all. Half a screen is the ceiling —
    // beyond that they are not at the end by any reading, whatever the last
    // paragraph of the document happens to be worth.
    let last_line = st.heights.last().copied().unwrap_or(st.row_h);
    let reach = (last_line * 2.0).max(st.row_h * 3.0).min(view_h * 0.5);
    (growth, reach)
  }) else {
    return;
  };
  // `scroll_top > 0` so that the first draw of all — content still empty, so the
  // end is also the beginning — cannot carry a reader to the bottom of a
  // document they have not read a line of.
  if growth <= 0.5 || going < -1.0 || scroll_top <= 0.0 {
    return;
  }
  if scroll_top >= max - growth - reach {
    set_scroll(ctx, root, (scroll_top + growth).min(max));
  }
}

/// DOM to model: the window is the truth for its own lines. Read them back,
/// splice them in, and leave the rest of the document alone.
fn sync(ctx: Ctx) {
  let Some((root, content)) = ctx.elements() else {
    return;
  };
  if ctx.state.with_value(|st| st.composing) {
    return;
  }
  let read = read_window(&content);
  let caret = ctx
    .state
    .with_value(|st| dom_caret(&content, st.first))
    .unwrap_or_default();

  let restructured = ctx.state.try_update_value(|st| {
    let first = st.first.min(st.lines.len());
    let last = st.last.clamp(first, st.lines.len());
    if read == st.lines[first..last] {
      return None;
    }
    // Only the lines that actually differ are cloned onto the undo stack, which
    // for ordinary typing is exactly one.
    let (head, tail) = diff(&st.lines[first..last], &read);
    let removed = st.lines[first + head..last - tail].to_vec();
    let inserted = read[head..read.len() - tail].to_vec();
    let touched = first + head;
    record(st, touched, removed, inserted, caret);
    // A changed line count means the browser merged or split our line divs —
    // Enter drops a bare `\n` into one of them — so the DOM has to be rebuilt
    // into one div per line before anything measures it again.
    let restructured = read.len() != last - first;
    let heights: Vec<f64> = read.iter().map(|line| st.estimate(line)).collect();
    st.lines.splice(first..last, read.iter().cloned());
    st.heights.splice(first..last, heights);
    st.measured.splice(first..last, std::iter::repeat_n(false, read.len()));
    // A splice moves every line after it along, so pricing's place in the
    // document is no longer where it thinks it is — and a paste can drop a
    // thousand unpriced lines in at once. It goes back to the edit and walks
    // down again; the lines it already knows cost it a bounds check each.
    st.priced = st.priced.min(first);
    st.first = first;
    st.last = first + read.len();
    if restructured {
      // The rows in the DOM no longer stand one to a line — Enter leaves two
      // lines inside one of them — so none of them can be reused.
      st.painted = (0, 0);
    }
    st.emitted = st.text();
    Some((restructured, touched))
  });
  let Some(Some((restructured, touched))) = restructured else {
    return;
  };

  ctx.state.with_value(|st| {
    ctx.blank.set(st.is_blank());
    ctx.value.set(st.emitted.clone());
  });
  schedule_pricing(ctx);

  if restructured {
    draw(
      ctx,
      &root,
      &content,
      dom_caret(&content, ctx.state.with_value(|st| st.first)),
      0.0,
    );
  } else {
    // Nothing moved between lines, so only the line that was typed in can have
    // changed height — and usually it has not. Measuring that one line, and
    // touching the padding and the numbers only when it grew a row, is what
    // keeps a keystroke costing the same whether the window holds ten rows or a
    // hundred. The DOM is otherwise left alone: rewriting a line under a live
    // caret is how editors lose keystrokes.
    let gutter = ctx.gutter_element();
    let grew = ctx
      .state
      .try_update_value(|st| measure_line(st, &content, touched))
      .unwrap_or(false);
    if grew {
      ctx.state.update_value(|st| {
        pad(st, &content);
        if let Some(gutter) = &gutter {
          paint_gutter(st, gutter, &content);
        }
      });
    }
    if ctx.highlight.is_some() {
      if let Some(pending) = ctx.relight.get_value() {
        pending.clear();
      }
      let handle = set_timeout_with_handle(move || relight(ctx), RELIGHT_AFTER);
      ctx.relight.set_value(handle.ok());
    }
  }
}

// -- The window -------------------------------------------------------------

/// Which lines belong in the DOM at a given scroll position: what shows, a
/// margin either side of it — wider in the direction of travel — and always the
/// caret's line, so an arrow key never walks off the end of what exists.
fn window_for(
  st: &State,
  scroll_top: f64,
  view_h: f64,
  caret: Option<Pos>,
  going: f64,
) -> (usize, usize) {
  let count = st.lines.len();
  let margin = view_h * OVERSCAN;
  let (above, below) = if going > 1.0 {
    (margin * TRAIL, margin * LEAD)
  } else if going < -1.0 {
    (margin * LEAD, margin * TRAIL)
  } else {
    (margin, margin)
  };
  let top_line = st.line_at(scroll_top);
  let bottom_line = st.line_at(scroll_top + view_h);
  // The floor only bites where a screenful is fewer lines than it.
  let floor = MIN_OVERSCAN_LINES.saturating_sub(bottom_line - top_line);
  let mut first = st
    .line_at((scroll_top - above).max(0.0))
    .min(top_line.saturating_sub(floor));
  let mut last = (st.line_at(scroll_top + view_h + below) + 1)
    .max(bottom_line + 1 + floor)
    .min(count);
  if let Some(caret) = caret {
    first = first.min(caret.line.saturating_sub(2));
    last = last.max((caret.line + 3).min(count));
  }
  first = first.min(count.saturating_sub(1));
  (first, last.clamp(first + 1, count))
}

/// Brings the DOM in line with the window, keeping every row that is staying.
///
/// Rebuilding the lot on each scroll is about a millisecond, which would be
/// affordable — but it also throws away the nodes a live selection is anchored
/// in, and a reader dragging a selection past the edge of the window would watch
/// it collapse. Adding and dropping rows at the ends instead is what lets the
/// window grow under a selection rather than freeze until the mouse comes up.
///
/// A rebuild is still the answer when the new window shares nothing with the old
/// one, or when the browser has been editing and the rows no longer stand one to
/// a line — which [`sync`] says by clearing `painted`.
fn paint(st: &mut State, content: &Element, highlight: Option<Highlighter>) {
  let Some(doc) = content.owner_document() else {
    return;
  };
  let (first, last) = (st.first, st.last.min(st.lines.len()));
  let (was_first, was_last) = st.painted;
  let intact = was_last > was_first
    && first < was_last
    && last > was_first
    && content.children().length() as usize == was_last - was_first;

  if !intact {
    let fragment = doc.create_document_fragment();
    for line in &st.lines[first.min(st.lines.len())..last] {
      if let Some(div) = make_line(&doc, line, highlight) {
        let _ = fragment.append_child(&div);
      }
    }
    content.set_inner_html("");
    let _ = content.append_child(&fragment);
    st.painted = (first, last);
    return;
  }

  // Off the front, off the back, then on at the front and on at the back. Doing
  // the removals first keeps the arithmetic in document order.
  for _ in was_first..first.min(was_last) {
    if let Some(row) = content.first_element_child() {
      let _ = content.remove_child(&row);
    }
  }
  for _ in last.max(was_first)..was_last {
    if let Some(row) = content.last_element_child() {
      let _ = content.remove_child(&row);
    }
  }
  for line in (first..was_first.min(last)).rev() {
    if let Some(div) = make_line(&doc, &st.lines[line], highlight) {
      let _ = content.insert_before(&div, content.first_child().as_ref());
    }
  }
  for line in was_last.max(first)..last {
    if let Some(div) = make_line(&doc, &st.lines[line], highlight) {
      let _ = content.append_child(&div);
    }
  }
  st.painted = (first, last);
}

/// One line's worth of DOM.
///
/// An empty line gets a `<br>`: an empty block is zero pixels tall, and a line
/// nobody can see is a line nobody can click into.
fn make_line(doc: &Document, text: &str, highlight: Option<Highlighter>) -> Option<Element> {
  let div = doc.create_element("div").ok()?;
  if text.is_empty() {
    let br = doc.create_element("br").ok()?;
    let _ = div.append_child(&br);
    return Some(div);
  }
  let Some(highlight) = highlight else {
    div.set_text_content(Some(text));
    return Some(div);
  };
  let mut at = 0usize;
  for span in highlight(text) {
    // A highlighter handing back overlapping, unsorted or mid-character offsets
    // is ignored rather than allowed to corrupt the line.
    if span.from < at
      || span.to <= span.from
      || span.to > text.len()
      || !text.is_char_boundary(span.from)
      || !text.is_char_boundary(span.to)
    {
      continue;
    }
    if span.from > at {
      let _ = div.append_child(&doc.create_text_node(&text[at..span.from]));
    }
    if let Ok(el) = doc.create_element("span") {
      el.set_class_name(span.class);
      el.set_text_content(Some(&text[span.from..span.to]));
      let _ = div.append_child(&el);
    }
    at = span.to;
  }
  if at < text.len() {
    let _ = div.append_child(&doc.create_text_node(&text[at..]));
  }
  Some(div)
}

/// Reads the rendered lines back out of the DOM.
fn read_window(content: &Element) -> Vec<String> {
  let mut lines = Vec::new();
  let mut child = content.first_child();
  while let Some(node) = child {
    let text = match node.node_type() {
      Node::TEXT_NODE => node.node_value().unwrap_or_default(),
      Node::ELEMENT_NODE => line_text(&node),
      _ => {
        child = node.next_sibling();
        continue;
      }
    };
    lines.extend(text.split('\n').map(str::to_string));
    child = node.next_sibling();
  }
  if lines.is_empty() {
    lines.push(String::new());
  }
  lines
}

/// What a rendered line says it holds.
///
/// Text nodes as they are, a `<br>` as a newline — and one trailing newline
/// dropped, because that is what the rendering rules add: a block ending in a
/// newline needs a second one before a browser will show an empty last row, and
/// pressing Enter at the end of a line is exactly how one gets there.
fn line_text(node: &Node) -> String {
  let mut out = String::new();
  collect_text(node, &mut out);
  if out.ends_with('\n') {
    out.pop();
  }
  out
}

/// The text under `node`, `<br>`s included, without the trailing-newline rule.
fn collect_text(node: &Node, out: &mut String) {
  let mut child = node.first_child();
  while let Some(node) = child {
    match node.node_type() {
      Node::TEXT_NODE => out.push_str(&node.node_value().unwrap_or_default()),
      Node::ELEMENT_NODE if node.node_name() == "BR" => out.push('\n'),
      Node::ELEMENT_NODE => collect_text(&node, out),
      _ => {}
    }
    child = node.next_sibling();
  }
}

/// Moves the scroll ourselves, and remembers that it was us.
///
/// Every write here is the editor making good on one of its own guesses, never
/// the reader going anywhere — so it must not be read back as movement on the
/// next render. Left unmarked, a correction downwards reads as "they are heading
/// down", which is exactly the licence the end-following rule needs to do it
/// again: the editor ends up chasing its own tail down the document.
fn set_scroll(ctx: Ctx, root: &Element, px: f64) {
  let px = px.round();
  root.set_scroll_top(px as i32);
  let landed = f64::from(root.scroll_top());
  ctx.state.update_value(|st| st.last_scroll = landed);
}

/// Props the scrollable area up at `px` while the rows underneath it change.
///
/// `min-height` rather than padding: it holds the floor without ever raising the
/// ceiling, so the document's height cannot momentarily *grow* either. A browser
/// dragging a scrollbar thumb maps the pointer against the height it saw when
/// the drag began, and every wobble in that height is a wobble in the drag.
/// [`pad`] takes it off again once the real padding is known.
fn hold_height(content: &Element, px: f64) {
  if let Some(el) = content.dyn_ref::<HtmlElement>() {
    let _ = el.style().set_property("min-height", &format!("{}px", px.round()));
  }
}

/// Replaces the lines off screen with the space they would have taken.
///
/// Padding rather than spacer elements on purpose: padding cannot be typed into,
/// selected, or landed in by a caret, and it costs the browser nothing.
fn pad(st: &State, content: &Element) {
  let Some(el) = content.dyn_ref::<HtmlElement>() else {
    return;
  };
  let above = st.offset_of(st.first);
  let below = st.total() - st.offset_of(st.last.min(st.lines.len()));
  let style = el.style();
  let _ = style.set_property("padding-top", &format!("{}px", above.max(0.0).round()));
  let _ = style.set_property("padding-bottom", &format!("{}px", below.max(0.0).round()));
  // The prop [`hold_height`] put under the swap comes off now that the real
  // heights are in.
  let _ = style.remove_property("min-height");
}

/// Records what the rendered lines actually turned out to be worth.
fn measure(st: &mut State, content: &Element) {
  let children = content.children();
  for i in 0..children.length() {
    let Some(child) = children.item(i) else {
      continue;
    };
    let line = st.first + i as usize;
    if line >= st.heights.len() {
      break;
    }
    let height = child.get_bounding_client_rect().height();
    if height > 0.0 {
      st.heights[line] = height;
      st.measured[line] = true;
      let text = st.lines[line].clone();
      st.observe(&text, height);
    }
  }
  // What was just measured may well say that everything still estimated is worth
  // something else. Re-pricing here is what keeps the scrollbar — and with it the
  // end of the document — from moving as the reader scrolls into it.
  //
  // But not *while* they are moving. Re-pricing changes the document's height,
  // and a scrollbar drag is mapped against the height the browser saw when the
  // drag began: shift it mid-drag and the rest of the thumb's travel points
  // somewhere else, which is felt as running into a wall short of the end. It
  // waits for the scrolling to stop — the settle render, a fifth of a second
  // later, does it when nobody is holding on to anything.
  if st.estimates_are_stale() && !st.is_moving() {
    let from = st.first;
    st.reprice(from);
  }
}

/// Re-measures one line — the one just typed in — and says whether it changed
/// height. Nothing else on screen can have moved, so nothing else is worth
/// asking the browser about.
fn measure_line(st: &mut State, content: &Element, line: usize) -> bool {
  let Some(index) = line.checked_sub(st.first) else {
    return false;
  };
  let Some(row) = content.children().item(index as u32) else {
    return false;
  };
  if line >= st.heights.len() {
    return false;
  }
  let height = row.get_bounding_client_rect().height();
  if height <= 0.0 {
    return false;
  }
  let grew = (height - st.heights[line]).abs() > 0.5;
  st.heights[line] = height;
  st.measured[line] = true;
  let text = st.lines[line].clone();
  st.observe(&text, height);
  grew
}

/// Measures the font: one row's height, and how many characters fit on a row at
/// the width the editor has now. Both feed the estimate that stands in for lines
/// nobody has rendered.
fn probe_metrics(st: &mut State, content: &Element) {
  let Some(doc) = content.owner_document() else {
    return;
  };
  let Ok(probe) = doc.create_element("div") else {
    return;
  };
  probe.set_text_content(Some("M"));
  let _ = content.append_child(&probe);
  let row = probe.get_bounding_client_rect().height();
  // A long, ordinary-looking run measures the *average* character rather than
  // the widest one, which is what an estimate wants from a proportional font.
  // This is only the opening guess: the first window that gets rendered replaces
  // it with the document's own lines.
  let sample = "aeiou nstr lmdc ohig ".repeat(20);
  probe.set_text_content(Some(&sample));
  let sample_h = probe.get_bounding_client_rect().height();
  let _ = content.remove_child(&probe);

  if row > 0.0 {
    st.row_h = row;
    // The probe enters the fit as evidence like any other, worth about a line's
    // worth of characters: enough to price a document before a single line of it
    // has been laid out, light enough that the document's own lines overrule it
    // as soon as there are any.
    let sample_chars = sample.chars().count() as f64;
    st.fit_chars = PROBE_WEIGHT;
    st.fit_extra = (sample_h - row).max(0.0) * PROBE_WEIGHT / sample_chars;
    st.applied_slope = f64::NAN;
  }
}

/// Draws the line numbers beside the lines they belong to.
///
/// Each number is placed at the line's own `offsetTop` rather than at a position
/// worked out from the model. `offsetTop` is measured from the same edge an
/// absolutely positioned cell's `top` is — the scroll box's padding edge — so
/// the two agree by construction, whatever padding stands between them and
/// however wrong an estimated height further up may be.
///
/// Arithmetic is what this used to do, and it was wrong in a way that took a
/// while to see: the padding standing in for the lines above does not move an
/// absolutely positioned child, so every number came out a window's worth of
/// document too high — line 980 wearing the number 1920.
fn paint_gutter(st: &State, gutter: &Element, content: &Element) {
  let Some(doc) = gutter.owner_document() else {
    return;
  };
  let fragment = doc.create_document_fragment();
  let children = content.children();
  for i in 0..children.length() {
    let Some(child) = children.item(i) else {
      continue;
    };
    let line = st.first + i as usize;
    if line >= st.lines.len() {
      break;
    }
    let Ok(cell) = doc.create_element("div") else {
      continue;
    };
    cell.set_class_name("absolute right-2");
    let top = child.dyn_ref::<HtmlElement>().map_or(0, HtmlElement::offset_top);
    if let Some(cell_el) = cell.dyn_ref::<HtmlElement>() {
      let _ = cell_el.style().set_property("top", &format!("{top}px"));
    }
    cell.set_text_content(Some(&(line + 1).to_string()));
    let _ = fragment.append_child(&cell);
  }
  gutter.set_inner_html("");
  let _ = gutter.append_child(&fragment);
}

/// Puts the whole document in the DOM and leaves it there until the selection
/// it was made for is gone.
///
/// This is what `Ctrl+A` and `Ctrl+Home`/`End` need: they are the browser's to
/// carry out, and a browser can only reach what it can see. On a very long
/// document it costs one hitch of a few tens of milliseconds, which is the
/// honest price of a whole-document operation.
fn materialise(ctx: Ctx) {
  let Some((root, content)) = ctx.elements() else {
    return;
  };
  let caret = ctx.state.with_value(|st| dom_caret(&content, st.first));
  ctx.state.update_value(|st| {
    st.first = 0;
    st.last = st.lines.len();
  });
  draw(ctx, &root, &content, caret, 0.0);
}

/// Whether the editor holds a selection with something in it, rather than a
/// bare caret.
fn selection_is_ranged(content: &Element) -> bool {
  let Some(selection) = selection() else {
    return false;
  };
  if selection.is_collapsed() {
    return false;
  }
  selection
    .anchor_node()
    .is_some_and(|anchor| content.contains(Some(&anchor)))
}

/// Re-colours the line the caret is on, a moment after the typing stops.
///
/// A line is highlighted when it is rendered, and the line being typed in is the
/// one line that deliberately is not re-rendered — rewriting it under a live
/// caret is how editors lose keystrokes. So it is rewritten once the typing
/// pauses, with the caret put back where it was: the writer sees a heading turn
/// into a heading, and never sees a keystroke go missing.
fn relight(ctx: Ctx) {
  let Some((_, content)) = ctx.elements() else {
    return;
  };
  let Some(highlight) = ctx.highlight else {
    return;
  };
  if ctx.state.with_value(|st| st.composing) || selection_is_ranged(&content) {
    return;
  }
  let Some(caret) = ctx.state.with_value(|st| dom_caret(&content, st.first)) else {
    return;
  };
  let Some(doc) = content.owner_document() else {
    return;
  };
  let replaced = ctx.state.with_value(|st| {
    let index = caret.line.checked_sub(st.first)?;
    let old = content.children().item(index as u32)?;
    let text = st.lines.get(caret.line)?;
    let new = make_line(&doc, text, Some(highlight))?;
    content.replace_child(&new, &old).ok()?;
    Some(())
  });
  if replaced.is_some() {
    ctx.state.with_value(|st| set_dom_caret(&content, st.first, caret));
  }
}

// -- Caret ------------------------------------------------------------------

/// Where the caret is, in the document's terms rather than the DOM's.
fn dom_caret(content: &Element, first: usize) -> Option<Pos> {
  let selection = selection()?;
  let anchor = selection.anchor_node()?;
  if !content.contains(Some(&anchor)) {
    return None;
  }
  let offset = selection.anchor_offset() as usize;
  let mut line = first;
  let mut child = content.first_child();
  while let Some(node) = child {
    let holds = node == anchor || node.contains(Some(&anchor));
    let text = match node.node_type() {
      Node::TEXT_NODE => node.node_value().unwrap_or_default(),
      Node::ELEMENT_NODE => line_text(&node),
      _ => String::new(),
    };
    if holds {
      let upto = text_offset_to(&node, &anchor, offset)?.min(text.len());
      if !text.is_char_boundary(upto) {
        return None;
      }
      let head = &text[..upto];
      return Some(Pos {
        line: line + head.matches('\n').count(),
        col: head.rsplit('\n').next().unwrap_or_default().len(),
      });
    }
    line += text.matches('\n').count() + 1;
    child = node.next_sibling();
  }
  None
}

/// How many bytes of `root`'s text come before `(target, offset)`.
fn text_offset_to(root: &Node, target: &Node, offset: usize) -> Option<usize> {
  let mut acc = 0usize;
  walk_to(root, target, offset, &mut acc).then_some(acc)
}

/// Walks `node`'s text in document order, adding to `acc`, and stops at
/// `target`. `true` once the target has been found.
fn walk_to(node: &Node, target: &Node, offset: usize, acc: &mut usize) -> bool {
  if node == target {
    if node.node_type() == Node::TEXT_NODE {
      let text = node.node_value().unwrap_or_default();
      *acc += utf16_prefix_bytes(&text, offset);
    } else {
      // A selection anchored on an element counts in child nodes, not
      // characters: everything before the `offset`-th child is behind the caret.
      let children = node.child_nodes();
      for i in 0..(offset as u32).min(children.length()) {
        if let Some(child) = children.item(i) {
          let mut text = String::new();
          collect_text_including(&child, &mut text);
          *acc += text.len();
        }
      }
    }
    return true;
  }
  match node.node_type() {
    Node::TEXT_NODE => {
      *acc += node.node_value().unwrap_or_default().len();
      false
    }
    Node::ELEMENT_NODE if node.node_name() == "BR" => {
      *acc += 1;
      false
    }
    _ => {
      let mut child = node.first_child();
      while let Some(node) = child {
        if walk_to(&node, target, offset, acc) {
          return true;
        }
        child = node.next_sibling();
      }
      false
    }
  }
}

/// [`collect_text`], but counting the node itself when it is a text node or a
/// `<br>` rather than only what is under it.
fn collect_text_including(node: &Node, out: &mut String) {
  match node.node_type() {
    Node::TEXT_NODE => out.push_str(&node.node_value().unwrap_or_default()),
    Node::ELEMENT_NODE if node.node_name() == "BR" => out.push('\n'),
    _ => collect_text(node, out),
  }
}

/// Puts the caret back on a line and byte offset of the document.
fn set_dom_caret(content: &Element, first: usize, pos: Pos) -> Option<()> {
  let doc = content.owner_document()?;
  let index = pos.line.checked_sub(first)?;
  let child = content.children().item(index as u32)?;
  let range = doc.create_range().ok()?;

  // The offset lands in one of the line's text nodes — or, on an empty line,
  // nowhere, since a `<br>` is not something a caret can sit inside.
  let mut acc = 0usize;
  if !place_in(child.unchecked_ref(), pos.col, &mut acc, &range) {
    range.set_start(child.unchecked_ref::<Node>(), 0).ok()?;
  }
  range.collapse_with_to_start(true);
  let selection = selection()?;
  selection.remove_all_ranges().ok()?;
  selection.add_range(&range).ok()?;
  Some(())
}

/// Walks a rendered line's text nodes looking for byte offset `col`, and starts
/// `range` there once it is found.
fn place_in(node: &Node, col: usize, acc: &mut usize, range: &Range) -> bool {
  if node.node_type() == Node::TEXT_NODE {
    let text = node.node_value().unwrap_or_default();
    if col <= *acc + text.len() {
      let within = (col - *acc).min(text.len());
      if !text.is_char_boundary(within) {
        return false;
      }
      return range.set_start(node, utf16_len(&text[..within]) as u32).is_ok();
    }
    *acc += text.len();
    return false;
  }
  let mut child = node.first_child();
  while let Some(node) = child {
    if place_in(&node, col, acc, range) {
      return true;
    }
    child = node.next_sibling();
  }
  false
}

// -- Editing the browser does not do for us ---------------------------------

/// Types through the browser, so the caret, the selection being replaced and the
/// DOM all end up where the browser thinks they should — and the `input` handler
/// reads the result back like any other edit.
fn exec(command: &str, value: &str) {
  if let Some(doc) = document().dyn_ref::<web_sys::HtmlDocument>() {
    let _ = doc.exec_command_with_show_ui_and_value(command, false, value);
  }
}

/// `Shift+Tab`: takes one level of indentation off the caret's line by selecting
/// it and deleting it, so that the edit arrives through the same path as any
/// other and lands on the undo stack with everything else.
fn outdent(ctx: Ctx, indent: &str) {
  let Some((_, content)) = ctx.elements() else {
    return;
  };
  let Some(caret) = ctx.state.with_value(|st| dom_caret(&content, st.first)) else {
    return;
  };
  let (first, line) = ctx
    .state
    .with_value(|st| (st.first, st.lines.get(caret.line).cloned().unwrap_or_default()));
  let spaces = line.len() - line.trim_start_matches(' ').len();
  let strip = if line.starts_with(indent) { indent.len() } else { spaces };
  if strip == 0 {
    return;
  }
  let Some(child) = caret
    .line
    .checked_sub(first)
    .and_then(|index| content.children().item(index as u32))
  else {
    return;
  };
  let Some(doc) = content.owner_document() else {
    return;
  };
  let (Ok(range), Ok(end)) = (doc.create_range(), doc.create_range()) else {
    return;
  };
  let (mut from_acc, mut to_acc) = (0usize, 0usize);
  if !place_in(child.unchecked_ref(), 0, &mut from_acc, &range)
    || !place_in(child.unchecked_ref(), strip, &mut to_acc, &end)
  {
    return;
  }
  let (Ok(node), Ok(offset)) = (end.start_container(), end.start_offset()) else {
    return;
  };
  if range.set_end(&node, offset).is_err() {
    return;
  }
  let Some(selection) = selection() else {
    return;
  };
  let _ = selection.remove_all_ranges();
  let _ = selection.add_range(&range);
  exec("delete", "");
}

// -- Undo -------------------------------------------------------------------

/// How many lines two versions of the window share at the front and at the back
/// — everything between the two is what the edit actually touched.
fn diff(before: &[String], after: &[String]) -> (usize, usize) {
  let head = before.iter().zip(after.iter()).take_while(|(a, b)| a == b).count();
  let tail = before[head..]
    .iter()
    .rev()
    .zip(after[head..].iter().rev())
    .take_while(|(a, b)| a == b)
    .count();
  (head, tail)
}

/// Folds a change into the undo stack, joining it to the one before when it is
/// the same kind of change in the same place — so that undo steps back over a
/// word rather than a letter.
fn record(st: &mut State, at: usize, removed: Vec<String>, inserted: Vec<String>, caret: Pos) {
  let now = now_ms();
  let coalesce = removed.len() == 1
    && inserted.len() == 1
    && now - st.last_edit_ms < UNDO_COALESCE_MS
    && st
      .undo
      .last()
      .is_some_and(|last| last.at == at && last.removed.len() == 1 && last.inserted.len() == 1);
  st.last_edit_ms = now;
  st.redo.clear();

  if coalesce {
    if let Some(last) = st.undo.last_mut() {
      last.inserted = inserted;
      last.after = caret;
    }
    return;
  }
  let before_caret = st.undo.last().map_or(caret, |last| last.after);
  st.undo.push(Edit {
    at,
    removed,
    inserted,
    before: before_caret,
    after: caret,
  });
}

/// Steps the document back — or forward — one recorded change.
fn history(ctx: Ctx, undo: bool) {
  let Some((root, content)) = ctx.elements() else {
    return;
  };
  let caret = ctx
    .state
    .try_update_value(|st| {
      let edit = if undo { st.undo.pop() } else { st.redo.pop() }?;
      let (from, to) = if undo {
        (&edit.inserted, &edit.removed)
      } else {
        (&edit.removed, &edit.inserted)
      };
      let at = edit.at.min(st.lines.len());
      let end = (at + from.len()).min(st.lines.len());
      let heights: Vec<f64> = to.iter().map(|line| st.estimate(line)).collect();
      st.lines.splice(at..end, to.iter().cloned());
      st.heights.splice(at..end, heights);
      st.measured.splice(at..end, std::iter::repeat_n(false, to.len()));
      st.priced = st.priced.min(at);
      if st.lines.is_empty() {
        st.lines.push(String::new());
        st.heights.push(st.row_h);
        st.measured.push(false);
      }
      let caret = if undo { edit.before } else { edit.after };
      if undo {
        st.redo.push(edit)
      } else {
        st.undo.push(edit)
      }
      st.emitted = st.text();
      st.last_edit_ms = 0.0;
      // The window is stale by construction — the document under it just
      // changed length — so make the next render redraw whatever it finds.
      st.last = st.last.min(st.lines.len());
      st.first = st.first.min(st.lines.len().saturating_sub(1));
      Some(Pos {
        line: caret.line.min(st.lines.len().saturating_sub(1)),
        col: caret.col,
      })
    })
    .flatten();
  let Some(caret) = caret else {
    return;
  };

  ctx.state.update_value(|st| {
    let view_h = f64::from(root.client_height());
    let scroll_top = f64::from(root.scroll_top());
    let (first, last) = window_for(st, scroll_top, view_h, Some(caret), 0.0);
    st.first = first;
    st.last = last;
  });
  draw(ctx, &root, &content, Some(caret), 0.0);
  ctx.state.with_value(|st| {
    ctx.blank.set(st.is_blank());
    ctx.value.set(st.emitted.clone());
  });
  scroll_caret_into_view(&root, &content);
  schedule_pricing(ctx);
}

/// Brings the caret's line into view after a jump the reader did not scroll to
/// themselves.
fn scroll_caret_into_view(root: &Element, content: &Element) {
  let Some(anchor) = selection().and_then(|selection| selection.anchor_node()) else {
    return;
  };
  if !content.contains(Some(&anchor)) {
    return;
  }
  let node = if anchor.node_type() == Node::TEXT_NODE {
    anchor.parent_element()
  } else {
    anchor.dyn_ref::<Element>().cloned()
  };
  let Some(node) = node else {
    return;
  };
  let line = node.get_bounding_client_rect();
  let view = root.get_bounding_client_rect();
  if line.top() < view.top() {
    root.set_scroll_top(root.scroll_top() + (line.top() - view.top()) as i32 - 8);
  } else if line.bottom() > view.bottom() {
    root.set_scroll_top(root.scroll_top() + (line.bottom() - view.bottom()) as i32 + 8);
  }
}

// -- Odds and ends ----------------------------------------------------------

fn selection() -> Option<Selection> {
  web_sys::window()?.get_selection().ok()?
}

fn now_ms() -> f64 {
  web_sys::window()
    .and_then(|window| window.performance())
    .map_or(0.0, |performance: web_sys::Performance| performance.now())
}

/// How many bytes the first `utf16` UTF-16 units of `s` take up. DOM offsets
/// count UTF-16 units and Rust counts bytes; Cyrillic — never mind emoji — is
/// where the two stop agreeing.
fn utf16_prefix_bytes(s: &str, utf16: usize) -> usize {
  let mut units = 0usize;
  for (at, ch) in s.char_indices() {
    if units >= utf16 {
      return at;
    }
    units += ch.len_utf16();
  }
  s.len()
}

/// The length of `s` in UTF-16 units, which is what a DOM offset is measured in.
fn utf16_len(s: &str) -> usize {
  s.chars().map(char::len_utf16).sum()
}

// -- A highlighter to start from --------------------------------------------

/// Markdown, one line at a time: headings, list bullets and quote marks,
/// emphasis, inline code, fence lines and links.
///
/// It sees a line and nothing around it, so a fence marks its own line but does
/// not colour what lies between two of them. That is the price of colouring only
/// what is on screen, and for a writing surface it is a fair one — the
/// alternative costs more than the editing does.
pub fn markdown_highlighter(line: &str) -> Vec<HighlightSpan> {
  const MARK: &str = "text-muted-foreground";
  const HEADING: &str = "font-semibold text-foreground";
  const CODE: &str = "text-primary";
  const EMPHASIS: &str = "italic";
  const STRONG: &str = "font-semibold";
  const LINK: &str = "text-primary underline underline-offset-2";

  let mut spans = Vec::new();
  let trimmed = line.trim_start();
  let lead = line.len() - trimmed.len();

  if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
    spans.push(HighlightSpan::new(0, line.len(), CODE));
    return spans;
  }
  if trimmed.starts_with('#') {
    let hashes = trimmed.len() - trimmed.trim_start_matches('#').len();
    if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
      spans.push(HighlightSpan::new(lead, lead + hashes, MARK));
      spans.push(HighlightSpan::new(lead + hashes, line.len(), HEADING));
      return spans;
    }
  }
  if trimmed.starts_with('>') {
    spans.push(HighlightSpan::new(lead, line.len(), MARK));
    return spans;
  }
  // A list marker, and only the marker.
  if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
    spans.push(HighlightSpan::new(lead, lead + 1, MARK));
  } else if let Some(dot) = trimmed.find(". ")
    && dot > 0
    && dot <= 3
    && trimmed[..dot].bytes().all(|b| b.is_ascii_digit())
  {
    spans.push(HighlightSpan::new(lead, lead + dot + 1, MARK));
  }

  // Inline runs. The delimiters are all ASCII, so a byte can be compared
  // against one safely — but stepping over a byte that is *not* one has to move
  // by a whole character, or the next slice lands inside a Cyrillic letter.
  let mut i = spans.last().map_or(0, |span| span.to);
  while i < line.len() {
    let rest = &line[i..];
    let (delim, class): (&str, &str) = if rest.starts_with("**") {
      ("**", STRONG)
    } else if rest.starts_with('`') {
      ("`", CODE)
    } else if rest.starts_with('*') || rest.starts_with('_') {
      (&rest[..1], EMPHASIS)
    } else if rest.starts_with('[') {
      // `[text](href)` — coloured whole when it closes, skipped when it does not.
      if let Some(close) = rest.find("](")
        && let Some(end) = rest[close..].find(')')
      {
        let to = i + close + end + 1;
        spans.push(HighlightSpan::new(i, to, LINK));
        i = to;
      } else {
        i = step(line, i);
      }
      continue;
    } else {
      i = step(line, i);
      continue;
    };
    let after = i + delim.len();
    if let Some(close) = line[after..].find(delim) {
      let to = after + close + delim.len();
      spans.push(HighlightSpan::new(i, to, class));
      i = to;
    } else {
      i = after;
    }
  }
  spans
}

/// One character on from `i`, never one byte into the middle of one.
fn step(line: &str, i: usize) -> usize {
  i + line[i..].chars().next().map_or(1, char::len_utf8)
}
