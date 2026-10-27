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
//! *replace*, and the highlighter sees one line at a time (a Markdown fence
//! marks its own line; it does not colour everything between two of them).
//!
//! Find there is, and it has to be: the browser's own `Ctrl+F` searches the DOM,
//! and the DOM here holds a few hundred lines around the viewport. On a document
//! long enough to need this editor at all, the browser's find reports most of it
//! as absent — which is worse than not offering find, because it answers.
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

use crate::syntax::{self, LangId, Token};

/// Classes on the scroll box the editor lives in. It takes its height from the
/// caller (`class="h-full"` in a flex column, `class="h-[60vh]"` otherwise):
/// unlike a textarea, something that renders a window has to be told how big the
/// window is.
const ROOT_CLASSES: &str = "relative w-full overflow-auto bg-transparent text-base outline-none md:text-sm";

/// What makes the scroll box look like a *field*: the border, the rounding, the
/// shadow and the focus ring.
///
/// Kept apart from [`ROOT_CLASSES`] because an editor is not always a field. One
/// that owns the screen — the whole area under a header, which is what a
/// document is opened into — has nothing to be bounded against, and a frame
/// drawn around the edge of the viewport reads as a box that ran out of room
/// rather than as a page. `bare` drops these; see [`SourceEditor`].
///
/// `dark:bg-input/30` belongs here rather than above for the same reason: it is
/// the tint that separates an input from the surface behind it, and a full-page
/// editor *is* the surface.
const ROOT_FIELD_CLASSES: &str = "rounded-md border border-input shadow-xs transition-[color,box-shadow] dark:bg-input/30 focus-within:border-ring focus-within:ring-ring/50 focus-within:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive";

/// Classes on the editable itself. `overflow-anchor: none` keeps the browser's
/// own scroll anchoring from fighting the padding that stands in for the lines
/// off screen — it would otherwise "correct" the scroll every time that padding
/// changed, which is every time the window moves.
const CONTENT_CLASSES: &str = "min-h-full whitespace-pre-wrap break-words outline-none [overflow-anchor:none]";

/// The breathing room above the first line and below the last, in pixels —
/// `py-2`, the same as every other field in the kit.
///
/// A number as well as a class because the padding is written by hand: what
/// stands in for the lines above the window is `padding-top`, and an inline
/// style overrules the class it is written over. Left to the class alone the
/// first line of a document sat flush against the top edge while the
/// placeholder — an ordinary element, wearing the class — sat two millimetres
/// below it, which is the same bug seen twice: text that starts at the very
/// edge, and a placeholder that does not sit where the text it stands in for
/// will appear.
///
/// Everything that converts a scroll position into a line therefore takes it
/// off first: the document's own coordinates start at the first line, and the
/// scroll box's start a little above it.
const PAD_Y: f64 = 8.0;

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

/// What a line inherits from the one above it.
///
/// A line-at-a-time highlighter is what makes colouring a window affordable, and
/// on its own it is also wrong: everything between two fences is code, and a
/// `*` in it is a multiplication rather than the start of a bold run. The
/// smallest thing that fixes it is for a line to be able to say what it *leaves
/// behind* — which is this, a few bytes carried from one line to the next.
///
/// The editor works these out from the top of the document with
/// [`Syntax::advance`], which never allocates, and keeps them so that an edit
/// only re-derives them from the line it touched downwards. Their meaning is the
/// syntax's own business; the editor only ever copies and compares them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Block {
  /// The markup's own state — 0 is "ordinary text", and what else means what is
  /// up to the syntax (a fence, a block comment, a front-matter header).
  pub kind: u8,
  /// How long the delimiter that opened it was, where a syntax needs to know: a
  /// fence is closed by a run of backticks at least as long as its own, so a
  /// shorter one inside it is content rather than the end.
  pub len: u8,
  /// The language the block is written in, when it is code — the word after the
  /// fence, looked up in the [registry](crate::syntax).
  pub lang: LangId,
  /// That language's own state: the block comment or the multiline string the
  /// line begins in the middle of.
  pub inner: u8,
}

/// How a document's markup is coloured.
///
/// Two functions rather than one, because the editor asks two questions of very
/// different sizes. `paint` is asked of the lines on screen — a hundred of them,
/// on every keystroke — and hands back the runs to colour. `advance` is asked of
/// *every* line above them, so that the window knows what it is in the middle
/// of; it allocates nothing and does no work beyond deciding what the line
/// leaves behind.
///
/// Spans must be sorted, must not overlap and must fall on character boundaries
/// — anything else is dropped rather than trusted.
#[derive(Clone, Copy)]
pub struct Syntax {
  /// The runs to colour in one line, given the state it starts in.
  pub paint: fn(&str, Block) -> Vec<HighlightSpan>,
  /// What that line leaves for the next one.
  pub advance: fn(&str, Block) -> Block,
}

/// Where the caret is, in the document's terms: a line, and a byte offset into
/// that line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Pos {
  line: usize,
  col: usize,
}

/// Where one search hit sits: the line, and the byte range inside it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Hit {
  line: usize,
  from: usize,
  to: usize,
}

/// Every occurrence of `needle`, in reading order.
///
/// Case-insensitive, and the offsets are byte offsets into the *original* line.
/// That distinction is the whole of the care here: lowercasing can change a
/// string's length — a Turkish dotted capital `İ` folds to two chars — so a
/// search run over a folded copy and reported against the original points past
/// its own match, by one byte here and by more in a line with several. The scan
/// therefore walks the original and folds only the candidate window.
///
/// Matches do not overlap: after a hit the scan resumes at its end, so `aa` in
/// `aaaa` finds two, not three — what a reader stepping through hits expects.
fn find_all(lines: &[String], needle: &str) -> Vec<Hit> {
  let mut hits = Vec::new();
  if needle.is_empty() {
    return hits;
  }
  let folded_needle = needle.to_lowercase();
  for (line, text) in lines.iter().enumerate() {
    let mut at = 0usize;
    while at < text.len() {
      if !text.is_char_boundary(at) {
        at += 1;
        continue;
      }
      let rest = &text[at..];
      // As many characters as it takes for the fold to reach the needle's
      // length — the window that could match, measured in folded bytes.
      let taken: String = rest
        .chars()
        .scan(0usize, |folded_len, c| {
          if *folded_len >= folded_needle.len() {
            return None;
          }
          *folded_len += c.to_lowercase().map(char::len_utf8).sum::<usize>();
          Some(c)
        })
        .collect();
      if taken.to_lowercase() == folded_needle {
        hits.push(Hit {
          line,
          from: at,
          to: at + taken.len(),
        });
        at += taken.len().max(1);
      } else {
        at += rest.chars().next().map_or(1, char::len_utf8);
      }
    }
  }
  hits
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
  /// What each line starts inside: `blocks[i]` is the state line `i` opens
  /// with, so `blocks[0]` is always the empty one and the vector is at most one
  /// longer than the document.
  ///
  /// Grown on demand down to the last line the window needs, and *truncated*
  /// back to the line an edit touched rather than recomputed — everything above
  /// an edit is unaffected by it, which is what keeps a keystroke from costing a
  /// walk over the whole document.
  blocks: Vec<Block>,
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
  /// Empty space kept below the last line, in pixels. Zero unless the caller
  /// asked for it; see [`overscroll_for`].
  overscroll: f64,
  /// Что за поиск нарисован сейчас: искомое и номер текущего совпадения.
  ///
  /// Отметки живут в нарисованных строках, а перерисовка по умолчанию трогает
  /// только края окна. Без этой памяти набранная в поиске буква не меняла бы
  /// ничего из уже нарисованного — то есть подсветка отставала бы на шаг.
  painted_find: Option<(String, usize)>,
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
      blocks: vec![Block::default()],
      priced: 0,
      pricing: false,
      price_batch: PRICE_BATCH,
      first: 0,
      last: 0,
      composing: false,
      painted: (0, 0),
      painted_find: None,
      last_scroll: 0.0,
      last_moved_ms: 0.0,
      heading: 0.0,
      last_max: 0.0,
      emitted: String::new(),
      undo: Vec::new(),
      redo: Vec::new(),
      last_edit_ms: 0.0,
      tab_escapes: false,
      overscroll: 0.0,
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
    self.is_moving() || (self.last_edit_ms > 0.0 && now_ms() - self.last_edit_ms <= SETTLE_AFTER.as_millis() as f64)
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

  /// Works the block states out down to and including line `upto`.
  ///
  /// Cheap by construction: `advance` allocates nothing, and the walk only ever
  /// covers ground that has not been walked since the last edit above it.
  fn ensure_blocks(&mut self, syntax: Option<Syntax>, upto: usize) {
    let Some(syntax) = syntax else { return };
    let upto = upto.min(self.lines.len());
    while self.blocks.len() <= upto && self.blocks.len() <= self.lines.len() {
      let line = self.blocks.len() - 1;
      let next = (syntax.advance)(&self.lines[line], self.blocks[line]);
      self.blocks.push(next);
    }
  }

  /// What line `line` starts inside, as far as it has been worked out.
  fn block_of(&self, line: usize) -> Block {
    self.blocks.get(line).copied().unwrap_or_default()
  }

  /// Forgets what was worked out below an edit at `line`. The state *at* that
  /// line is decided by everything above it, and survives.
  fn forget_blocks(&mut self, line: usize) {
    self.blocks.truncate(line + 1);
  }

  /// Re-derives the document from `text`, keeping nothing.
  fn load(&mut self, text: &str) {
    self.lines = text.split('\n').map(str::to_string).collect();
    if self.lines.is_empty() {
      self.lines.push(String::new());
    }
    self.heights = self.lines.iter().map(|line| self.estimate(line)).collect();
    self.measured = vec![false; self.lines.len()];
    self.blocks = vec![Block::default()];
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
  highlight: Option<Syntax>,
  value: RwSignal<String>,
  blank: RwSignal<bool>,
  /// The pending "colour the line being typed in" timer, if any.
  relight: StoredValue<Option<TimeoutHandle>>,
  /// The pending "the scrolling has stopped" timer, if any.
  settle: StoredValue<Option<TimeoutHandle>>,
  /// Whether the document may be scrolled past its last line.
  past_end: bool,
  /// Сколько сверху закрыто чужим — см. свойство `top_inset`.
  top_inset: Signal<f64>,
  /// What the find bar is looking for, or `None` while it is closed. The empty
  /// string is an *open* bar with nothing typed in it yet, which is a different
  /// thing from a closed one.
  needle: RwSignal<Option<String>>,
  hits: RwSignal<Vec<Hit>>,
  /// Which hit is the current one, as an index into `hits`.
  at: RwSignal<usize>,
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
/// * `highlight` — a [`Syntax`] run over each line as it is rendered;
///   [`markdown_syntax`] and [`typx_syntax`] are two. Left out, the text stays
///   plain.
/// * `line_numbers` — a gutter down the left, aligned to soft-wrapped lines.
/// * `tab_size` — how many spaces `Tab` inserts (two by default). `Tab` types
///   indentation here rather than moving focus, as it does in every editor;
///   `Shift+Tab` takes it back out.
///
/// ```ignore
/// let content = RwSignal::new(String::new());
/// view! {
///   <SourceEditor value=content class="h-full font-mono" highlight=markdown_syntax() />
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
  /// Drop the field chrome — border, rounding, shadow, focus ring — and render
  /// only the text. For an editor that *is* the page rather than a control on
  /// it; see [`ROOT_FIELD_CLASSES`].
  #[prop(optional)]
  bare: bool,
  #[prop(optional)] spellcheck: bool,
  #[prop(optional)] tab_size: Option<usize>,
  #[prop(optional)] highlight: Option<Syntax>,
  /// Keep a screenful of empty space below the last line, so the end of the
  /// document can be brought up to the top of the editor. See
  /// [`overscroll_for`]. Off by default: in a field that is one control among
  /// several, blank space under the text is a field that looks broken.
  #[prop(optional)]
  scroll_past_end: bool,
  /// `Ctrl+F` opens a find bar belonging to this editor.
  ///
  /// Opt-in, and for one reason either way. In a full-page editor the browser's
  /// own find is not merely redundant, it is *wrong*: only the window of lines
  /// around the viewport is in the DOM, so `Ctrl+F` searches a few hundred
  /// lines of a document and reports the rest as absent. In a small field it is
  /// the opposite — nobody searches a three-line note, and taking `Ctrl+F` away
  /// from the page to offer it would be the worse trade.
  #[prop(optional)]
  searchable: bool,
  /// Wording for the find bar, for an app that is not in English.
  #[prop(optional)]
  find_labels: FindLabels,
  /// Счётчик «открой поиск»: сдвинулся — строка поиска открывается.
  ///
  /// `Ctrl+F` редактор ловит сам, но пункт меню им воспользоваться не может:
  /// нажатие в нативной панели приезжает из обратного вызова JS, вне всякого
  /// представления, и дотянуться оттуда до поля внутри компонента нечем.
  /// Счётчик — то, чем до него дотягиваются: приложение его сдвигает, редактор
  /// открывается.
  ///
  /// Счётчик, а не флаг: искать просят и второй раз подряд, не закрыв строку, —
  /// флаг, уже поднятый, об этом не скажет, а счётчик скажет.
  #[prop(into, optional)]
  find_at: Signal<u32>,
  /// Сколько сверху редактора закрыто чужим: шапкой приложения, липкой
  /// панелью — всем, что лежит поверх его первых строк.
  ///
  /// Редактор этого не видит: закрывающий его элемент ему не родственник и в
  /// его измерения не попадает. А знать надо: место под концом документа
  /// отмеряется так, чтобы последняя строка встала наверх редактора, и если
  /// верх редактора под шапкой — последняя строка оказывается под шапкой тоже.
  /// Ровно на её высоту и не домотать.
  #[prop(into, optional)]
  top_inset: Signal<f64>,
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
    past_end: scroll_past_end,
    top_inset,
    needle: RwSignal::new(None),
    hits: RwSignal::new(Vec::new()),
    at: RwSignal::new(0),
  };
  let editable = !disabled && !readonly;
  let indent = StoredValue::new(" ".repeat(tab_size.unwrap_or(2).clamp(1, 8)));
  let has_placeholder = !placeholder.is_empty();

  // Просьба открыть поиск, пришедшая снаружи — из пункта меню (см. `find_at`).
  //
  // Первое значение пропускается: эффект выполняется и на монтировании, а
  // открывать поиск всякому, кто просто открыл документ, незачем.
  if searchable {
    let seen = StoredValue::new(None::<u32>);
    Effect::new(move |_| {
      let now = find_at.get();
      let first = seen.get_value().is_none();
      seen.set_value(Some(now));
      if !first {
        open_find(ctx);
      }
    });
  }

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
    // The browser's own find would search the window of lines in the DOM and
    // report the rest of the document as absent, so for a searchable editor it
    // is ours. `а` is what the `f` key types on a Russian layout, and `key()`
    // reports the letter, not the key.
    if searchable && ctrl && matches!(key.as_str(), "f" | "F" | "а" | "А") {
      ev.prevent_default();
      // И не пускать выше: приложение вправе объявить поиск пунктом меню с той
      // же горячей клавишей, и тогда её ловят двое. Открыться дважды поиск
      // переживёт, но строку, в которую уже что-то набрали, второе открытие
      // засеет заново — выделением, которое само же и поставило.
      ev.stop_propagation();
      open_find(ctx);
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
      // The find bar is the innermost thing `Escape` can close, so it closes
      // first — and the key goes no further, or the editor would also hand it
      // to whatever is listening on the window.
      if ctx.needle.get_untracked().is_some() {
        ev.prevent_default();
        close_find(ctx);
        return;
      }
      let composing = ctx.state.try_update_value(|st| {
        st.tab_escapes = true;
        st.composing
      });
      // While an IME is composing, `Escape` means "cancel what I am typing" and
      // nothing else — the platform's own default, which is left alone. What it
      // must not also do is travel on to whatever is listening for it on the
      // window and close the page out from under a half-typed word. Every
      // Cyrillic, Chinese or Japanese keyboard puts a composition in front of
      // the text, so this is not a corner case for the people it happens to.
      if composing == Some(true) {
        ev.stop_propagation();
      }
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
    // Рамка вокруг коробки прокрутки, и нужна она ровно для одного: строка
    // поиска лежит **поверх** редактора, а не внутри его прокрутки.
    //
    // Внутри она быть не может никак. Что лежит в прокрутке — часть документа:
    // липкость держит элемент у края, только пока его собственное место в
    // потоке не ушло за край, а фокус на поле браузер показывает, прокручивая к
    // нему, — то есть к началу документа, где это место и было. Снаружи ни того,
    // ни другого нет: полоса стоит от рамки, рамка не прокручивается, и
    // прокручивать к полосе некуда.
    //
    // Высоту и начертание рамке даёт вызывающий (`class`): высота нужна ей,
    // чтобы коробка прокрутки взяла её целиком, а начертание наследуется внутрь,
    // к тексту.
    <div data-slot="source-editor-frame" class=cn(&["relative", class.as_str()])>
    <div
      node_ref=ctx.root
      data-slot="source-editor"
      class=cn(&[ROOT_CLASSES, "h-full", if bare { "" } else { ROOT_FIELD_CLASSES }])
      aria-disabled=disabled.then_some("true")
      on:scroll=move |_| {
        render(ctx, false);
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
      {searchable
        .then(|| {
          let labels = StoredValue::new(find_labels);
          // The id is not knowable until the content element exists, so it is
          // taken on the first render rather than built here.
          let id = StoredValue::new(String::new());
          Effect::new(move |_| {
            if id.get_value().is_empty() {
              id.set_value(find_input_id(ctx));
            }
          });
          view! {
            <Show when=move || ctx.needle.get().is_some()>
              <div
                data-slot="source-editor-find"
                // Наложением поверх редактора: прижата к верхнему правому углу
                // рамки, не к документу. Рамка не прокручивается, поэтому
                // полоса стоит, сколько бы текста под ней ни проехало, и к
                // началу документа при появлении не уносит.
                //
                // Правый угол, а не весь верх: она накрывает конец первых строк
                // и ничего больше, а найденное показывается на трети высоты
                // сверху и под неё не попадает.
                //
                // Места в потоке не занимает вовсе — текст под ней не
                // сдвигается, и нумерация строк с подсказкой пустого поля, обе
                // отмеренные от верха редактора, остаются на своих местах.
                class="absolute top-0 right-0 z-10 flex w-fit items-center gap-1 rounded-b-md border border-t-0 border-border bg-popover p-1 shadow-md"
                // Полоса — поле ввода, и нажатие в ней не нажатие в документе.
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                  ev.stop_propagation();
                  match ev.key().as_str() {
                    "Escape" => {
                      ev.prevent_default();
                      close_find(ctx);
                    }
                    "Enter" => {
                      ev.prevent_default();
                      step_hit(ctx, !ev.shift_key());
                    }
                    _ => {}
                  }
                }
              >
                <input
                  id=move || id.get_value()
                  type="text"
                  class="h-8 w-44 rounded-sm bg-transparent px-2 text-sm outline-none placeholder:text-muted-foreground"
                  placeholder=labels.with_value(|l| l.placeholder.clone())
                  prop:value=move || ctx.needle.get().unwrap_or_default()
                  on:input=move |ev| {
                    ctx.needle.set(Some(event_target_value(&ev)));
                    research(ctx);
                    // Перерисовка до показа: отметки живут в нарисованных
                    // строках, и стёртая буква должна убрать подсветку даже
                    // тогда, когда показывать стало нечего.
                    render(ctx, true);
                    show_hit(ctx);
                  }
                />
                <span class="shrink-0 px-1 text-xs tabular-nums text-muted-foreground">
                  {move || {
                    let total = ctx.hits.with(Vec::len);
                    if ctx.needle.get().unwrap_or_default().is_empty() {
                      String::new()
                    } else if total == 0 {
                      labels.with_value(|l| l.nothing.clone())
                    } else {
                      format!("{}/{total}", ctx.at.get() + 1)
                    }
                  }}
                </span>
                <FindStep ctx=ctx forward=false label=labels.with_value(|l| l.previous.clone()) />
                <FindStep ctx=ctx forward=true label=labels.with_value(|l| l.next.clone()) />
                <button
                  type="button"
                  class="grid size-7 shrink-0 place-items-center rounded-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                  title=labels.with_value(|l| l.close.clone())
                  aria-label=labels.with_value(|l| l.close.clone())
                  on:click=move |_| close_find(ctx)
                >
                  "\u{d7}"
                </button>
              </div>
            </Show>
          }
        })}
    </div>
  }
}

// -- Find ------------------------------------------------------------------

/// Wording for the find bar. Defaults are English.
#[derive(Clone)]
pub struct FindLabels {
  pub placeholder: String,
  /// Shown in place of the count when the needle matches nothing.
  pub nothing: String,
  pub previous: String,
  pub next: String,
  pub close: String,
}

impl Default for FindLabels {
  fn default() -> Self {
    Self {
      placeholder: "Find".into(),
      nothing: "No matches".into(),
      previous: "Previous match".into(),
      next: "Next match".into(),
      close: "Close".into(),
    }
  }
}

/// The id the find bar's input carries, so opening the bar can focus it.
///
/// Derived from the content element's own id, which Leptos makes unique: a page
/// may hold two editors, and two inputs under one id is one input nobody can
/// reach.
fn find_input_id(ctx: Ctx) -> String {
  let at = ctx
    .content
    .get_untracked()
    .map(|el| el.unchecked_into::<Element>().id())
    .unwrap_or_default();
  format!("source-editor-find-{at}")
}

/// Opens the find bar, seeding it with whatever is selected.
///
/// Seeding, because what somebody has just selected is almost always what they
/// are about to look for, and every editor behaves this way. One line only: a
/// multi-line selection is not a word being hunted for, and dropping it into the
/// box would hide the box's own contents.
fn open_find(ctx: Ctx) {
  let selected = selection()
    .and_then(|s| s.to_string().as_string())
    .unwrap_or_default();
  let seed = (!selected.contains('\n') && !selected.is_empty())
    .then_some(selected)
    .or_else(|| ctx.needle.get_untracked())
    .unwrap_or_default();
  ctx.needle.set(Some(seed));
  research(ctx);
  let id = find_input_id(ctx);
  request_animation_frame(move || {
    if let Some(input) = document()
      .get_element_by_id(&id)
      .and_then(|el| el.dyn_into::<HtmlElement>().ok())
    {
      focus_without_scrolling(&input);
    }
  });
}

/// Closes the bar and puts the caret back in the document.
fn close_find(ctx: Ctx) {
  ctx.needle.set(None);
  ctx.hits.set(Vec::new());
  // Отметки нарисованы в строках и сами не уйдут.
  render(ctx, true);
  if let Some(content) = ctx.content.get_untracked() {
    let _ = content.unchecked_into::<HtmlElement>().focus();
  }
}

/// Re-runs the search and keeps the reader on the hit nearest where they were.
///
/// Nearest rather than the first: the search re-runs on every keystroke, and
/// restarting at the top each time would walk somebody typing a longer word
/// backwards through their own document.
fn research(ctx: Ctx) {
  let Some(needle) = ctx.needle.get_untracked() else {
    ctx.hits.set(Vec::new());
    return;
  };
  let was = ctx
    .hits
    .with_untracked(|hits| hits.get(ctx.at.get_untracked()).copied());
  let hits = ctx.state.with_value(|st| find_all(&st.lines, &needle));
  let at = match was {
    Some(was) => hits
      .iter()
      .position(|hit| (hit.line, hit.from) >= (was.line, was.from))
      .unwrap_or(0),
    None => 0,
  };
  ctx.hits.set(hits);
  ctx.at.set(at);
}

/// Moves to the current hit and shows it: scrolls its line into view, renders
/// the window that now covers it, and selects the match itself.
///
/// The selection waits a frame because the line may not be in the DOM yet — the
/// window is painted only once the scroll has been applied, and until then there
/// is nothing to select.
fn show_hit(ctx: Ctx) {
  let Some(hit) = ctx
    .hits
    .with_untracked(|hits| hits.get(ctx.at.get_untracked()).copied())
  else {
    return;
  };
  let Some((root, _)) = ctx.elements() else {
    return;
  };
  // Видимая высота, а не вся: треть закрытого шапкой — не треть экрана, и
  // найденное на ней оказалось бы под шапкой.
  let view_h = viewport_of(ctx, &root);
  // A third of the way down rather than at the very top: a hit on the first
  // visible row shows what comes after it and nothing of what comes before, and
  // the line before a match is usually what says whether it is the one.
  let target = ctx
    .state
    .with_value(|st| st.offset_of(hit.line) + PAD_Y - view_h / 3.0 - ctx.top_inset.get_untracked())
    .max(0.0);
  set_scroll(ctx, &root, target);
  render(ctx, true);
  request_animation_frame(move || {
    let Some((_, content)) = ctx.elements() else {
      return;
    };
    ctx.state.with_value(|st| {
      select_range(
        &content,
        st.first,
        Pos {
          line: hit.line,
          col: hit.from,
        },
        Pos {
          line: hit.line,
          col: hit.to,
        },
      )
    });
    // Выделив совпадение, вернуть ввод в строку поиска.
    //
    // Выделение в `contenteditable` — это и перевод фокуса в него: браузер не
    // умеет выделять в поле, в котором не стоит курсор. А строка поиска ищет на
    // каждом нажатии, то есть выделяет на каждом нажатии, — и второй набранный
    // символ уходил уже в документ. Выглядело это так, будто курсор сам уехал
    // из поля в текст.
    //
    // Браузер продолжает рисовать выделение и потеряв фокус, приглушённым, так
    // что найденное видно, а печатается по-прежнему в поле.
    reclaim_find_focus(ctx);
  });
}

/// Ставит курсор в поле, не прокручивая к нему.
///
/// Браузер показывает то, что получило фокус, — прокручивая к нему ближайшего
/// прокручиваемого предка. Полоса поиска лежит вне прокрутки редактора, так
/// что его текст от этого не уедет, но уехать может страница, если редактор на
/// ней не ограничен по высоте. Показывать её незачем: полоса и так на виду.
fn focus_without_scrolling(el: &HtmlElement) {
  let opts = web_sys::FocusOptions::new();
  opts.set_prevent_scroll(true);
  let _ = el.focus_with_options(&opts);
}

/// Возвращает ввод в строку поиска, если она открыта.
fn reclaim_find_focus(ctx: Ctx) {
  if ctx.needle.get_untracked().is_none() {
    return;
  }
  let id = find_input_id(ctx);
  if let Some(input) = document()
    .get_element_by_id(&id)
    .and_then(|el| el.dyn_into::<HtmlElement>().ok())
  {
    focus_without_scrolling(&input);
  }
}

/// Steps to the next hit (or the previous one), wrapping around.
fn step_hit(ctx: Ctx, forward: bool) {
  let count = ctx.hits.with_untracked(Vec::len);
  if count == 0 {
    return;
  }
  ctx.at.update(|at| {
    *at = match forward {
      true => (*at + 1) % count,
      false => (*at + count - 1) % count,
    };
  });
  show_hit(ctx);
}

/// Selects `from..to` inside one rendered line.
///
/// Both ends go through the same walk [`set_dom_caret`] uses for the caret, so a
/// match falling inside a highlighted span — which is several text nodes, not
/// one — is selected across them.
fn select_range(content: &Element, first: usize, from: Pos, to: Pos) -> Option<()> {
  let doc = content.owner_document()?;
  let range = doc.create_range().ok()?;
  let start = content.children().item(from.line.checked_sub(first)? as u32)?;
  let end = content.children().item(to.line.checked_sub(first)? as u32)?;

  let mut acc = 0usize;
  if !place_in(start.unchecked_ref(), from.col, &mut acc, &range, true) {
    range.set_start(start.unchecked_ref::<Node>(), 0).ok()?;
  }
  let mut acc = 0usize;
  if !place_in(end.unchecked_ref(), to.col, &mut acc, &range, false) {
    range.set_end_after(end.unchecked_ref::<Node>()).ok()?;
  }
  let selection = selection()?;
  selection.remove_all_ranges().ok()?;
  selection.add_range(&range).ok()?;
  Some(())
}

/// One of the find bar's two step buttons.
///
/// A component rather than two copies of the markup: the arrow is the only
/// thing that differs, and a chevron written twice is a chevron that goes out of
/// step once.
#[component]
fn FindStep(ctx: Ctx, forward: bool, label: String) -> impl IntoView {
  view! {
    <button
      type="button"
      class="grid size-7 shrink-0 place-items-center rounded-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground disabled:opacity-40"
      title=label.clone()
      aria-label=label
      prop:disabled=move || ctx.hits.with(Vec::is_empty)
      on:click=move |_| step_hit(ctx, forward)
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="size-4"
      >
        <path d=if forward { "m6 9 6 6 6-6" } else { "m18 15-6-6-6 6" } />
      </svg>
    </button>
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
    // Before the window is worked out: the space below the last line is part of
    // how far the document can be scrolled, and `pad` writes whatever is here.
    // Не `view_h`: место под последней строкой меряется окном, а не
    // содержимым, и `client_height` говорит о первом только пока редактор
    // кем-то ограничен (см. [`viewport_of`]).
    st.overscroll = overscroll_for(st, viewport_of(ctx, &root), ctx.past_end);
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
  let find = Find {
    hits: ctx.hits.get_untracked(),
    at: ctx.at.get_untracked(),
    key: ctx.needle.get_untracked().map(|needle| (needle, ctx.at.get_untracked())),
  };
  let corrected = ctx.state.try_update_value(|st| {
    let anchor = st.line_at(scroll_top - PAD_Y);
    let was = st.offset_of(anchor);
    paint(st, content, ctx.highlight, &find);
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
      if let Some(div) = make_line(&doc, &st.lines[line], None, Block::default(), &[]) {
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
    let anchor = st.line_at(scroll_top - PAD_Y);
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
  // Без места под концом документа: оно часть `scroll_height`, но не часть
  // документа. Появившись впервые — а появляется оно на первой же отрисовке,
  // где его попросили, — оно выглядело отсюда как разом доросший документ, и
  // читателя уносило в самый низ. Ровно это и делал поиск: открываясь, он
  // вызывал отрисовку, та впервые писала отступ, и вместо найденного человек
  // оказывался в конце текста.
  //
  // Расти должен текст, а не наша собственная подложка под ним.
  let overscroll = ctx.state.with_value(|st| st.overscroll);
  let max = f64::from(root.scroll_height() - root.client_height()) - overscroll;
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
  let highlight = ctx.highlight;

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
    let inserted = &read[head..read.len() - tail];
    let touched = first + head;
    record(st, touched, removed, inserted.to_vec(), caret);
    // A changed line count means the browser merged or split our line divs —
    // Enter drops a bare `\n` into one of them — so the DOM has to be rebuilt
    // into one div per line before anything measures it again.
    let mut restructured = read.len() != last - first;
    // Only the lines the edit touched lose their heights, and that "only" is the
    // whole of it. The rows either side of it still hold the same text, at the
    // same width, in the same DOM nodes, so what they were measured at is a fact
    // — and replacing a measurement with a guess is not a neutral thing to do
    // here, because the guesses are what the scroll position gets corrected
    // against.
    //
    // Splicing the whole window is what this used to do, and it is what a
    // keystroke in a long document was felt as. Every line on screen went back to
    // an estimate; every estimate is a pixel or two out; a fifth of a second
    // later the walk down the document laid them all out again and put the real
    // heights back — and the part of that correction landing *above* the
    // viewport is taken out of the scroll position, because a line that changes
    // height above the reader has moved the reader. So a writer at the end of the
    // piece typed a character, arrived at the bottom, and a moment later was
    // pulled a couple of pixels back up; the next character put them at the
    // bottom again, and the one after that pulled them up again. Nothing was
    // wrong with the correction. There was nothing to correct.
    let changed = touched..last - tail;
    let heights: Vec<f64> = inserted.iter().map(|line| st.estimate(line)).collect();
    st.lines.splice(changed.clone(), inserted.iter().cloned());
    st.heights.splice(changed.clone(), heights);
    st.measured.splice(changed, std::iter::repeat_n(false, inserted.len()));
    // A splice moves every line after it along, so pricing's place in the
    // document is no longer where it thinks it is — and a paste can drop a
    // thousand unpriced lines in at once. It goes back to the edit and walks
    // down again; the lines it already knows cost it a bounds check each.
    st.priced = st.priced.min(touched);
    st.first = first;
    st.last = first + read.len();
    // Typing the third backtick of a fence turns the rest of the document into
    // code, and deleting it turns it back into prose: an edit changes what the
    // lines *below* it are inside of, and when it does, colouring only the line
    // that was typed in is colouring one line out of a page that all changed.
    // Comparing what the touched line used to leave behind with what it leaves
    // behind now is what tells the two apart — and for ordinary typing, which is
    // every keystroke that is not a delimiter, the two are equal and nothing
    // else is repainted.
    let was = st.blocks.get(touched + 1).copied();
    st.forget_blocks(touched);
    st.ensure_blocks(highlight, touched + 1);
    if was.is_some() && was != st.blocks.get(touched + 1).copied() {
      restructured = true;
    }
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
fn window_for(st: &State, scroll_top: f64, view_h: f64, caret: Option<Pos>, going: f64) -> (usize, usize) {
  // Into the document's own coordinates, which start at the first line rather
  // than at the top of the scroll box; see [`PAD_Y`].
  let scroll_top = scroll_top - PAD_Y;
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
fn paint(st: &mut State, content: &Element, highlight: Option<Syntax>, find: &Find) {
  let Some(doc) = content.owner_document() else {
    return;
  };
  let (first, last) = (st.first, st.last.min(st.lines.len()));
  // Nothing can be coloured before it is known what it is inside of.
  st.ensure_blocks(highlight, last);
  let (was_first, was_last) = st.painted;
  // Поиск изменился — отметки стоят не там, и края окна тут не помогут:
  // перерисовывать надо всё, что на экране.
  let same_find = st.painted_find.as_ref() == find.key.as_ref();
  st.painted_find = find.key.clone();
  let intact = same_find
    && was_last > was_first
    && first < was_last
    && last > was_first
    && content.children().length() as usize == was_last - was_first;

  if !intact {
    let fragment = doc.create_document_fragment();
    for line in first.min(st.lines.len())..last {
      if let Some(div) = make_line(&doc, &st.lines[line], highlight, st.block_of(line), &find.on(line)) {
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
    if let Some(div) = make_line(&doc, &st.lines[line], highlight, st.block_of(line), &find.on(line)) {
      let _ = content.insert_before(&div, content.first_child().as_ref());
    }
  }
  for line in was_last.max(first)..last {
    if let Some(div) = make_line(&doc, &st.lines[line], highlight, st.block_of(line), &find.on(line)) {
      let _ = content.append_child(&div);
    }
  }
  st.painted = (first, last);
}

/// Найденное — так, как его видит отрисовка: отметки по строкам.
///
/// Снимок, а не сигнал: отрисовка идёт в середине правки состояния, и читать
/// из неё реактивное значит подписать её на себя же.
struct Find {
  hits: Vec<Hit>,
  /// Номер совпадения, на котором стоим, в `hits`.
  at: usize,
  /// Чем нарисовано: искомое и номер текущего. Ничего не ищут — `None`.
  key: Option<(String, usize)>,
}

impl Find {
  /// Отметки на одной строке, слева направо.
  fn on(&self, line: usize) -> Vec<(usize, usize, &'static str)> {
    self
      .hits
      .iter()
      .enumerate()
      .filter(|(_, hit)| hit.line == line)
      .map(|(i, hit)| {
        let class = match i == self.at {
          true => HIT_CURRENT_CLASS,
          false => HIT_CLASS,
        };
        (hit.from, hit.to, class)
      })
      .collect()
  }
}

/// Как помечается найденное: подсветка на каждом совпадении.
///
/// Выделением браузера обойтись нельзя. Оно одно на документ — значит показало
/// бы одно совпадение из двадцати, — и гаснет, стоит фокусу уйти в строку
/// поиска, то есть ровно тогда, когда в неё и печатают. Отметка — часть
/// нарисованной строки и живёт по тем же правилам, что и раскраска синтаксиса.
const HIT_CLASS: &str = "rounded-xs bg-primary/25";

/// То же для совпадения, на котором стоим: его видно среди остальных.
const HIT_CURRENT_CLASS: &str = "rounded-xs bg-primary/60 text-primary-foreground";

/// Накладывает отметки найденного на раскраску синтаксиса.
///
/// Отметка сильнее цвета: внутри совпадения раскраска уступает ей целиком, а
/// не смешивается с ней. Смешать было бы нечем — класс у отрезка один, — да и
/// читается подсветка тем хуже, чем больше под ней всего.
///
/// На вход идут отрезки, отсортированные и непересекающиеся (`find_all` даёт
/// именно такие), на выходе — тоже: [`make_line`] других не принимает.
fn with_hits(painted: Vec<HighlightSpan>, marks: &[(usize, usize, &'static str)]) -> Vec<HighlightSpan> {
  if marks.is_empty() {
    return painted;
  }
  let mut out: Vec<HighlightSpan> = Vec::with_capacity(painted.len() + marks.len());
  for span in painted {
    let mut at = span.from;
    for &(from, to, _) in marks {
      if to <= at || from >= span.to {
        continue;
      }
      if from > at {
        out.push(HighlightSpan { from: at, to: from, class: span.class });
      }
      at = at.max(to);
    }
    if at < span.to {
      out.push(HighlightSpan { from: at, to: span.to, class: span.class });
    }
  }
  out.extend(marks.iter().map(|&(from, to, class)| HighlightSpan { from, to, class }));
  out.sort_by_key(|span| (span.from, span.to));
  out
}

/// One line's worth of DOM.
///
/// An empty line gets a `<br>`: an empty block is zero pixels tall, and a line
/// nobody can see is a line nobody can click into.
fn make_line(
  doc: &Document,
  text: &str,
  highlight: Option<Syntax>,
  block: Block,
  marks: &[(usize, usize, &'static str)],
) -> Option<Element> {
  let div = doc.create_element("div").ok()?;
  if text.is_empty() {
    let br = doc.create_element("br").ok()?;
    let _ = div.append_child(&br);
    return Some(div);
  }
  // Раскраски может не быть, а отметки — быть: искать можно и в простом тексте.
  let painted = match highlight {
    Some(highlight) => (highlight.paint)(text, block),
    None if marks.is_empty() => {
      div.set_text_content(Some(text));
      return Some(div);
    }
    None => Vec::new(),
  };
  let mut at = 0usize;
  for span in with_hits(painted, marks) {
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
/// How much empty space to keep below the last line, given the viewport.
///
/// A viewport less one row and the field's own top padding: the last line comes
/// all the way to the top of the editor and no further. That is the whole of
/// what scrolling past the end is for — a writer finishing a piece otherwise
/// does it on the bottom edge of the screen, with the line they are typing at
/// the very limit of where a line can be. One row rather than none, so the end
/// never leaves the screen.
///
/// [`PAD_Y`] because the document does not start at the top of the scroll box:
/// the field's breathing room is above it, and the padding below has to give
/// that back or the last line stops just short of the top — under whatever is
/// drawn there. `view_h` arrives already less whatever the app says covers it
/// (see the `top_inset` prop), so the same arithmetic answers both.
///
/// A screenful, in other words, and never more: this is a property of the
/// window, not of the document. A thousand-line document gets exactly what a
/// ten-line one gets, because the two are scrolled to the same place — the last
/// line, at the top.
///
/// Zero while the document still fits: there is nothing to scroll past, and
/// padding a short note out to a screenful of blank would be inventing a
/// document that is not there.
fn overscroll_for(st: &State, view_h: f64, wanted: bool) -> f64 {
  if !wanted || st.total() + PAD_Y * 2.0 <= view_h {
    return 0.0;
  }
  (view_h - st.row_h - PAD_Y).max(0.0)
}

/// The editor's own height, as far as it can be believed.
///
/// `client_height` is the viewport only while something above actually bounds
/// the editor. The root carries `overflow-auto` and no height of its own — that
/// is the caller's to give — and a caller who does not give it gets a box that
/// grows with its text instead of scrolling inside it. `client_height` is then
/// the height of the *document*, and the space kept below the last line comes
/// out the same size: a thousand lines of text, a thousand lines of blank under
/// it. Which is exactly what it looked like.
///
/// No editor is taller than the window it is drawn in, so the window is the
/// bound. It cannot repair the layout — an unbounded editor is still a page
/// that scrolls as a whole — but it keeps one misplaced class from turning the
/// end of a document into a field of nothing.
fn viewport_of(ctx: Ctx, root: &Element) -> f64 {
  let measured = f64::from(root.client_height());
  let window = web_sys::window()
    .and_then(|w| w.inner_height().ok())
    .and_then(|v| v.as_f64())
    .filter(|h| *h > 0.0);
  let bounded = match window {
    Some(limit) => measured.min(limit),
    None => measured,
  };
  // И без того, что приложение накрыло сверху: под шапкой места нет, сколько
  // бы его ни было у редактора.
  (bounded - ctx.top_inset.get_untracked()).max(0.0)
}

fn pad(st: &State, content: &Element) {
  let Some(el) = content.dyn_ref::<HtmlElement>() else {
    return;
  };
  // Plus the field's own breathing room at either end — see [`PAD_Y`]. Written
  // into the same property the lines off screen stand in, because a browser has
  // only one `padding-top` and the inline one wins.
  let above = st.offset_of(st.first) + PAD_Y;
  let below = st.total() - st.offset_of(st.last.min(st.lines.len())) + PAD_Y + st.overscroll;
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
  let find = Find {
    hits: ctx.hits.get_untracked(),
    at: ctx.at.get_untracked(),
    key: ctx.needle.get_untracked().map(|needle| (needle, ctx.at.get_untracked())),
  };
  let replaced = ctx.state.try_update_value(|st| {
    st.ensure_blocks(Some(highlight), caret.line);
    let index = caret.line.checked_sub(st.first)?;
    let old = content.children().item(index as u32)?;
    let text = st.lines.get(caret.line)?;
    // Отметки на этой строке — те же, что нарисованы сейчас: правка строки не
    // ищет заново, а стереть найденное на ней значило бы потерять подсветку до
    // следующего прохода.
    let marks = find.on(caret.line);
    let new = make_line(&doc, text, Some(highlight), st.block_of(caret.line), &marks)?;
    content.replace_child(&new, &old).ok()?;
    Some(())
  });
  if replaced.flatten().is_some() {
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
  if !place_in(child.unchecked_ref(), pos.col, &mut acc, &range, true) {
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
fn place_in(node: &Node, col: usize, acc: &mut usize, range: &Range, start: bool) -> bool {
  if node.node_type() == Node::TEXT_NODE {
    let text = node.node_value().unwrap_or_default();
    if col <= *acc + text.len() {
      let within = (col - *acc).min(text.len());
      if !text.is_char_boundary(within) {
        return false;
      }
      let offset = utf16_len(&text[..within]) as u32;
      return match start {
        true => range.set_start(node, offset).is_ok(),
        false => range.set_end(node, offset).is_ok(),
      };
    }
    *acc += text.len();
    return false;
  }
  let mut child = node.first_child();
  while let Some(node) = child {
    if place_in(&node, col, acc, range, start) {
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
  if !place_in(child.unchecked_ref(), 0, &mut from_acc, &range, true)
    || !place_in(child.unchecked_ref(), strip, &mut to_acc, &end, true)
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
      st.forget_blocks(at);
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

// -- The syntaxes that ship here ---------------------------------------------

/// Markup that is not content: a heading's `=`, a list's bullet, a fence.
const MARK: &str = "text-muted-foreground";
const HEADING: &str = "font-semibold text-foreground";
const CODE: &str = "text-primary";
const EMPHASIS: &str = "italic";
const STRONG: &str = "font-semibold";
const LINK: &str = "text-primary underline underline-offset-2";

/// Inside a fence opened with backticks; `len` is how many of them opened it.
const IN_FENCE: u8 = 1;
/// The same, with tildes. Kept apart so a `~~~` block is not closed by a ```` ``` ````.
const IN_TILDE: u8 = 2;
/// Inside a `/* … */` comment; `len` is how deep the nesting is. typx only —
/// Markdown has no such thing.
const IN_COMMENT: u8 = 3;

/// The fence a line opens, if it opens one.
///
/// The word after it names the language the block is written in, and that word
/// is looked up in the [registry](crate::syntax) here — once, when the line is
/// scanned — so that colouring a line inside the block is a lookup by number.
/// A word nobody registered gives [`LangId::NONE`], and the block is left
/// uncoloured rather than coloured as prose, which is the whole point: the text
/// between two fences is code, and the `*` in it is a multiplication.
fn opens_fence(line: &str) -> Option<Block> {
  let trimmed = line.trim_start();
  let mark = trimmed.chars().next().filter(|ch| *ch == '`' || *ch == '~')?;
  let run = trimmed.chars().take_while(|ch| *ch == mark).count();
  if run < 3 {
    return None;
  }
  let info = trimmed[run..].trim();
  // A backtick fence's info string cannot itself contain a backtick — that is
  // what keeps `` `a` and `b` `` from reading as an opening fence.
  if mark == '`' && info.contains('`') {
    return None;
  }
  let word = info
    .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == ':' || ch == ';')
    .next()
    .unwrap_or_default()
    .trim_matches(|ch: char| ch == '{' || ch == '}' || ch == '.');
  Some(Block {
    kind: if mark == '`' { IN_FENCE } else { IN_TILDE },
    len: run.min(u8::MAX as usize) as u8,
    lang: syntax::lang_id(word),
    inner: 0,
  })
}

/// Whether this line closes the fence `block` is inside: a run of the same
/// character, at least as long as the one that opened it, and nothing after it.
fn closes_fence(line: &str, block: Block) -> bool {
  let mark = if block.kind == IN_TILDE { '~' } else { '`' };
  let trimmed = line.trim_start();
  let run = trimmed.chars().take_while(|ch| *ch == mark).count();
  run >= usize::from(block.len) && trimmed[run..].trim().is_empty()
}

/// One line of a fenced block: the closing fence, or code in whatever language
/// the fence named.
fn fenced(line: &str, block: Block, out: Option<&mut Vec<HighlightSpan>>) -> Block {
  if closes_fence(line, block) {
    if let Some(out) = out {
      out.push(HighlightSpan::new(0, line.len(), CODE));
    }
    return Block::default();
  }
  Block {
    inner: syntax::scan(line, block.lang, block.inner, 0, out),
    ..block
  }
}

/// Markdown: headings, list bullets and quote marks, emphasis, inline code,
/// links — and fenced blocks, coloured as the language they name.
///
/// Inside a fence the prose rules are off, which is the point of carrying a
/// [`Block`] from line to line at all: a `*` in a shell command is not the start
/// of a bold run, and an underscore in a Python name is not an emphasis that
/// swallows the rest of the paragraph.
pub fn markdown_syntax() -> Syntax {
  Syntax {
    paint: |line, block| {
      let mut out = Vec::new();
      markdown_scan(line, block, Some(&mut out));
      out
    },
    advance: |line, block| markdown_scan(line, block, None),
  }
}

/// The one walk both halves of [`markdown_syntax`] are made of: it colours the
/// line when it is given somewhere to put the colours, and either way says what
/// the line leaves behind.
fn markdown_scan(line: &str, block: Block, out: Option<&mut Vec<HighlightSpan>>) -> Block {
  if matches!(block.kind, IN_FENCE | IN_TILDE) {
    return fenced(line, block, out);
  }
  if let Some(fence) = opens_fence(line) {
    if let Some(out) = out {
      out.push(HighlightSpan::new(0, line.len(), CODE));
    }
    return fence;
  }
  if let Some(out) = out {
    out.extend(markdown_inline(line));
  }
  Block::default()
}

/// One line of Markdown prose, one line at a time — the part of the highlighter
/// that never needed to know what came before it.
fn markdown_inline(line: &str) -> Vec<HighlightSpan> {
  let mut spans = Vec::new();
  let trimmed = line.trim_start();
  let lead = line.len() - trimmed.len();

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

/// typx (Typst-flavoured markup): headings, lists, emphasis, inline raw and
/// math, `#`-code, labels and references, comments — and raw blocks, coloured as
/// the language they name.
///
/// The differences from Markdown are the ones that matter for a document being
/// typed: a heading is `=` rather than `#`, `#` opens *code* rather than a
/// heading, and comments (`//`, and `/* … */` nested to any depth) run through
/// the markup rather than only inside code.
pub fn typx_syntax() -> Syntax {
  Syntax {
    paint: |line, block| {
      let mut out = Vec::new();
      typx_scan(line, block, Some(&mut out));
      out
    },
    advance: |line, block| typx_scan(line, block, None),
  }
}

/// One walk over a line of typx: colours it when asked, and always says what it
/// leaves behind — an open raw block, or a comment several levels deep.
fn typx_scan(line: &str, block: Block, mut out: Option<&mut Vec<HighlightSpan>>) -> Block {
  if matches!(block.kind, IN_FENCE | IN_TILDE) {
    return fenced(line, block, out);
  }
  let mut push = |from: usize, to: usize, class: &'static str| {
    if let Some(out) = out.as_deref_mut()
      && to > from
    {
      out.push(HighlightSpan::new(from, to, class));
    }
  };

  let mut i = 0usize;

  // A comment carried in from the line above runs until it closes — and it may
  // close into another one, which is why the depth is counted rather than
  // flagged.
  let carried = if block.kind == IN_COMMENT {
    usize::from(block.len)
  } else {
    0
  };
  if carried > 0 {
    let (at, left) = comment_run(line, 0, carried);
    push(0, at, Token::Comment.class());
    if left > 0 {
      return in_comment(left);
    }
    i = at;
  }

  if i == 0 {
    if let Some(fence) = opens_fence(line) {
      push(0, line.len(), CODE);
      return fence;
    }
    let trimmed = line.trim_start();
    let lead = line.len() - trimmed.len();
    // `= Заголовок`, `== Подзаголовок` — Typst's headings, and the one place
    // where a leading run of a character is markup rather than an operator.
    if trimmed.starts_with('=') {
      let marks = trimmed.len() - trimmed.trim_start_matches('=').len();
      if (1..=6).contains(&marks) && trimmed[marks..].starts_with(' ') {
        push(lead, lead + marks, MARK);
        push(lead + marks, line.len(), HEADING);
        return Block::default();
      }
    }
    // A list: `-`, `+`, and `/ term:` for a definition.
    if trimmed.starts_with("- ") || trimmed.starts_with("+ ") || trimmed.starts_with("/ ") {
      push(lead, lead + 1, MARK);
      i = lead + 1;
    } else if let Some(dot) = trimmed.find(". ")
      && dot > 0
      && dot <= 3
      && trimmed[..dot].bytes().all(|b| b.is_ascii_digit())
    {
      push(lead, lead + dot + 1, MARK);
      i = lead + dot + 1;
    }
  }

  while i < line.len() {
    let rest = &line[i..];

    // `\*` is a literal asterisk, and neither half of it is markup.
    if rest.starts_with('\\') {
      i = step(line, step(line, i));
      continue;
    }
    if rest.starts_with("/*") {
      // Past the opener, already counted — `comment_run` counts the ones it
      // finds, and counting this one twice is a comment that never closes.
      let (at, left) = comment_run(line, i + 2, 1);
      push(i, at, Token::Comment.class());
      if left > 0 {
        return in_comment(left);
      }
      i = at;
      continue;
    }
    if rest.starts_with("//") {
      push(i, line.len(), Token::Comment.class());
      return Block::default();
    }
    // `#` opens code, and the rest of the line is read as code: `#let x = 2`,
    // `#figure(image("plan.svg"))`. A heading it is not — that is `=` here.
    if rest.starts_with('#') {
      let word_end = ident_end(line, i + 1);
      let word = &line[i + 1..word_end];
      let class = if word.is_empty() {
        MARK
      } else if syntax::lang_def(syntax::lang_id("typst")).is_some_and(|def| def.keywords.iter().any(|k| k == word)) {
        Token::Keyword.class()
      } else {
        Token::Function.class()
      };
      push(i, word_end.max(i + 1), class);
      i = word_end.max(i + 1);
      // Whatever follows on this line is code: strings, numbers and the
      // language's own words, from the same scanner every fenced block uses.
      let typst = syntax::lang_id("typst");
      if let Some(out) = out.as_deref_mut() {
        syntax::scan(&line[i..], typst, syntax::IN_NOTHING, i, Some(out));
      }
      return Block::default();
    }
    // Inline raw, and math — both are code set in the middle of prose, and both
    // read better in the colour code is set in.
    let inline = if rest.starts_with('`') {
      Some(("`", CODE))
    } else if rest.starts_with('$') {
      Some(("$", CODE))
    } else if rest.starts_with('*') {
      Some(("*", STRONG))
    } else if rest.starts_with('_') {
      Some(("_", EMPHASIS))
    } else {
      None
    };
    if let Some((delim, class)) = inline {
      let after = i + delim.len();
      match line[after..].find(delim) {
        Some(close) => {
          let to = after + close + delim.len();
          push(i, to, class);
          i = to;
        }
        None => i = after,
      }
      continue;
    }
    // `<label>` and `@reference` — the two ways a typx document points at
    // itself.
    if rest.starts_with('<')
      && let Some(close) = rest.find('>')
      && rest[1..close]
        .chars()
        .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == ':')
    {
      push(i, i + close + 1, LINK);
      i += close + 1;
      continue;
    }
    if rest.starts_with('@') {
      let to = ident_end(line, i + 1);
      if to > i + 1 {
        push(i, to, LINK);
        i = to;
        continue;
      }
    }
    i = step(line, i);
  }
  Block::default()
}

/// Where a `/* … */` run ends on this line, and how deep it still is when the
/// line runs out: comments nest in Typst, so `/* a /* b */ */` is one comment
/// and not two thirds of one.
fn comment_run(line: &str, from: usize, mut depth: usize) -> (usize, usize) {
  let mut i = from;
  while i < line.len() {
    let rest = &line[i..];
    if rest.starts_with("/*") {
      depth += 1;
      i += 2;
      continue;
    }
    if rest.starts_with("*/") {
      depth = depth.saturating_sub(1);
      i += 2;
      if depth == 0 {
        return (i, 0);
      }
      continue;
    }
    i = step(line, i);
  }
  (line.len(), depth)
}

/// The state a line ends in when it is `depth` comments deep.
fn in_comment(depth: usize) -> Block {
  Block {
    kind: IN_COMMENT,
    len: depth.min(u8::MAX as usize) as u8,
    ..Block::default()
  }
}

/// Where the identifier starting at `from` ends — one past its last character,
/// or `from` itself if there is no identifier there.
fn ident_end(line: &str, from: usize) -> usize {
  let mut i = from;
  while i < line.len() {
    let Some(ch) = line[i..].chars().next() else { break };
    if !(ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.') {
      break;
    }
    i += ch.len_utf8();
  }
  i
}

/// One character on from `i`, never one byte into the middle of one.
fn step(line: &str, i: usize) -> usize {
  i + line[i..].chars().next().map_or(1, char::len_utf8)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Walks a document the way the editor does — line by line, carrying the
  /// block state — and hands back what each line was coloured with.
  fn walk(syntax: Syntax, text: &str) -> Vec<Vec<(String, &'static str)>> {
    let mut block = Block::default();
    let mut out = Vec::new();
    for line in text.lines() {
      let spans = (syntax.paint)(line, block);
      out.push(
        spans
          .iter()
          .map(|span| (line[span.from..span.to].to_string(), span.class))
          .collect(),
      );
      // The two halves must agree about what the line leaves behind, or the
      // window would be coloured against a state nothing else has.
      let after = (syntax.advance)(line, block);
      block = after;
    }
    out
  }

  #[test]
  fn markdown_does_not_read_prose_rules_inside_a_fence() {
    let lines = walk(
      markdown_syntax(),
      "**bold**\n```rust\nlet a = b * c; // *not bold*\n```\n**bold again**\n",
    );
    assert_eq!(lines[0][0].1, STRONG);
    assert_eq!(lines[1][0].1, CODE, "the opening fence");
    let inside: Vec<&str> = lines[2].iter().map(|(_, class)| *class).collect();
    assert!(inside.contains(&Token::Keyword.class()), "`let` is a keyword");
    assert!(!inside.contains(&STRONG), "nothing inside a fence is bold");
    assert_eq!(lines[3][0].1, CODE, "the closing fence");
    assert_eq!(lines[4][0].1, STRONG, "and prose resumes after it");
  }

  #[test]
  fn a_shorter_run_of_backticks_does_not_close_a_fence() {
    let lines = walk(markdown_syntax(), "````text\n```\nstill inside\n````\n");
    assert_eq!(lines[1].len(), 0, "the inner ``` is content");
    assert_eq!(lines[3][0].1, CODE);
  }

  #[test]
  fn an_unnamed_fence_is_left_alone() {
    let lines = walk(markdown_syntax(), "```\n*not bold*\n```\n");
    assert!(lines[1].is_empty());
  }

  #[test]
  fn typx_headings_lists_and_code() {
    let lines = walk(typx_syntax(), "= Заголовок\n- пункт\n#let x = 2\n");
    assert_eq!(lines[0][0].0, "=");
    assert_eq!(lines[0][1].1, HEADING);
    assert_eq!(lines[1][0].0, "-");
    assert_eq!(lines[2][0].0, "#let");
    assert_eq!(lines[2][0].1, Token::Keyword.class());
    assert!(
      lines[2]
        .iter()
        .any(|(text, class)| text == "2" && *class == Token::Number.class())
    );
  }

  #[test]
  fn typx_comments_nest_and_span_lines() {
    let lines = walk(typx_syntax(), "/* one /* two */\nstill\n*/ *bold*\n");
    assert_eq!(lines[1][0].1, Token::Comment.class(), "still inside");
    assert_eq!(lines[2][0].1, Token::Comment.class());
    assert!(lines[2].iter().any(|(_, class)| *class == STRONG), "and out again");
  }

  #[test]
  fn typx_raw_blocks_are_code() {
    let lines = walk(typx_syntax(), "```python\ndef f(): return \"*x*\"\n```\n");
    let inside: Vec<&str> = lines[1].iter().map(|(_, class)| *class).collect();
    assert!(inside.contains(&Token::Keyword.class()));
    assert!(inside.contains(&Token::Str.class()));
    assert!(!inside.contains(&STRONG));
  }

  #[test]
  fn spans_stay_sorted_and_on_character_boundaries() {
    for text in [
      "# Заголовок с *акцентом* и `кодом`\n> цитата\n1. пункт\n",
      "= Заголовок\n#figure(image(\"схема.svg\"), caption: [Схема])\nтекст @ссылка <метка>\n",
    ] {
      for syntax in [markdown_syntax(), typx_syntax()] {
        let mut block = Block::default();
        for line in text.lines() {
          let mut at = 0;
          for span in (syntax.paint)(line, block) {
            assert!(span.from >= at, "{line}: spans out of order");
            assert!(span.to <= line.len(), "{line}: span past the end");
            assert!(
              line.is_char_boundary(span.from) && line.is_char_boundary(span.to),
              "{line}: mid-character"
            );
            at = span.to;
          }
          block = (syntax.advance)(line, block);
        }
      }
    }
  }
}

#[cfg(test)]
mod find_tests {
  use super::*;

  fn doc(text: &str) -> Vec<String> {
    text.lines().map(str::to_string).collect()
  }

  fn hit_texts<'a>(lines: &'a [String], hits: &[Hit]) -> Vec<&'a str> {
    hits.iter().map(|h| &lines[h.line][h.from..h.to]).collect()
  }

  #[test]
  fn finds_every_occurrence_in_reading_order() {
    let lines = doc("the cat sat\non the mat\nnothing here\n");
    let hits = find_all(&lines, "at");
    assert_eq!(hit_texts(&lines, &hits), ["at", "at", "at"]);
    assert_eq!(
      hits.iter().map(|h| (h.line, h.from)).collect::<Vec<_>>(),
      [(0, 5), (0, 9), (1, 8)]
    );
  }

  #[test]
  fn an_empty_needle_matches_nothing() {
    assert!(find_all(&doc("anything\n"), "").is_empty());
  }

  /// Case-insensitive, and the offsets still point into the original text.
  #[test]
  fn case_is_ignored_on_both_sides() {
    let lines = doc("Rust and RUST and rust\n");
    let hits = find_all(&lines, "RuSt");
    assert_eq!(hit_texts(&lines, &hits), ["Rust", "RUST", "rust"]);
  }

  /// Matches do not overlap: `aa` in `aaaa` is two hits, not three.
  #[test]
  fn matches_do_not_overlap() {
    let lines = doc("aaaa\n");
    let hits = find_all(&lines, "aa");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[1].from, 2);
  }

  /// The offsets are byte offsets into the line and land on char boundaries —
  /// the selection is built from them, and a boundary off by one byte is a
  /// panic in the middle of somebody's document.
  #[test]
  fn offsets_land_on_character_boundaries() {
    let lines = doc("путь и ПУТЬ\n");
    let hits = find_all(&lines, "путь");
    assert_eq!(hit_texts(&lines, &hits), ["путь", "ПУТЬ"]);
    for hit in &hits {
      assert!(lines[hit.line].is_char_boundary(hit.from));
      assert!(lines[hit.line].is_char_boundary(hit.to));
    }
  }

  /// A fold that changes length must not shift the offsets.
  ///
  /// `İ` (U+0130) lowercases to two chars, so a search done over a folded copy
  /// and reported against the original would point past its own match.
  #[test]
  fn a_fold_that_grows_does_not_shift_the_offsets() {
    let lines = doc("İstanbul\n");
    let hits = find_all(&lines, "stanbul");
    assert_eq!(hits.len(), 1);
    assert_eq!(hit_texts(&lines, &hits), ["stanbul"]);
    assert_eq!(hits[0].from, "İ".len());
  }

  /// A needle longer than the line is not a match, and does not read past it.
  #[test]
  fn a_needle_longer_than_the_line_is_not_a_match() {
    assert!(find_all(&doc("ab\n"), "abcdef").is_empty());
  }

  /// Найденное помечается поверх раскраски, а не вместо неё.
  #[test]
  fn a_hit_marks_the_line_without_losing_the_colour_around_it() {
    // «ключевое слово» на всю строку, и совпадение внутри него.
    let painted = vec![HighlightSpan { from: 0, to: 12, class: "kw" }];
    let out = with_hits(painted, &[(4, 8, HIT_CLASS)]);
    let shape: Vec<_> = out.iter().map(|s| (s.from, s.to, s.class)).collect();
    assert_eq!(shape, vec![(0, 4, "kw"), (4, 8, HIT_CLASS), (8, 12, "kw")]);
  }

  /// Отрезки на выходе отсортированы и не пересекаются — других `make_line` не
  /// принимает, он их молча выбрасывает.
  #[test]
  fn merged_spans_stay_sorted_and_apart() {
    let painted = vec![
      HighlightSpan { from: 0, to: 5, class: "a" },
      HighlightSpan { from: 7, to: 20, class: "b" },
    ];
    let out = with_hits(painted, &[(3, 9, HIT_CURRENT_CLASS), (15, 17, HIT_CLASS)]);
    let mut at = 0;
    for span in &out {
      assert!(span.from >= at, "отрезки разъехались: {:?}", (span.from, span.to));
      assert!(span.to > span.from, "пустой отрезок");
      at = span.to;
    }
    // Отметки целы, а раскраска под ними уступила.
    assert!(out.iter().any(|s| (s.from, s.to, s.class) == (3, 9, HIT_CURRENT_CLASS)));
    assert!(out.iter().any(|s| (s.from, s.to, s.class) == (15, 17, HIT_CLASS)));
  }

  /// Текущее совпадение отличается от остальных, иначе среди двадцати его не
  /// найти.
  #[test]
  fn the_hit_being_stood_on_looks_different() {
    assert_ne!(HIT_CLASS, HIT_CURRENT_CLASS);
    let find = Find {
      hits: vec![
        Hit { line: 3, from: 0, to: 2 },
        Hit { line: 3, from: 5, to: 7 },
        Hit { line: 9, from: 1, to: 3 },
      ],
      at: 1,
      key: None,
    };
    assert_eq!(
      find.on(3),
      vec![(0, 2, HIT_CLASS), (5, 7, HIT_CURRENT_CLASS)],
      "на строке две отметки, и вторая — текущая"
    );
    assert_eq!(find.on(9), vec![(1, 3, HIT_CLASS)]);
    assert!(find.on(4).is_empty(), "на строке без совпадений отметок нет");
  }

  /// Без поиска раскраска не трогается вовсе.
  #[test]
  fn nothing_searched_means_nothing_touched() {
    let painted = vec![HighlightSpan { from: 2, to: 6, class: "kw" }];
    let out = with_hits(painted.clone(), &[]);
    assert_eq!(out.len(), painted.len());
    assert_eq!((out[0].from, out[0].to, out[0].class), (2, 6, "kw"));
  }

  /// Scrolling past the end is offered only once there is something to scroll.
  #[test]
  fn overscroll_waits_until_the_document_is_taller_than_the_view() {
    let st = State {
      row_h: 20.0,
      heights: vec![20.0; 3],
      measured: vec![true; 3],
      lines: vec![String::new(); 3],
      ..Default::default()
    };
    // 60px of text in a 400px editor: nothing to scroll, nothing to pad.
    assert_eq!(overscroll_for(&st, 400.0, true), 0.0);
    // The same document in a 50px editor already scrolls, so the affordance
    // appears — a viewport less one row and the field's own padding, which puts
    // the last line at the top and not a hair above it.
    assert_eq!(overscroll_for(&st, 50.0, true), 50.0 - 20.0 - PAD_Y);
    // And never when the caller did not ask.
    assert_eq!(overscroll_for(&st, 50.0, false), 0.0);
  }

  /// Подложка под концом — не рост документа.
  ///
  /// `follow_end` уносит читателя вниз, когда конец ушёл от него, и меряет это
  /// ростом `scroll_height`. Место под концом — тоже `scroll_height`, и
  /// появляется оно разом, на первой же отрисовке, где его попросили. Открытие
  /// поиска такую отрисовку и вызывало: вместо найденного человек оказывался в
  /// конце текста.
  ///
  /// Проверяется арифметика, которой `follow_end` считает рост: из высоты
  /// прокрутки вычитается место под концом, и появление этого места ростом не
  /// выглядит.
  #[test]
  fn the_padding_below_the_end_is_not_the_document_growing() {
    let view_h = 600.0;
    let doc_h = 20_000.0;
    // Как считает `follow_end`: `scroll_height` минус окно минус подложка.
    let max_of = |scroll_h: f64, overscroll: f64| scroll_h - view_h - overscroll;

    // До того, как место под концом написано.
    let before = max_of(doc_h, 0.0);
    // И после: `scroll_height` вырос ровно на него.
    let after = max_of(doc_h + (view_h - 20.0), view_h - 20.0);
    assert_eq!(before, after, "подложка под концом прочиталась как рост документа");

    // А настоящая дописанная строка ростом быть обязана.
    let typed = max_of(doc_h + 20.0 + (view_h - 20.0), view_h - 20.0);
    assert!(typed > before, "дописанная строка не выглядит ростом");
    assert_eq!(typed - before, 20.0, "рост должен быть ровно в одну строку");
  }

  /// The space below the end is a property of the window, not of the document.
  ///
  /// It read as proportional to the text — a thousand lines of blank under a
  /// thousand lines of writing — and that is what a viewport measured off an
  /// unbounded editor does to it: `client_height` becomes the document's own
  /// height, and a screenful becomes a documentful. The arithmetic was never
  /// the part that was wrong, so this pins the shape of it.
  #[test]
  fn the_space_below_the_end_is_one_screen_whatever_the_document() {
    let screen = 600.0;
    let short = State {
      row_h: 20.0,
      heights: vec![20.0; 40],
      measured: vec![true; 40],
      lines: vec![String::new(); 40],
      ..Default::default()
    };
    let long = State {
      row_h: 20.0,
      heights: vec![20.0; 1000],
      measured: vec![true; 1000],
      lines: vec![String::new(); 1000],
      ..Default::default()
    };
    assert_eq!(
      overscroll_for(&short, screen, true),
      overscroll_for(&long, screen, true),
      "документ длиннее — место под ним то же"
    );
    // И это именно экран без строки и без отступа поля: последняя строка
    // встаёт ровно наверх — не ниже, чтобы её было видно, и не выше, чтобы не
    // срезало.
    assert_eq!(overscroll_for(&long, screen, true), screen - 20.0 - PAD_Y);
    // Двадцать тысяч пикселей текста не дают двадцати тысяч пикселей пустоты.
    assert!(
      overscroll_for(&long, screen, true) < long.total() / 10.0,
      "место под концом выросло с документом"
    );
  }
}
