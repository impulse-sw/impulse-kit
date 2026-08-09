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
//! A selection can only span what the DOM holds, so dragging past the window's
//! edge stops there. The whole-document keys people actually use — `Ctrl+A`,
//! `Ctrl+Home`, `Ctrl+End` and their `Shift` forms — put the whole document in
//! the DOM first and hand over to the browser, so they work at any size, at the
//! cost of one hitch of a few tens of milliseconds on a very long document. The
//! window returns on the next scroll or edit.

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

/// How much beyond the viewport is kept in the DOM, in viewports. Anything one
/// arrow key can reach is already rendered; a drag-selection reaches the edge of
/// this and no further.
const OVERSCAN: f64 = 1.0;

/// How long a run of single-character edits keeps folding into one undo entry.
const UNDO_COALESCE_MS: f64 = 700.0;

/// How much of the fit survives each measured line. Low enough to follow a
/// document that changes shape halfway through, high enough that one odd
/// paragraph does not reprice the rest of it.
const FIT_DECAY: f64 = 0.995;

/// What the font probe is worth in the fit, in characters — about a line.
const PROBE_WEIGHT: f64 = 120.0;

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
  /// The half-open range of lines currently in the DOM.
  first: usize,
  last: usize,
  /// An IME is composing: until it says otherwise, the DOM is the browser's and
  /// touching it destroys the composition.
  composing: bool,
  /// A pointer is down. Re-rendering under a drag throws away the selection
  /// being dragged, so the window holds still until the pointer comes up.
  frozen: bool,
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
      first: 0,
      last: 0,
      composing: false,
      frozen: false,
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

  /// Re-prices every line nobody has measured, from the current fit.
  fn reprice(&mut self) {
    self.applied_slope = self.slope();
    for i in 0..self.lines.len() {
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
        st.reprice();
      });
      render(ctx, true);
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
      on:scroll=move |_| render(ctx, false)
      on:pointerdown=move |_| ctx.state.update_value(|st| st.frozen = true)
      on:pointerup=move |_| {
        ctx.state.update_value(|st| st.frozen = false);
        request_animation_frame(move || render(ctx, false));
      }
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
  // A selection that is not a bare caret is a selection somebody is using: it
  // only exists in the nodes currently rendered, and re-rendering under it
  // throws it away. The window stops moving until the selection collapses.
  let held = ctx.state.with_value(|st| st.frozen) || selection_is_ranged(&content);
  let moved = ctx.state.try_update_value(|st| {
    let view_h = f64::from(root.client_height());
    let scroll_top = f64::from(root.scroll_top());
    let (first, last) = window_for(st, scroll_top, view_h, caret, held);
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
  draw(ctx, &root, &content, caret);
}

/// Paints the window, measures it, and keeps the line the reader is looking at
/// where they are looking at it.
///
/// The heights of unrendered lines are estimates, and rendering replaces an
/// estimate with the truth — which moves everything below it. Anchoring on the
/// topmost visible line is what keeps that correction from yanking the text
/// around as the reader scrolls.
fn draw(ctx: Ctx, root: &Element, content: &Element, caret: Option<Pos>) {
  let scroll_top = f64::from(root.scroll_top());
  // Somebody who has scrolled to the end wants to be at the end, not at the
  // pixel the end used to be at. Measuring a window replaces estimates with the
  // truth, and when the truth is shorter the page gets shorter under them: hold
  // them to the top of the last line and the end stays where they scrolled to,
  // instead of retreating a screen every time they reach for it.
  let at_bottom = scroll_top >= f64::from(root.scroll_height() - root.client_height()) - 2.0;
  let gutter = ctx.gutter_element();
  let corrected = ctx.state.try_update_value(|st| {
    let anchor = st.line_at(scroll_top);
    let gap = scroll_top - st.offset_of(anchor);
    paint(st, content, ctx.highlight);
    measure(st, content);
    pad(st, content);
    if let Some(gutter) = &gutter {
      paint_gutter(st, gutter);
    }
    if let Some(caret) = caret {
      set_dom_caret(content, st.first, caret);
    }
    st.offset_of(anchor) + gap
  });
  if at_bottom {
    root.set_scroll_top(root.scroll_height() - root.client_height());
  } else if let Some(corrected) = corrected
    && (corrected - scroll_top).abs() > 0.5
  {
    root.set_scroll_top(corrected.round() as i32);
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
    record(st, first + head, removed, inserted, caret);
    // A changed line count means the browser merged or split our line divs —
    // Enter drops a bare `\n` into one of them — so the DOM has to be rebuilt
    // into one div per line before anything measures it again.
    let restructured = read.len() != last - first;
    let heights: Vec<f64> = read.iter().map(|line| st.estimate(line)).collect();
    st.lines.splice(first..last, read.iter().cloned());
    st.heights.splice(first..last, heights);
    st.measured.splice(first..last, std::iter::repeat_n(false, read.len()));
    st.first = first;
    st.last = first + read.len();
    st.emitted = st.text();
    Some(restructured)
  });
  let Some(Some(restructured)) = restructured else {
    return;
  };

  ctx.state.with_value(|st| {
    ctx.blank.set(st.is_blank());
    ctx.value.set(st.emitted.clone());
  });

  if restructured {
    draw(
      ctx,
      &root,
      &content,
      dom_caret(&content, ctx.state.with_value(|st| st.first)),
    );
  } else {
    // Nothing moved between lines: measure the line that was typed in, so the
    // scrollbar keeps up with a paragraph growing a row, and leave the DOM alone.
    // Rewriting it under a live caret is how editors lose keystrokes.
    let gutter = ctx.gutter_element();
    ctx.state.update_value(|st| {
      measure(st, &content);
      pad(st, &content);
      if let Some(gutter) = &gutter {
        paint_gutter(st, gutter);
      }
    });
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

/// Which lines belong in the DOM at a given scroll position: what shows,
/// [`OVERSCAN`] screens either side of it, and — always — the caret's line, so
/// an arrow key never walks off the end of what exists.
fn window_for(
  st: &State,
  scroll_top: f64,
  view_h: f64,
  caret: Option<Pos>,
  held: bool,
) -> (usize, usize) {
  let count = st.lines.len();
  if held && st.last > st.first {
    return (st.first, st.last);
  }
  let margin = view_h * OVERSCAN;
  let mut first = st.line_at((scroll_top - margin).max(0.0));
  let mut last = (st.line_at(scroll_top + view_h + margin) + 1).min(count);
  if let Some(caret) = caret {
    first = first.min(caret.line.saturating_sub(2));
    last = last.max((caret.line + 3).min(count));
  }
  first = first.min(count.saturating_sub(1));
  (first, last.clamp(first + 1, count))
}

/// Puts the window's lines in the DOM, one `<div>` each.
fn paint(st: &State, content: &Element, highlight: Option<Highlighter>) {
  let Some(doc) = content.owner_document() else {
    return;
  };
  let fragment = doc.create_document_fragment();
  for line in &st.lines[st.first.min(st.lines.len())..st.last.min(st.lines.len())] {
    if let Some(div) = make_line(&doc, line, highlight) {
      let _ = fragment.append_child(&div);
    }
  }
  content.set_inner_html("");
  let _ = content.append_child(&fragment);
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
  if st.estimates_are_stale() {
    st.reprice();
  }
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

/// Draws the line numbers beside the lines they belong to. Positioned rather
/// than laid out: a soft-wrapped line is two rows tall, and its number belongs
/// against the first of them.
fn paint_gutter(st: &State, gutter: &Element) {
  let Some(doc) = gutter.owner_document() else {
    return;
  };
  let fragment = doc.create_document_fragment();
  let top = st.offset_of(st.first);
  for line in st.first..st.last.min(st.lines.len()) {
    let Ok(cell) = doc.create_element("div") else {
      continue;
    };
    cell.set_class_name("absolute right-2");
    if let Some(cell_el) = cell.dyn_ref::<HtmlElement>() {
      let _ = cell_el
        .style()
        .set_property("top", &format!("{}px", (st.offset_of(line) - top).round()));
    }
    cell.set_text_content(Some(&(line + 1).to_string()));
    let _ = fragment.append_child(&cell);
  }
  gutter.set_inner_html("");
  let _ = gutter.append_child(&fragment);
  if let Some(gutter_el) = gutter.dyn_ref::<HtmlElement>() {
    let _ = gutter_el
      .style()
      .set_property("padding-top", &format!("{}px", top.round()));
  }
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
  draw(ctx, &root, &content, caret);
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
    let (first, last) = window_for(st, scroll_top, view_h, Some(caret), false);
    st.first = first;
    st.last = last;
  });
  draw(ctx, &root, &content, Some(caret));
  ctx.state.with_value(|st| {
    ctx.blank.set(st.is_blank());
    ctx.value.set(st.emitted.clone());
  });
  scroll_caret_into_view(&root, &content);
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
