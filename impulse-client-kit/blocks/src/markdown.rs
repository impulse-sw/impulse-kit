//! A [`Markdown`] block: render a Markdown document — given either inline text
//! or a URL to fetch an `.md` file from — into styled HTML.
//!
//! Every Markdown element (headings, links, code, tables, …) is rendered with a
//! sensible default set of Tailwind classes that follow the Client Kit theme tokens,
//! and every one of them can be overridden individually via [`MarkdownClasses`].
//!
//! ```
//! use impulse_client_kit_blocks::markdown::{render_markdown, MarkdownClasses};
//!
//! let html = render_markdown("# Hello World\n\nSome **bold** text.", &MarkdownClasses::default());
//! assert!(html.contains("<h1"));
//! assert!(html.contains("Hello World"));
//! assert!(html.contains("<strong"));
//! // Headings get GitHub-style slug ids, so in-page `#anchor` links resolve.
//! assert!(html.contains("id=\"hello-world\""));
//!
//! // GitHub-style alerts are rendered as titled callouts.
//! let alert = render_markdown("> [!NOTE]\n> Heads up.", &MarkdownClasses::default());
//! assert!(alert.contains(">Note</div>"));
//! assert!(alert.contains("Heads up."));
//! ```
//!
//! > **Security note:** like any Markdown renderer, the output is injected via
//! > `inner_html`, and raw HTML embedded in the source is passed through as-is.
//! > Only render Markdown you trust, or sanitize it upstream.

use std::collections::HashMap;

use impulse_client_kit::utils::cn;
use impulse_client_kit_components::spinner::{Spinner, SpinnerSize};
use leptos::prelude::*;
use pulldown_cmark::{BlockQuoteKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Where a [`Markdown`] block reads its source document from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownSource {
  /// Markdown content passed directly as a string.
  Inline(String),
  /// A URL to fetch the Markdown document from at render time (CSR/hydrate).
  Url(String),
}

impl MarkdownSource {
  /// Build an [`MarkdownSource::Inline`] from anything string-like.
  pub fn inline(content: impl Into<String>) -> Self {
    Self::Inline(content.into())
  }

  /// Build an [`MarkdownSource::Url`] from anything string-like.
  pub fn url(url: impl Into<String>) -> Self {
    Self::Url(url.into())
  }
}

/// Tailwind classes for one kind of GitHub-style alert (`> [!NOTE]`, …).
///
/// An alert is rendered as a `<div>` callout whose first child is a colored
/// title row (icon + label); the remaining blockquote content follows as normal
/// paragraphs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlertClasses {
  /// The outer `<div>` callout container (border, background, padding).
  pub container: String,
  /// The title row (`<div>` holding the icon and the alert label).
  pub title: String,
}

/// Per-element Tailwind classes applied while rendering Markdown.
///
/// Each field carries the full class string used for the corresponding element.
/// Construct via [`MarkdownClasses::default`] and override only what you need:
///
/// ```
/// use impulse_client_kit_blocks::markdown::MarkdownClasses;
///
/// let classes = MarkdownClasses {
///   h1: "text-4xl font-black text-primary".into(),
///   link: "text-blue-500 underline".into(),
///   ..Default::default()
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownClasses {
  /// `<h1>` heading.
  pub h1: String,
  /// `<h2>` heading.
  pub h2: String,
  /// `<h3>` heading.
  pub h3: String,
  /// `<h4>` heading.
  pub h4: String,
  /// `<h5>` heading.
  pub h5: String,
  /// `<h6>` heading.
  pub h6: String,
  /// `<p>` paragraph.
  pub paragraph: String,
  /// `<a>` link.
  pub link: String,
  /// `<em>` emphasis.
  pub emphasis: String,
  /// `<strong>` strong emphasis.
  pub strong: String,
  /// `<del>` strikethrough (GFM).
  pub strikethrough: String,
  /// Inline `` `code` ``.
  pub inline_code: String,
  /// Fenced/indented `<pre>` code block.
  pub code_block: String,
  /// `<blockquote>`.
  pub blockquote: String,
  /// `> [!NOTE]` alert callout.
  pub alert_note: AlertClasses,
  /// `> [!TIP]` alert callout.
  pub alert_tip: AlertClasses,
  /// `> [!IMPORTANT]` alert callout.
  pub alert_important: AlertClasses,
  /// `> [!WARNING]` alert callout.
  pub alert_warning: AlertClasses,
  /// `> [!CAUTION]` alert callout.
  pub alert_caution: AlertClasses,
  /// `<ul>` unordered list.
  pub unordered_list: String,
  /// `<ol>` ordered list.
  pub ordered_list: String,
  /// `<li>` list item.
  pub list_item: String,
  /// `<img>` image.
  pub image: String,
  /// `<hr>` thematic break.
  pub horizontal_rule: String,
  /// `<table>`.
  pub table: String,
  /// `<thead>` table header group.
  pub table_head: String,
  /// `<tr>` table row.
  pub table_row: String,
  /// `<th>` table header cell.
  pub table_header_cell: String,
  /// `<td>` table body cell.
  pub table_cell: String,
}

impl Default for MarkdownClasses {
  fn default() -> Self {
    Self {
      h1: "mt-8 mb-4 text-3xl font-bold tracking-tight text-foreground".into(),
      h2: "mt-8 mb-3 border-b border-border/60 pb-2 text-2xl font-semibold tracking-tight text-foreground".into(),
      h3: "mt-6 mb-3 text-xl font-semibold tracking-tight text-foreground".into(),
      h4: "mt-6 mb-2 text-lg font-semibold text-foreground".into(),
      h5: "mt-4 mb-2 text-base font-semibold text-foreground".into(),
      h6: "mt-4 mb-2 text-sm font-semibold uppercase tracking-wide text-muted-foreground".into(),
      paragraph: "my-4 leading-7 text-foreground/90".into(),
      link: "font-medium text-primary underline underline-offset-4 hover:text-primary/80".into(),
      emphasis: "italic".into(),
      strong: "font-semibold text-foreground".into(),
      strikethrough: "line-through text-muted-foreground".into(),
      inline_code: "rounded border border-border/40 bg-muted/70 px-1.5 py-0.5 font-mono text-[0.85em] text-foreground/90"
        .into(),
      code_block:
        "my-4 overflow-x-auto rounded-lg border border-border/60 bg-muted/40 p-4 text-sm leading-relaxed font-mono text-foreground/90"
          .into(),
      blockquote: "my-4 border-l-4 border-border pl-4 italic text-muted-foreground".into(),
      alert_note: AlertClasses {
        container: "my-4 space-y-2 rounded-md border-l-4 border-blue-500 bg-blue-500/10 px-4 py-3 [&>p]:my-0".into(),
        title: "flex items-center gap-2 font-semibold text-blue-600 dark:text-blue-400".into(),
      },
      alert_tip: AlertClasses {
        container: "my-4 space-y-2 rounded-md border-l-4 border-emerald-500 bg-emerald-500/10 px-4 py-3 [&>p]:my-0".into(),
        title: "flex items-center gap-2 font-semibold text-emerald-600 dark:text-emerald-400".into(),
      },
      alert_important: AlertClasses {
        container: "my-4 space-y-2 rounded-md border-l-4 border-violet-500 bg-violet-500/10 px-4 py-3 [&>p]:my-0".into(),
        title: "flex items-center gap-2 font-semibold text-violet-600 dark:text-violet-400".into(),
      },
      alert_warning: AlertClasses {
        container: "my-4 space-y-2 rounded-md border-l-4 border-amber-500 bg-amber-500/10 px-4 py-3 [&>p]:my-0".into(),
        title: "flex items-center gap-2 font-semibold text-amber-600 dark:text-amber-400".into(),
      },
      alert_caution: AlertClasses {
        container: "my-4 space-y-2 rounded-md border-l-4 border-red-500 bg-red-500/10 px-4 py-3 [&>p]:my-0".into(),
        title: "flex items-center gap-2 font-semibold text-red-600 dark:text-red-400".into(),
      },
      unordered_list: "my-4 ml-6 list-disc space-y-2 marker:text-muted-foreground".into(),
      ordered_list: "my-4 ml-6 list-decimal space-y-2 marker:text-muted-foreground".into(),
      list_item: "leading-7 text-foreground/90".into(),
      image: "my-4 max-w-full rounded-lg border border-border/60".into(),
      horizontal_rule: "my-8 border-border/60".into(),
      table: "my-4 w-full border-collapse overflow-hidden rounded-lg border border-border/60 text-sm".into(),
      table_head: "bg-muted/50".into(),
      table_row: "border-b border-border/40 last:border-0".into(),
      table_header_cell: "px-4 py-2 text-left font-semibold text-foreground".into(),
      table_cell: "px-4 py-2 text-foreground/90".into(),
    }
  }
}

const CONTAINER_BASE: &str = "impulse-markdown w-full max-w-none break-words";

/// Render a Markdown document into an HTML string, injecting the per-element
/// Tailwind classes from `classes`.
///
/// GFM extensions — tables, strikethrough, task lists, smart punctuation and
/// GitHub-style alerts (`> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`,
/// `> [!WARNING]`, `> [!CAUTION]`) — are enabled. This is the same renderer the
/// [`Markdown`] component uses; it is exposed so callers can pre-render or test
/// the output independently.
pub fn render_markdown(input: &str, classes: &MarkdownClasses) -> String {
  let mut options = Options::empty();
  options.insert(Options::ENABLE_TABLES);
  options.insert(Options::ENABLE_STRIKETHROUGH);
  options.insert(Options::ENABLE_TASKLISTS);
  options.insert(Options::ENABLE_SMART_PUNCTUATION);
  // Parse GitHub-style alerts: `> [!NOTE]`, `> [!TIP]`, `> [!WARNING]`, … —
  // pulldown-cmark surfaces these as `BlockQuote(Some(BlockQuoteKind))`.
  options.insert(Options::ENABLE_GFM);

  let parser = Parser::new_ext(input, options);

  let mut out = String::with_capacity(input.len() * 2);
  // While inside an image, all inner events only contribute to its `alt` text.
  let mut image: Option<ImageCtx> = None;
  let mut in_table_head = false;
  // Headings are buffered so the opening tag can carry a slug `id` (computed
  // from the heading text) for in-page anchor links to resolve.
  let mut head: Option<HeadingCtx> = None;
  let mut head_image: Option<ImageCtx> = None;
  let mut head_thead = false;
  let mut slug_counts: HashMap<String, usize> = HashMap::new();

  for event in parser {
    if let Some(ctx) = image.as_mut() {
      match event {
        Event::Start(_) => ctx.depth += 1,
        Event::End(_) => {
          ctx.depth -= 1;
          if ctx.depth == 0 {
            let ctx = image.take().expect("image context is set");
            push_image(&mut out, &ctx, &classes.image);
          }
        }
        Event::Text(text) | Event::Code(text) => ctx.alt.push_str(text.as_ref()),
        Event::SoftBreak | Event::HardBreak => ctx.alt.push(' '),
        _ => {}
      }
      continue;
    }

    // Inside a heading, collect both its inner HTML and its plain text.
    if head.is_some() {
      match event {
        Event::End(TagEnd::Heading(_)) => {
          let h = head.take().expect("heading is set");
          let (name, class) = heading(h.level, classes);
          let slug = slugify(&h.text, &mut slug_counts);
          out.push_str(&format!("<{name} id=\"{slug}\"{}>", class_attr(class)));
          out.push_str(&h.inner);
          out.push_str(&format!("</{name}>\n"));
        }
        Event::Text(text) => {
          let h = head.as_mut().expect("heading is set");
          push_escaped(&mut h.inner, text.as_ref());
          h.text.push_str(text.as_ref());
        }
        Event::Code(text) => {
          let h = head.as_mut().expect("heading is set");
          h.inner.push_str(&open("code", &classes.inline_code));
          push_escaped(&mut h.inner, text.as_ref());
          h.inner.push_str("</code>");
          h.text.push_str(text.as_ref());
        }
        Event::Start(tag) => {
          let h = head.as_mut().expect("heading is set");
          start_tag(&mut h.inner, tag, classes, &mut head_image, &mut head_thead);
        }
        Event::End(tag) => {
          let h = head.as_mut().expect("heading is set");
          end_tag(&mut h.inner, tag, classes, &mut head_thead);
        }
        Event::Html(html) | Event::InlineHtml(html) => {
          head.as_mut().expect("heading is set").inner.push_str(html.as_ref());
        }
        Event::SoftBreak | Event::HardBreak => {
          let h = head.as_mut().expect("heading is set");
          h.inner.push(' ');
          h.text.push(' ');
        }
        _ => {}
      }
      continue;
    }

    match event {
      Event::Start(Tag::Heading { level, .. }) => {
        head = Some(HeadingCtx {
          level,
          inner: String::new(),
          text: String::new(),
        });
      }
      Event::Start(tag) => start_tag(&mut out, tag, classes, &mut image, &mut in_table_head),
      Event::End(tag) => end_tag(&mut out, tag, classes, &mut in_table_head),
      Event::Text(text) => push_escaped(&mut out, text.as_ref()),
      Event::Code(text) => {
        out.push_str(&open("code", &classes.inline_code));
        push_escaped(&mut out, text.as_ref());
        out.push_str("</code>");
      }
      // Raw HTML embedded in the Markdown is passed through verbatim.
      Event::Html(html) | Event::InlineHtml(html) => out.push_str(html.as_ref()),
      Event::SoftBreak => out.push('\n'),
      Event::HardBreak => out.push_str("<br />\n"),
      Event::Rule => {
        out.push_str("<hr");
        out.push_str(&class_attr(&classes.horizontal_rule));
        out.push_str(" />\n");
      }
      Event::TaskListMarker(checked) => {
        out.push_str("<input disabled type=\"checkbox\"");
        if checked {
          out.push_str(" checked");
        }
        out.push_str(" class=\"mr-2 align-middle\" />");
      }
      _ => {}
    }
  }

  out
}

/// Buffered heading state, so the opening tag can carry a computed slug `id`.
struct HeadingCtx {
  level: HeadingLevel,
  inner: String,
  text: String,
}

/// Build a GitHub-style anchor slug from heading text, de-duplicating with a
/// numeric suffix so repeated headings get unique ids.
fn slugify(text: &str, counts: &mut HashMap<String, usize>) -> String {
  let mut base = String::with_capacity(text.len());
  for c in text.chars() {
    if c.is_alphanumeric() {
      base.extend(c.to_lowercase());
    } else if c == ' ' || c == '-' {
      base.push('-');
    } else if c == '_' {
      base.push('_');
    }
  }
  let n = counts.entry(base.clone()).or_insert(0);
  let slug = if *n == 0 { base.clone() } else { format!("{base}-{n}") };
  *n += 1;
  slug
}

/// State captured while serializing a Markdown image.
struct ImageCtx {
  dest: String,
  title: String,
  alt: String,
  depth: usize,
}

fn start_tag(
  out: &mut String,
  tag: Tag,
  classes: &MarkdownClasses,
  image: &mut Option<ImageCtx>,
  in_table_head: &mut bool,
) {
  match tag {
    Tag::Paragraph => out.push_str(&open("p", &classes.paragraph)),
    Tag::Heading { level, .. } => {
      let (name, class) = heading(level, classes);
      out.push_str(&open(name, class));
    }
    Tag::BlockQuote(Some(kind)) => {
      let (cls, label, icon) = alert(kind, classes);
      out.push_str(&open("div", &cls.container));
      out.push_str(&open("div", &cls.title));
      out.push_str(icon);
      out.push_str(label);
      out.push_str("</div>");
    }
    Tag::BlockQuote(None) => out.push_str(&open("blockquote", &classes.blockquote)),
    Tag::CodeBlock(_) => {
      out.push_str(&open("pre", &classes.code_block));
      out.push_str("<code>");
    }
    Tag::List(Some(start)) => {
      out.push_str("<ol");
      out.push_str(&class_attr(&classes.ordered_list));
      if start != 1 {
        out.push_str(&format!(" start=\"{start}\""));
      }
      out.push('>');
    }
    Tag::List(None) => out.push_str(&open("ul", &classes.unordered_list)),
    Tag::Item => out.push_str(&open("li", &classes.list_item)),
    Tag::Emphasis => out.push_str(&open("em", &classes.emphasis)),
    Tag::Strong => out.push_str(&open("strong", &classes.strong)),
    Tag::Strikethrough => out.push_str(&open("del", &classes.strikethrough)),
    Tag::Link { dest_url, title, .. } => {
      out.push_str("<a");
      out.push_str(&class_attr(&classes.link));
      out.push_str(" href=\"");
      push_escaped(out, dest_url.as_ref());
      out.push('"');
      if !title.is_empty() {
        out.push_str(" title=\"");
        push_escaped(out, title.as_ref());
        out.push('"');
      }
      out.push('>');
    }
    Tag::Image { dest_url, title, .. } => {
      *image = Some(ImageCtx {
        dest: dest_url.into_string(),
        title: title.into_string(),
        alt: String::new(),
        depth: 1,
      });
    }
    Tag::Table(_) => out.push_str(&open("table", &classes.table)),
    Tag::TableHead => {
      *in_table_head = true;
      out.push_str(&open("thead", &classes.table_head));
      out.push_str("<tr>");
    }
    Tag::TableRow => out.push_str("<tr>"),
    Tag::TableCell => {
      let (name, class) = if *in_table_head {
        ("th", &classes.table_header_cell)
      } else {
        ("td", &classes.table_cell)
      };
      out.push_str(&open(name, class));
    }
    _ => {}
  }
}

fn end_tag(out: &mut String, tag: TagEnd, _classes: &MarkdownClasses, in_table_head: &mut bool) {
  match tag {
    TagEnd::Paragraph => out.push_str("</p>\n"),
    TagEnd::Heading(level) => {
      out.push_str("</");
      out.push_str(heading_name(level));
      out.push_str(">\n");
    }
    // An alert blockquote was opened as a `<div>` callout; close it as one.
    TagEnd::BlockQuote(Some(_)) => out.push_str("</div>\n"),
    TagEnd::BlockQuote(None) => out.push_str("</blockquote>\n"),
    TagEnd::CodeBlock => out.push_str("</code></pre>\n"),
    TagEnd::List(true) => out.push_str("</ol>\n"),
    TagEnd::List(false) => out.push_str("</ul>\n"),
    TagEnd::Item => out.push_str("</li>\n"),
    TagEnd::Emphasis => out.push_str("</em>"),
    TagEnd::Strong => out.push_str("</strong>"),
    TagEnd::Strikethrough => out.push_str("</del>"),
    TagEnd::Link => out.push_str("</a>"),
    TagEnd::Table => out.push_str("</tbody></table>\n"),
    TagEnd::TableHead => {
      *in_table_head = false;
      out.push_str("</tr></thead><tbody>");
    }
    TagEnd::TableRow => out.push_str("</tr>"),
    TagEnd::TableCell => {
      out.push_str(if *in_table_head { "</th>" } else { "</td>" });
    }
    _ => {}
  }
}

/// Emit an `<img>` from a finished [`ImageCtx`].
fn push_image(out: &mut String, ctx: &ImageCtx, class: &str) {
  out.push_str("<img");
  out.push_str(&class_attr(class));
  out.push_str(" src=\"");
  push_escaped(out, &ctx.dest);
  out.push_str("\" alt=\"");
  push_escaped(out, &ctx.alt);
  out.push('"');
  if !ctx.title.is_empty() {
    out.push_str(" title=\"");
    push_escaped(out, &ctx.title);
    out.push('"');
  }
  out.push_str(" />");
}

/// Map an alert kind to its configured classes, display label and icon SVG.
fn alert(kind: BlockQuoteKind, classes: &MarkdownClasses) -> (&AlertClasses, &'static str, &'static str) {
  match kind {
    BlockQuoteKind::Note => (&classes.alert_note, "Note", ICON_NOTE),
    BlockQuoteKind::Tip => (&classes.alert_tip, "Tip", ICON_TIP),
    BlockQuoteKind::Important => (&classes.alert_important, "Important", ICON_IMPORTANT),
    BlockQuoteKind::Warning => (&classes.alert_warning, "Warning", ICON_WARNING),
    BlockQuoteKind::Caution => (&classes.alert_caution, "Caution", ICON_CAUTION),
  }
}

/// GitHub Octicon SVGs used as alert title icons. `fill-current` makes them
/// inherit the title row's text color.
const ICON_NOTE: &str = "<svg viewBox=\"0 0 16 16\" width=\"16\" height=\"16\" aria-hidden=\"true\" class=\"h-4 w-4 shrink-0 fill-current\"><path d=\"M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z\"></path></svg>";
const ICON_TIP: &str = "<svg viewBox=\"0 0 16 16\" width=\"16\" height=\"16\" aria-hidden=\"true\" class=\"h-4 w-4 shrink-0 fill-current\"><path d=\"M8 1.5c-2.363 0-4 1.69-4 3.75 0 .984.424 1.625.984 2.304l.214.253c.223.264.47.556.673.848.284.411.537.896.621 1.49a.75.75 0 0 1-1.484.211c-.04-.282-.163-.547-.37-.847a8.456 8.456 0 0 0-.542-.68c-.084-.1-.173-.205-.268-.32C3.201 7.75 2.5 6.766 2.5 5.25 2.5 2.31 4.863 0 8 0s5.5 2.31 5.5 5.25c0 1.516-.701 2.5-1.328 3.259-.095.115-.184.22-.268.319-.207.245-.383.453-.541.681-.208.3-.33.565-.37.847a.751.751 0 0 1-1.485-.212c.084-.593.337-1.078.621-1.489.203-.292.45-.584.673-.848.075-.088.147-.173.213-.253.561-.679.985-1.32.985-2.304 0-2.06-1.637-3.75-4-3.75ZM5.75 12h4.5a.75.75 0 0 1 0 1.5h-4.5a.75.75 0 0 1 0-1.5ZM6 15.25a.75.75 0 0 1 .75-.75h2.5a.75.75 0 0 1 0 1.5h-2.5a.75.75 0 0 1-.75-.75Z\"></path></svg>";
const ICON_IMPORTANT: &str = "<svg viewBox=\"0 0 16 16\" width=\"16\" height=\"16\" aria-hidden=\"true\" class=\"h-4 w-4 shrink-0 fill-current\"><path d=\"M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v9.5A1.75 1.75 0 0 1 14.25 13H8.06l-2.573 2.573A1.458 1.458 0 0 1 3 14.543V13H1.75A1.75 1.75 0 0 1 0 11.25Zm1.75-.25a.25.25 0 0 0-.25.25v9.5c0 .138.112.25.25.25h2a.75.75 0 0 1 .75.75v2.19l2.72-2.72a.749.749 0 0 1 .53-.22h6.5a.25.25 0 0 0 .25-.25v-9.5a.25.25 0 0 0-.25-.25Zm7 2.25v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 9a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z\"></path></svg>";
const ICON_WARNING: &str = "<svg viewBox=\"0 0 16 16\" width=\"16\" height=\"16\" aria-hidden=\"true\" class=\"h-4 w-4 shrink-0 fill-current\"><path d=\"M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z\"></path></svg>";
const ICON_CAUTION: &str = "<svg viewBox=\"0 0 16 16\" width=\"16\" height=\"16\" aria-hidden=\"true\" class=\"h-4 w-4 shrink-0 fill-current\"><path d=\"M4.47.22A.749.749 0 0 1 5 0h6c.199 0 .389.079.53.22l4.25 4.25c.141.141.22.331.22.53v6a.749.749 0 0 1-.22.53l-4.25 4.25A.749.749 0 0 1 11 16H5a.749.749 0 0 1-.53-.22L.22 11.53A.749.749 0 0 1 0 11V5c0-.199.079-.389.22-.53Zm.84 1.28L1.5 5.31v5.38l3.81 3.81h5.38l3.81-3.81V5.31L10.69 1.5ZM8 4a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z\"></path></svg>";

/// Map a heading level to its tag name and configured class.
fn heading(level: HeadingLevel, classes: &MarkdownClasses) -> (&'static str, &str) {
  match level {
    HeadingLevel::H1 => ("h1", &classes.h1),
    HeadingLevel::H2 => ("h2", &classes.h2),
    HeadingLevel::H3 => ("h3", &classes.h3),
    HeadingLevel::H4 => ("h4", &classes.h4),
    HeadingLevel::H5 => ("h5", &classes.h5),
    HeadingLevel::H6 => ("h6", &classes.h6),
  }
}

fn heading_name(level: HeadingLevel) -> &'static str {
  match level {
    HeadingLevel::H1 => "h1",
    HeadingLevel::H2 => "h2",
    HeadingLevel::H3 => "h3",
    HeadingLevel::H4 => "h4",
    HeadingLevel::H5 => "h5",
    HeadingLevel::H6 => "h6",
  }
}

/// Open `<{name} class="...">`, omitting the attribute when the class is empty.
fn open(name: &str, class: &str) -> String {
  format!("<{name}{}>", class_attr(class))
}

/// Render a ` class="..."` attribute, or nothing when `class` is empty.
fn class_attr(class: &str) -> String {
  if class.is_empty() {
    String::new()
  } else {
    format!(" class=\"{class}\"")
  }
}

/// Append `s` to `out`, escaping HTML-significant characters.
fn push_escaped(out: &mut String, s: &str) {
  for c in s.chars() {
    match c {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      _ => out.push(c),
    }
  }
}

/// Fetch a Markdown document over HTTP (CSR/hydrate only).
async fn fetch_markdown(url: &str) -> Result<String, String> {
  let response = gloo_net::http::Request::get(url)
    .send()
    .await
    .map_err(|e| format!("request failed: {e}"))?;
  if !response.ok() {
    return Err(format!("HTTP {} for {url}", response.status()));
  }
  response.text().await.map_err(|e| format!("failed to read body: {e}"))
}

/// Render a Markdown document — inline or fetched from a URL — into styled HTML.
///
/// * `source` — inline Markdown text or a URL to fetch it from.
/// * `classes` — per-element Tailwind overrides; defaults follow the Client Kit theme.
/// * `class` — extra classes for the wrapping container.
///
/// ```rust,ignore
/// use impulse_client_kit_blocks::markdown::{Markdown, MarkdownSource};
/// use leptos::prelude::*;
///
/// view! {
///   <Markdown source=MarkdownSource::inline("# Title\n\nHello **world**.") />
///   <Markdown source=MarkdownSource::url("/docs/readme.md") />
/// }
/// ```
#[component]
pub fn Markdown(
  source: MarkdownSource,
  #[prop(optional)] classes: MarkdownClasses,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  match source {
    MarkdownSource::Inline(text) => {
      let html = render_markdown(&text, &classes);
      view! { <div class=cn(&[CONTAINER_BASE, class.as_str()]) inner_html=html></div> }.into_any()
    }
    MarkdownSource::Url(url) => {
      let classes = StoredValue::new(classes);
      let document = LocalResource::new(move || {
        let url = url.clone();
        async move { fetch_markdown(&url).await }
      });

      let rendered = move || match document.get() {
        None => view! {
          <div class="flex items-center justify-center py-8 text-muted-foreground">
            <Spinner size=SpinnerSize::Default />
          </div>
        }
        .into_any(),
        Some(Ok(text)) => {
          let html = classes.with_value(|classes| render_markdown(&text, classes));
          view! { <div inner_html=html></div> }.into_any()
        }
        Some(Err(error)) => view! {
          <div class="rounded-lg border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            "Failed to load Markdown: " {error}
          </div>
        }
        .into_any(),
      };

      view! { <div class=cn(&[CONTAINER_BASE, class.as_str()])>{rendered}</div> }.into_any()
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn renders_github_alert_as_callout() {
    let html = render_markdown("> [!WARNING]\n> Be careful here.", &MarkdownClasses::default());
    // Alerts become a `<div>` callout, not a `<blockquote>`.
    assert!(!html.contains("<blockquote"));
    assert!(html.contains("border-amber-500"));
    // Title row carries the icon and the kind label.
    assert!(html.contains("<svg"));
    assert!(html.contains(">Warning</div>"));
    // The blockquote body is rendered as normal content.
    assert!(html.contains("Be careful here."));
  }

  #[test]
  fn renders_each_alert_kind() {
    for (marker, label, border) in [
      ("NOTE", "Note", "border-blue-500"),
      ("TIP", "Tip", "border-emerald-500"),
      ("IMPORTANT", "Important", "border-violet-500"),
      ("WARNING", "Warning", "border-amber-500"),
      ("CAUTION", "Caution", "border-red-500"),
    ] {
      let src = format!("> [!{marker}]\n> Body.");
      let html = render_markdown(&src, &MarkdownClasses::default());
      assert!(html.contains(border), "{marker} should use {border}");
      assert!(html.contains(&format!(">{label}</div>")), "{marker} should show {label}");
    }
  }

  #[test]
  fn plain_blockquote_is_unchanged() {
    let html = render_markdown("> Just a quote.", &MarkdownClasses::default());
    assert!(html.contains("<blockquote"));
    assert!(!html.contains("<svg"));
  }
}
