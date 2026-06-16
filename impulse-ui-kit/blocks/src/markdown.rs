//! A [`Markdown`] block: render a Markdown document — given either inline text
//! or a URL to fetch an `.md` file from — into styled HTML.
//!
//! Every Markdown element (headings, links, code, tables, …) is rendered with a
//! sensible default set of Tailwind classes that follow the UI Kit theme tokens,
//! and every one of them can be overridden individually via [`MarkdownClasses`].
//!
//! ```
//! use impulse_ui_kit_blocks::markdown::{render_markdown, MarkdownClasses};
//!
//! let html = render_markdown("# Hello\n\nSome **bold** text.", &MarkdownClasses::default());
//! assert!(html.contains("<h1"));
//! assert!(html.contains("Hello"));
//! assert!(html.contains("<strong"));
//! ```
//!
//! > **Security note:** like any Markdown renderer, the output is injected via
//! > `inner_html`, and raw HTML embedded in the source is passed through as-is.
//! > Only render Markdown you trust, or sanitize it upstream.

use impulse_ui_kit::utils::cn;
use impulse_ui_kit_components::spinner::{Spinner, SpinnerSize};
use leptos::prelude::*;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

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

/// Per-element Tailwind classes applied while rendering Markdown.
///
/// Each field carries the full class string used for the corresponding element.
/// Construct via [`MarkdownClasses::default`] and override only what you need:
///
/// ```
/// use impulse_ui_kit_blocks::markdown::MarkdownClasses;
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
/// GFM extensions — tables, strikethrough, task lists and smart punctuation —
/// are enabled. This is the same renderer the [`Markdown`] component uses; it is
/// exposed so callers can pre-render or test the output independently.
pub fn render_markdown(input: &str, classes: &MarkdownClasses) -> String {
  let mut options = Options::empty();
  options.insert(Options::ENABLE_TABLES);
  options.insert(Options::ENABLE_STRIKETHROUGH);
  options.insert(Options::ENABLE_TASKLISTS);
  options.insert(Options::ENABLE_SMART_PUNCTUATION);

  let parser = Parser::new_ext(input, options);

  let mut out = String::with_capacity(input.len() * 2);
  // While inside an image, all inner events only contribute to its `alt` text.
  let mut image: Option<ImageCtx> = None;
  let mut in_table_head = false;

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

    match event {
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
    Tag::BlockQuote(_) => out.push_str(&open("blockquote", &classes.blockquote)),
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
    TagEnd::BlockQuote(_) => out.push_str("</blockquote>\n"),
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
/// * `classes` — per-element Tailwind overrides; defaults follow the UI Kit theme.
/// * `class` — extra classes for the wrapping container.
///
/// ```rust,ignore
/// use impulse_ui_kit_blocks::markdown::{Markdown, MarkdownSource};
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
            "Failed to load Markdown: "
            {error}
          </div>
        }
        .into_any(),
      };

      view! { <div class=cn(&[CONTAINER_BASE, class.as_str()])>{rendered}</div> }.into_any()
    }
  }
}
