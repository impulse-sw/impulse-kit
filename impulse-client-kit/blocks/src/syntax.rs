//! Colouring code: a small language registry, and one scanner that reads every
//! language in it.
//!
//! # Why a registry rather than a highlighter per language
//!
//! [`SourceEditor`](crate::editor::SourceEditor) takes one
//! [`Syntax`](crate::editor::Syntax) — the markup the document is written in.
//! But a document written in Markdown (or in typx) has *other* languages inside
//! it: everything between two fences is Rust, or SQL, or a shell session, and
//! colouring that as prose is worse than not colouring it at all, which is the
//! bug this module exists to fix. The fence names the language as a word —
//! ```` ```rust ```` — so what the editor needs is a way to turn a word into a
//! way of colouring, at the moment it paints the line.
//!
//! That is what this is: languages are *data* in a registry, addressed by
//! [`LangId`], and one scanner reads all of them. A `LangId` is a number, so it
//! fits in the couple of bytes a line carries to the next one, and looking a
//! language up costs nothing per line.
//!
//! # Adding your own
//!
//! A language is a [`LangDef`]: its keywords, what a comment looks like, what a
//! string looks like, and whether it has numbers. That covers the useful part of
//! what a syntax file in an editor like Kate describes, at a fraction of the
//! machinery — and the part it leaves out (context stacks, regular expressions,
//! embedded rules per region) is the part that costs a scanner its speed, which
//! here is spent on every visible line of a document on every keystroke.
//!
//! ```rust,ignore
//! use impulse_client_kit_blocks::syntax::{register_lang, LangDef, StringRule};
//!
//! register_lang(
//!   LangDef::new("nim", &["nim", "nimrod"])
//!     .keywords(&["proc", "func", "let", "var", "const", "if", "else", "for", "while"])
//!     .types(&["int", "float", "string", "bool", "seq"])
//!     .line_comments(&["#"])
//!     .strings(&[StringRule::quoted('"'), StringRule::quoted('\'')]),
//! );
//! ```
//!
//! Register before the editor first paints — at start-up, next to the theme —
//! and any fence naming `nim` is coloured from then on. Registering a name that
//! is already taken replaces it, so an application can also *re-spell* a
//! built-in: give it a wider keyword list, or take the string rules off a
//! language whose quotes it uses for something else.
//!
//! # What a colour means
//!
//! A rule does not pick a colour: it picks a [`Token`], and the token decides
//! the classes. Two reasons, and both matter. Tailwind emits CSS only for
//! classes it can *see* while scanning the sources, so a class assembled at
//! runtime — from a definition registered by an application, say — is a class
//! with no CSS behind it and no error to say so. And a palette that every
//! language shares is what keeps a document that switches language mid-page
//! from switching colour schemes with it.

use std::cell::RefCell;
use std::rc::Rc;

use crate::editor::HighlightSpan;

/// What a run of code *is*, which is what decides how it is coloured.
///
/// Deliberately few: a scanner that can tell a keyword from a type from a string
/// is telling the reader everything the colour is for, and every category beyond
/// that is one more thing for a hand-written language definition to get wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
  /// `fn`, `let`, `if` — the language's own words.
  Keyword,
  /// `u32`, `String` — the words that name types.
  Type,
  /// `println!`, `len` — what the language ships with.
  Builtin,
  /// `true`, `nil`, `NULL`.
  Constant,
  /// A name being called: `foo` in `foo(x)`.
  Function,
  /// Anything in quotes, and the quotes with it.
  Str,
  /// A numeric literal.
  Number,
  /// A comment, to the end of the line or to its closing delimiter.
  Comment,
  /// Markup that is not content: a fence, a heading's hashes, a list bullet.
  Marker,
}

impl Token {
  /// The Tailwind classes this token is drawn with.
  ///
  /// Written out as literals, one per arm, because that is the only form
  /// Tailwind's scanner can see (see the module docs). The palette is the same
  /// in both themes by construction: a dark-mode variant on every colour, and
  /// the theme's own `muted-foreground` where the point is to recede.
  pub fn class(self) -> &'static str {
    match self {
      Token::Keyword => "font-medium text-violet-700 dark:text-violet-400",
      Token::Type => "text-sky-700 dark:text-sky-300",
      Token::Builtin => "text-teal-700 dark:text-teal-300",
      Token::Constant => "text-orange-700 dark:text-orange-400",
      Token::Function => "text-blue-700 dark:text-blue-400",
      Token::Str => "text-emerald-700 dark:text-emerald-400",
      Token::Number => "text-amber-700 dark:text-amber-400",
      Token::Comment => "italic text-muted-foreground",
      Token::Marker => "text-muted-foreground",
    }
  }
}

/// A language's place in the registry. `LangId::NONE` is "no language": the
/// scanner does nothing with it, which is what an unnamed fence gets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LangId(u16);

impl LangId {
  /// No language — a fence with no word after it, or a word nobody registered.
  pub const NONE: LangId = LangId(0);

  /// Whether this actually names a language.
  pub fn is_some(self) -> bool {
    self.0 != 0
  }
}

/// How one flavour of string literal is written.
#[derive(Clone, Debug)]
pub struct StringRule {
  /// What opens it (`"`, `'''`, `r#"`).
  pub open: String,
  /// What closes it. The same as `open` for an ordinary quote.
  pub close: String,
  /// The character that makes the next one literal, if the language has one.
  pub escape: Option<char>,
  /// Whether it may run past the end of a line.
  pub multiline: bool,
}

impl StringRule {
  /// The ordinary case: one character opens and closes it, `\` escapes.
  pub fn quoted(ch: char) -> Self {
    Self {
      open: ch.to_string(),
      close: ch.to_string(),
      escape: Some('\\'),
      multiline: false,
    }
  }

  /// A literal that runs until its closing delimiter, however many lines away.
  pub fn multiline(open: &str, close: &str) -> Self {
    Self {
      open: open.to_string(),
      close: close.to_string(),
      escape: None,
      multiline: true,
    }
  }

  /// Without the escape character — for a language whose backslash means
  /// nothing (a shell's single quotes, TOML's literal strings).
  pub fn raw(mut self) -> Self {
    self.escape = None;
    self
  }
}

/// Everything the scanner knows about a language.
///
/// Built with the chained setters rather than by filling in fields, so a
/// definition reads as a list of what the language *has* and stays valid when
/// this grows another one.
#[derive(Clone, Debug, Default)]
pub struct LangDef {
  /// What the language is called, for diagnostics.
  pub name: String,
  /// The words a fence may name it by, lower-cased.
  pub aliases: Vec<String>,
  pub keywords: Vec<String>,
  pub types: Vec<String>,
  pub builtins: Vec<String>,
  pub constants: Vec<String>,
  /// What starts a comment that runs to the end of the line.
  pub line_comments: Vec<String>,
  /// What opens and closes a comment that does not.
  pub block_comment: Option<(String, String)>,
  /// The string literals, tried in order — so a longer opener (`"""`) must come
  /// before the shorter one it starts with (`"`).
  pub strings: Vec<StringRule>,
  /// Whether a run of digits is worth colouring. Off for prose-ish formats,
  /// where every date and version number would light up.
  pub numbers: bool,
  /// Characters that may appear in an identifier besides letters, digits and
  /// `_` — `$` in JavaScript, `-` in Lisp and CSS.
  pub ident_extra: String,
  /// Whether `name(` colours `name` as a call.
  pub function_calls: bool,
}

impl LangDef {
  /// A language with a name and the words a fence may name it by.
  pub fn new(name: &str, aliases: &[&str]) -> Self {
    Self {
      name: name.to_string(),
      aliases: aliases.iter().map(|a| a.to_lowercase()).collect(),
      function_calls: true,
      numbers: true,
      ..Self::default()
    }
  }

  pub fn keywords(mut self, words: &[&str]) -> Self {
    self.keywords = words.iter().map(|w| (*w).to_string()).collect();
    self
  }

  pub fn types(mut self, words: &[&str]) -> Self {
    self.types = words.iter().map(|w| (*w).to_string()).collect();
    self
  }

  pub fn builtins(mut self, words: &[&str]) -> Self {
    self.builtins = words.iter().map(|w| (*w).to_string()).collect();
    self
  }

  pub fn constants(mut self, words: &[&str]) -> Self {
    self.constants = words.iter().map(|w| (*w).to_string()).collect();
    self
  }

  pub fn line_comments(mut self, marks: &[&str]) -> Self {
    self.line_comments = marks.iter().map(|m| (*m).to_string()).collect();
    self
  }

  pub fn block_comment(mut self, open: &str, close: &str) -> Self {
    self.block_comment = Some((open.to_string(), close.to_string()));
    self
  }

  pub fn strings(mut self, rules: &[StringRule]) -> Self {
    self.strings = rules.to_vec();
    self
  }

  pub fn numbers(mut self, on: bool) -> Self {
    self.numbers = on;
    self
  }

  pub fn ident_extra(mut self, chars: &str) -> Self {
    self.ident_extra = chars.to_string();
    self
  }

  pub fn function_calls(mut self, on: bool) -> Self {
    self.function_calls = on;
    self
  }

  /// What a word is, if it is anything.
  fn word(&self, word: &str) -> Option<Token> {
    let has = |list: &[String]| list.iter().any(|w| w == word);
    if has(&self.keywords) {
      Some(Token::Keyword)
    } else if has(&self.types) {
      Some(Token::Type)
    } else if has(&self.builtins) {
      Some(Token::Builtin)
    } else if has(&self.constants) {
      Some(Token::Constant)
    } else {
      None
    }
  }

  /// Whether `ch` may appear inside an identifier of this language.
  fn ident_char(&self, ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || self.ident_extra.contains(ch)
  }
}

// -- The registry ------------------------------------------------------------

#[derive(Default)]
struct Registry {
  /// Definitions by id. Index 0 is never filled: [`LangId::NONE`] is a language
  /// that does not exist, and giving it a slot would make "unknown" and "the
  /// first one registered" the same number.
  langs: Vec<Option<Rc<LangDef>>>,
  loaded: bool,
}

thread_local! {
  static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// Adds a language, or replaces one registered under the same alias.
///
/// Replacing keeps the id: a document that is already open carries ids on its
/// lines, and handing the same name a new number would leave those lines
/// pointing at the old definition until every one of them was re-scanned.
pub fn register_lang(def: LangDef) -> LangId {
  REGISTRY.with_borrow_mut(|registry| {
    registry.ensure_builtins();
    registry.insert(def)
  })
}

/// The id a fence's word names, or [`LangId::NONE`] for a word nobody
/// registered.
///
/// The word is taken as written and matched case-insensitively, so `Rust`,
/// `rust` and `RUST` are one language.
pub fn lang_id(name: &str) -> LangId {
  let name = name.trim().to_lowercase();
  if name.is_empty() {
    return LangId::NONE;
  }
  REGISTRY.with_borrow_mut(|registry| {
    registry.ensure_builtins();
    registry
      .langs
      .iter()
      .position(|slot| {
        slot
          .as_ref()
          .is_some_and(|def| def.aliases.iter().any(|alias| *alias == name))
      })
      .map(|index| LangId(index as u16))
      .unwrap_or(LangId::NONE)
  })
}

/// The definition behind an id, if it still names one.
pub fn lang_def(id: LangId) -> Option<Rc<LangDef>> {
  if !id.is_some() {
    return None;
  }
  REGISTRY.with_borrow_mut(|registry| {
    registry.ensure_builtins();
    registry.langs.get(id.0 as usize).cloned().flatten()
  })
}

/// The languages the registry knows, by name — what a "choose a language" menu
/// would list.
pub fn lang_names() -> Vec<String> {
  REGISTRY.with_borrow_mut(|registry| {
    registry.ensure_builtins();
    registry
      .langs
      .iter()
      .flatten()
      .map(|def| def.name.clone())
      .collect()
  })
}

impl Registry {
  fn insert(&mut self, def: LangDef) -> LangId {
    let existing = self.langs.iter().position(|slot| {
      slot
        .as_ref()
        .is_some_and(|old| old.aliases.iter().any(|alias| def.aliases.contains(alias)))
    });
    match existing {
      Some(index) => {
        self.langs[index] = Some(Rc::new(def));
        LangId(index as u16)
      }
      None => {
        if self.langs.is_empty() {
          // The slot `LangId::NONE` names, kept empty on purpose.
          self.langs.push(None);
        }
        self.langs.push(Some(Rc::new(def)));
        LangId((self.langs.len() - 1) as u16)
      }
    }
  }

  /// Fills the registry with what ships here, once.
  ///
  /// Lazily rather than at start-up: an application that never opens a fenced
  /// document never pays for the list, and one that replaces a built-in can do
  /// it from anywhere without having to be ordered against an initialiser.
  fn ensure_builtins(&mut self) {
    if std::mem::replace(&mut self.loaded, true) {
      return;
    }
    for def in builtins() {
      self.insert(def);
    }
  }
}

// -- The scanner -------------------------------------------------------------

/// Where a line leaves the scanner: not in anything (0), inside the block
/// comment (1), or inside the multiline string rule `n` (2 + n).
pub(crate) const IN_NOTHING: u8 = 0;
pub(crate) const IN_COMMENT: u8 = 1;
pub(crate) const IN_STRING: u8 = 2;

/// Colours one line of `lang`, and says what the *next* line starts inside.
///
/// `base` is where the line's own text begins in the string the offsets are
/// measured against, which lets a markup highlighter hand a *part* of a line to
/// this — the text after a `>` quote marker, say. `out` is optional because the
/// same walk answers both questions the editor asks: "what does this line look
/// like", for the lines on screen, and "what does it leave behind", for every
/// line above them. The second one runs over whole documents and must not
/// allocate.
pub(crate) fn scan(line: &str, lang: LangId, state: u8, base: usize, mut out: Option<&mut Vec<HighlightSpan>>) -> u8 {
  let Some(def) = lang_def(lang) else {
    return IN_NOTHING;
  };
  let mut push = |from: usize, to: usize, token: Token| {
    if let Some(out) = out.as_deref_mut()
      && to > from
    {
      out.push(HighlightSpan::new(base + from, base + to, token.class()));
    }
  };

  let mut i = 0usize;
  // A line that opens inside something finishes that first, and only what is
  // left of it is code.
  match state {
    IN_COMMENT => {
      let Some((_, close)) = def.block_comment.as_ref() else {
        return IN_NOTHING;
      };
      match line.find(close.as_str()) {
        Some(at) => {
          push(0, at + close.len(), Token::Comment);
          i = at + close.len();
        }
        None => {
          push(0, line.len(), Token::Comment);
          return IN_COMMENT;
        }
      }
    }
    state if state >= IN_STRING => {
      let index = (state - IN_STRING) as usize;
      let Some(rule) = def.strings.get(index) else {
        return IN_NOTHING;
      };
      match find_close(line, 0, rule) {
        Some(end) => {
          push(0, end, Token::Str);
          i = end;
        }
        None => {
          push(0, line.len(), Token::Str);
          return state;
        }
      }
    }
    _ => {}
  }

  while i < line.len() {
    let rest = &line[i..];

    if def.line_comments.iter().any(|mark| rest.starts_with(mark.as_str())) {
      push(i, line.len(), Token::Comment);
      return IN_NOTHING;
    }

    if let Some((open, close)) = def.block_comment.as_ref()
      && rest.starts_with(open.as_str())
    {
      match line[i + open.len()..].find(close.as_str()) {
        Some(at) => {
          let to = i + open.len() + at + close.len();
          push(i, to, Token::Comment);
          i = to;
        }
        None => {
          push(i, line.len(), Token::Comment);
          return IN_COMMENT;
        }
      }
      continue;
    }

    if let Some((index, rule)) = def
      .strings
      .iter()
      .enumerate()
      .find(|(_, rule)| rest.starts_with(rule.open.as_str()))
    {
      let from = i + rule.open.len();
      match find_close(line, from, rule) {
        Some(end) => {
          push(i, end, Token::Str);
          i = end;
        }
        None => {
          push(i, line.len(), Token::Str);
          // A quote left open on a line is a typo in a language whose strings
          // stop at the newline, and the *next* line is not part of it — that is
          // what keeps one stray `'` from colouring the rest of the file green.
          return if rule.multiline {
            IN_STRING + index as u8
          } else {
            IN_NOTHING
          };
        }
      }
      continue;
    }

    let Some(ch) = rest.chars().next() else { break };

    if def.numbers && ch.is_ascii_digit() {
      let to = number_end(line, i);
      push(i, to, Token::Number);
      i = to;
      continue;
    }

    // A word: everything a letter can start, up to the first character that is
    // not part of one. Digits are inside a word but never start it, or `0x1f`
    // would be an identifier.
    if ch.is_alphabetic() || ch == '_' || def.ident_extra.contains(ch) {
      let mut to = i;
      while to < line.len() {
        let Some(next) = line[to..].chars().next() else { break };
        if !def.ident_char(next) {
          break;
        }
        to += next.len_utf8();
      }
      let word = &line[i..to];
      if let Some(token) = def.word(word) {
        push(i, to, token);
      } else if def.function_calls && line[to..].starts_with('(') {
        push(i, to, Token::Function);
      }
      i = to;
      continue;
    }

    i += ch.len_utf8();
  }
  IN_NOTHING
}

/// Where the literal opened by `rule` ends, counting from `from` — one past its
/// closing delimiter, or `None` if the line does not close it.
fn find_close(line: &str, from: usize, rule: &StringRule) -> Option<usize> {
  let mut i = from;
  while i < line.len() {
    let rest = &line[i..];
    if let Some(escape) = rule.escape
      && rest.starts_with(escape)
    {
      // The escape and whatever it escapes, together, so an escaped quote is
      // never read as the closing one.
      i += escape.len_utf8();
      i += line[i..].chars().next().map_or(0, char::len_utf8);
      continue;
    }
    if rest.starts_with(rule.close.as_str()) {
      return Some(i + rule.close.len());
    }
    i += rest.chars().next().map_or(1, char::len_utf8);
  }
  None
}

/// Where the number starting at `from` ends.
///
/// Generous on purpose: hex digits, the letters that mark a base or a width
/// (`0x`, `1_000u32`, `1e-9`), and the separators languages allow inside one. A
/// number is not the place to be exact — it is coloured as one run either way,
/// and stopping short of a suffix leaves a stray `u32` looking like a type.
fn number_end(line: &str, from: usize) -> usize {
  let mut i = from;
  let mut prev = '0';
  while i < line.len() {
    let Some(ch) = line[i..].chars().next() else { break };
    let sign = (ch == '+' || ch == '-') && matches!(prev, 'e' | 'E');
    if !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || sign) {
      break;
    }
    // `1..10` is a range, not a number with two dots in it.
    if ch == '.' && line[i + 1..].starts_with('.') {
      break;
    }
    prev = ch;
    i += ch.len_utf8();
  }
  i
}

// -- What ships here ---------------------------------------------------------

/// The languages the registry starts with.
///
/// Chosen by what turns up between fences in the documents this editor is for —
/// notes, articles, READMEs and specifications — rather than by any attempt at
/// coverage. Anything missing is a [`register_lang`] call away, which is the
/// point of the registry.
fn builtins() -> Vec<LangDef> {
  let c_strings = [StringRule::quoted('"'), StringRule::quoted('\'')];
  vec![
    LangDef::new("Rust", &["rust", "rs"])
      .keywords(&[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "fn", "for",
        "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
        "static", "struct", "super", "trait", "type", "union", "unsafe", "use", "where", "while", "yield",
      ])
      .types(&[
        "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "str", "u8", "u16", "u32", "u64",
        "u128", "usize", "String", "Vec", "Option", "Result", "Box", "Rc", "Arc", "HashMap", "HashSet", "BTreeMap",
      ])
      .constants(&["true", "false", "None", "Some", "Ok", "Err"])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&c_strings)
      .ident_extra("!"),
    LangDef::new("C", &["c", "h"])
      .keywords(&[
        "auto", "break", "case", "const", "continue", "default", "do", "else", "enum", "extern", "for", "goto", "if",
        "inline", "register", "restrict", "return", "sizeof", "static", "struct", "switch", "typedef", "union",
        "volatile", "while",
      ])
      .types(&[
        "char", "double", "float", "int", "long", "short", "signed", "unsigned", "void", "size_t", "ssize_t",
        "uint8_t", "uint16_t", "uint32_t", "uint64_t", "int8_t", "int16_t", "int32_t", "int64_t", "bool",
      ])
      .constants(&["NULL", "true", "false"])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&c_strings),
    LangDef::new("C++", &["cpp", "c++", "cxx", "hpp"])
      .keywords(&[
        "alignas", "auto", "break", "case", "catch", "class", "concept", "const", "constexpr", "continue",
        "co_await", "co_return", "co_yield", "default", "delete", "do", "else", "enum", "explicit", "export",
        "extern", "for", "friend", "goto", "if", "inline", "namespace", "new", "noexcept", "operator", "private",
        "protected", "public", "requires", "return", "sizeof", "static", "static_cast", "struct", "switch",
        "template", "this", "throw", "try", "typedef", "typename", "union", "using", "virtual", "volatile", "while",
      ])
      .types(&[
        "bool", "char", "double", "float", "int", "long", "short", "signed", "unsigned", "void", "size_t",
        "std::string", "string", "vector", "map", "set", "optional", "unique_ptr", "shared_ptr",
      ])
      .constants(&["nullptr", "true", "false", "NULL"])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&c_strings),
    LangDef::new("Python", &["python", "py", "python3"])
      .keywords(&[
        "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else",
        "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal", "not", "or",
        "pass", "raise", "return", "try", "while", "with", "yield", "match", "case",
      ])
      .types(&["bool", "bytes", "dict", "float", "frozenset", "int", "list", "set", "str", "tuple", "object"])
      .builtins(&[
        "abs", "all", "any", "enumerate", "filter", "isinstance", "len", "map", "max", "min", "open", "print",
        "range", "repr", "reversed", "round", "sorted", "sum", "super", "type", "zip",
      ])
      .constants(&["True", "False", "None", "self", "cls"])
      .line_comments(&["#"])
      .strings(&[
        StringRule::multiline("\"\"\"", "\"\"\""),
        StringRule::multiline("'''", "'''"),
        StringRule::quoted('"'),
        StringRule::quoted('\''),
      ]),
    LangDef::new("JavaScript", &["javascript", "js", "jsx", "mjs", "cjs"])
      .keywords(&[
        "async", "await", "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete",
        "do", "else", "export", "extends", "finally", "for", "from", "function", "get", "if", "import", "in",
        "instanceof", "let", "new", "of", "return", "set", "static", "super", "switch", "this", "throw", "try",
        "typeof", "var", "void", "while", "yield",
      ])
      .types(&["Array", "Boolean", "Map", "Number", "Object", "Promise", "Set", "String", "Symbol"])
      .builtins(&["console", "document", "fetch", "globalThis", "JSON", "Math", "process", "window"])
      .constants(&["true", "false", "null", "undefined", "NaN", "Infinity"])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&[
        StringRule::multiline("`", "`"),
        StringRule::quoted('"'),
        StringRule::quoted('\''),
      ])
      .ident_extra("$"),
    LangDef::new("TypeScript", &["typescript", "ts", "tsx"])
      .keywords(&[
        "abstract", "as", "async", "await", "break", "case", "catch", "class", "const", "continue", "declare",
        "default", "delete", "do", "else", "enum", "export", "extends", "finally", "for", "from", "function", "get",
        "if", "implements", "import", "in", "infer", "instanceof", "interface", "keyof", "let", "namespace", "new",
        "of", "private", "protected", "public", "readonly", "return", "satisfies", "set", "static", "super",
        "switch", "this", "throw", "try", "type", "typeof", "var", "void", "while", "yield",
      ])
      .types(&[
        "any", "bigint", "boolean", "never", "number", "object", "string", "symbol", "unknown", "void", "Array",
        "Map", "Partial", "Promise", "Readonly", "Record", "Set",
      ])
      .constants(&["true", "false", "null", "undefined"])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&[
        StringRule::multiline("`", "`"),
        StringRule::quoted('"'),
        StringRule::quoted('\''),
      ])
      .ident_extra("$"),
    LangDef::new("Go", &["go", "golang"])
      .keywords(&[
        "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough", "for", "func",
        "go", "goto", "if", "import", "interface", "map", "package", "range", "return", "select", "struct",
        "switch", "type", "var",
      ])
      .types(&[
        "bool", "byte", "complex64", "complex128", "error", "float32", "float64", "int", "int8", "int16", "int32",
        "int64", "rune", "string", "uint", "uint8", "uint16", "uint32", "uint64", "uintptr", "any",
      ])
      .builtins(&["append", "cap", "close", "copy", "delete", "len", "make", "new", "panic", "print", "recover"])
      .constants(&["true", "false", "nil", "iota"])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&[
        StringRule::multiline("`", "`"),
        StringRule::quoted('"'),
        StringRule::quoted('\''),
      ]),
    LangDef::new("Java", &["java"])
      .keywords(&[
        "abstract", "assert", "break", "case", "catch", "class", "const", "continue", "default", "do", "else",
        "enum", "extends", "final", "finally", "for", "goto", "if", "implements", "import", "instanceof",
        "interface", "native", "new", "package", "private", "protected", "public", "record", "return", "sealed",
        "static", "strictfp", "super", "switch", "synchronized", "this", "throw", "throws", "transient", "try",
        "var", "volatile", "while", "yield",
      ])
      .types(&[
        "boolean", "byte", "char", "double", "float", "int", "long", "short", "void", "String", "Integer", "List",
        "Map", "Object", "Optional", "Set",
      ])
      .constants(&["true", "false", "null"])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&c_strings),
    LangDef::new("Kotlin", &["kotlin", "kt", "kts"])
      .keywords(&[
        "as", "break", "by", "catch", "class", "companion", "const", "constructor", "continue", "data", "do",
        "else", "enum", "external", "false", "final", "finally", "for", "fun", "if", "import", "in", "infix",
        "init", "inline", "interface", "internal", "is", "lateinit", "object", "open", "operator", "out",
        "override", "package", "private", "protected", "public", "reified", "return", "sealed", "super",
        "suspend", "this", "throw", "try", "typealias", "val", "var", "vararg", "when", "while",
      ])
      .types(&[
        "Any", "Boolean", "Byte", "Char", "Double", "Float", "Int", "List", "Long", "Map", "Nothing", "Set",
        "Short", "String", "Unit",
      ])
      .constants(&["true", "false", "null", "it"])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&[StringRule::multiline("\"\"\"", "\"\"\""), StringRule::quoted('"')]),
    LangDef::new("Shell", &["bash", "sh", "shell", "zsh", "console", "terminal"])
      .keywords(&[
        "case", "do", "done", "elif", "else", "esac", "exit", "export", "fi", "for", "function", "if", "in",
        "local", "read", "return", "select", "then", "until", "while",
      ])
      .builtins(&[
        "awk", "cat", "cd", "chmod", "cp", "curl", "cut", "docker", "echo", "find", "git", "grep", "head", "kill",
        "ls", "make", "mkdir", "mv", "printf", "rm", "sed", "set", "sort", "source", "sudo", "tail", "tar", "test",
        "touch", "uniq", "wget", "xargs",
      ])
      .line_comments(&["#"])
      .strings(&[StringRule::quoted('"'), StringRule::quoted('\'').raw()])
      .numbers(false)
      .function_calls(false)
      .ident_extra("-$"),
    LangDef::new("SQL", &["sql", "postgres", "postgresql", "mysql", "sqlite"])
      .keywords(&[
        "ALTER", "AND", "AS", "ASC", "BEGIN", "BETWEEN", "BY", "CASE", "COMMIT", "CREATE", "DELETE", "DESC",
        "DISTINCT", "DROP", "ELSE", "END", "EXISTS", "FROM", "FULL", "GROUP", "HAVING", "IN", "INDEX", "INNER",
        "INSERT", "INTO", "IS", "JOIN", "LEFT", "LIKE", "LIMIT", "NOT", "OFFSET", "ON", "OR", "ORDER", "OUTER",
        "RETURNING", "RIGHT", "ROLLBACK", "SELECT", "SET", "TABLE", "THEN", "UNION", "UPDATE", "VALUES", "VIEW",
        "WHEN", "WHERE", "WITH",
        "alter", "and", "as", "asc", "begin", "between", "by", "case", "commit", "create", "delete", "desc",
        "distinct", "drop", "else", "end", "exists", "from", "full", "group", "having", "in", "index", "inner",
        "insert", "into", "is", "join", "left", "like", "limit", "not", "offset", "on", "or", "order", "outer",
        "returning", "right", "rollback", "select", "set", "table", "then", "union", "update", "values", "view",
        "when", "where", "with",
      ])
      .types(&[
        "BIGINT", "BOOLEAN", "BYTEA", "DATE", "DECIMAL", "DOUBLE", "FLOAT", "INT", "INTEGER", "JSON", "JSONB",
        "NUMERIC", "REAL", "SERIAL", "SMALLINT", "TEXT", "TIMESTAMP", "UUID", "VARCHAR",
      ])
      .constants(&["NULL", "TRUE", "FALSE", "null", "true", "false"])
      .line_comments(&["--"])
      .block_comment("/*", "*/")
      .strings(&[StringRule::quoted('\'').raw(), StringRule::quoted('"').raw()]),
    LangDef::new("JSON", &["json", "jsonc", "json5"])
      .constants(&["true", "false", "null"])
      .line_comments(&["//"])
      .strings(&[StringRule::quoted('"')])
      .function_calls(false),
    LangDef::new("YAML", &["yaml", "yml"])
      .constants(&["true", "false", "null", "yes", "no", "on", "off", "~"])
      .line_comments(&["#"])
      .strings(&[StringRule::quoted('"'), StringRule::quoted('\'').raw()])
      .function_calls(false),
    LangDef::new("TOML", &["toml"])
      .constants(&["true", "false"])
      .line_comments(&["#"])
      .strings(&[
        StringRule::multiline("\"\"\"", "\"\"\""),
        StringRule::multiline("'''", "'''"),
        StringRule::quoted('"'),
        StringRule::quoted('\'').raw(),
      ])
      .function_calls(false),
    LangDef::new("INI", &["ini", "conf", "cfg", "properties"])
      .constants(&["true", "false", "yes", "no"])
      .line_comments(&[";", "#"])
      .strings(&[StringRule::quoted('"'), StringRule::quoted('\'').raw()])
      .function_calls(false),
    LangDef::new("XML", &["xml", "html", "svg", "xhtml", "vue"])
      .keywords(&[
        "a", "body", "br", "button", "div", "form", "h1", "h2", "h3", "head", "html", "img", "input", "li", "link",
        "meta", "nav", "ol", "p", "script", "section", "span", "style", "table", "td", "th", "title", "tr", "ul",
      ])
      .block_comment("<!--", "-->")
      .strings(&[StringRule::quoted('"'), StringRule::quoted('\'')])
      .numbers(false)
      .function_calls(false)
      .ident_extra("-:"),
    LangDef::new("CSS", &["css", "scss", "sass", "less"])
      .keywords(&[
        "@apply", "@import", "@media", "@keyframes", "@font-face", "@supports", "@layer", "@tailwind", "@variant",
        "!important",
      ])
      .types(&[
        "align-items", "background", "border", "color", "display", "flex", "font-family", "font-size", "gap",
        "grid", "height", "justify-content", "margin", "overflow", "padding", "position", "width", "z-index",
      ])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&[StringRule::quoted('"'), StringRule::quoted('\'')])
      .function_calls(true)
      .ident_extra("-@!"),
    LangDef::new("Diff", &["diff", "patch"])
      .line_comments(&["#"])
      .numbers(false)
      .function_calls(false),
    LangDef::new("Lua", &["lua"])
      .keywords(&[
        "and", "break", "do", "else", "elseif", "end", "for", "function", "goto", "if", "in", "local", "not", "or",
        "repeat", "return", "then", "until", "while",
      ])
      .builtins(&["ipairs", "pairs", "print", "require", "select", "setmetatable", "tonumber", "tostring", "type"])
      .constants(&["true", "false", "nil", "self"])
      .line_comments(&["--"])
      .block_comment("--[[", "]]")
      .strings(&[
        StringRule::multiline("[[", "]]"),
        StringRule::quoted('"'),
        StringRule::quoted('\''),
      ]),
    LangDef::new("Ruby", &["ruby", "rb"])
      .keywords(&[
        "alias", "and", "begin", "break", "case", "class", "def", "defined?", "do", "else", "elsif", "end",
        "ensure", "for", "if", "in", "module", "next", "not", "or", "redo", "rescue", "retry", "return", "self",
        "super", "then", "unless", "until", "when", "while", "yield",
      ])
      .builtins(&["attr_accessor", "attr_reader", "attr_writer", "puts", "require", "require_relative", "raise"])
      .constants(&["true", "false", "nil"])
      .line_comments(&["#"])
      .strings(&c_strings)
      .ident_extra("?!@"),
    LangDef::new("PHP", &["php"])
      .keywords(&[
        "abstract", "as", "break", "case", "catch", "class", "const", "continue", "declare", "default", "do",
        "echo", "else", "elseif", "enum", "extends", "final", "finally", "fn", "for", "foreach", "function",
        "global", "if", "implements", "include", "instanceof", "interface", "match", "namespace", "new", "print",
        "private", "protected", "public", "readonly", "require", "return", "static", "switch", "throw", "trait",
        "try", "use", "var", "while", "yield",
      ])
      .types(&["array", "bool", "callable", "float", "int", "iterable", "mixed", "object", "string", "void"])
      .constants(&["true", "false", "null", "$this"])
      .line_comments(&["//", "#"])
      .block_comment("/*", "*/")
      .strings(&c_strings)
      .ident_extra("$"),
    LangDef::new("C#", &["csharp", "cs", "c#"])
      .keywords(&[
        "abstract", "as", "async", "await", "base", "break", "case", "catch", "class", "const", "continue",
        "default", "delegate", "do", "else", "enum", "event", "explicit", "extern", "finally", "fixed", "for",
        "foreach", "get", "goto", "if", "implicit", "in", "interface", "internal", "is", "lock", "namespace",
        "new", "operator", "out", "override", "params", "private", "protected", "public", "readonly", "record",
        "ref", "return", "sealed", "set", "sizeof", "static", "struct", "switch", "this", "throw", "try",
        "typeof", "using", "var", "virtual", "void", "while", "yield",
      ])
      .types(&[
        "bool", "byte", "char", "decimal", "double", "float", "int", "long", "object", "sbyte", "short", "string",
        "uint", "ulong", "ushort", "Dictionary", "List", "Task",
      ])
      .constants(&["true", "false", "null"])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&c_strings),
    LangDef::new("Dockerfile", &["dockerfile", "docker"])
      .keywords(&[
        "ADD", "ARG", "CMD", "COPY", "ENTRYPOINT", "ENV", "EXPOSE", "FROM", "HEALTHCHECK", "LABEL", "RUN", "SHELL",
        "STOPSIGNAL", "USER", "VOLUME", "WORKDIR",
      ])
      .line_comments(&["#"])
      .strings(&[StringRule::quoted('"'), StringRule::quoted('\'').raw()])
      .numbers(false)
      .function_calls(false),
    LangDef::new("Typst", &["typst", "typ", "typx"])
      .keywords(&[
        "and", "as", "break", "continue", "else", "for", "if", "import", "in", "include", "let", "not", "or",
        "return", "set", "show", "while",
      ])
      .builtins(&[
        "align", "block", "box", "calc", "document", "emph", "figure", "grid", "heading", "image", "link", "list",
        "pad", "pagebreak", "par", "raw", "rect", "stack", "strong", "table", "text", "v", "h",
      ])
      .constants(&["true", "false", "none", "auto"])
      .line_comments(&["//"])
      .block_comment("/*", "*/")
      .strings(&[StringRule::quoted('"')])
      .ident_extra("-"),
  ]
}

#[cfg(test)]
mod tests {
  use super::*;

  fn run(line: &str, lang: &str, state: u8) -> (Vec<HighlightSpan>, u8) {
    let mut out = Vec::new();
    let next = scan(line, lang_id(lang), state, 0, Some(&mut out));
    (out, next)
  }

  #[test]
  fn known_and_unknown_languages() {
    assert!(lang_id("rust").is_some());
    assert!(lang_id("RUST").is_some());
    assert!(!lang_id("nothing-like-this").is_some());
    assert!(!lang_id("").is_some());
  }

  #[test]
  fn colours_a_line_of_rust() {
    let (spans, next) = run("let x: u32 = 1; // hi", "rust", IN_NOTHING);
    let classes: Vec<&str> = spans.iter().map(|s| s.class).collect();
    assert_eq!(next, IN_NOTHING);
    assert_eq!(classes[0], Token::Keyword.class());
    assert!(classes.contains(&Token::Type.class()));
    assert!(classes.contains(&Token::Number.class()));
    assert_eq!(*classes.last().unwrap(), Token::Comment.class());
  }

  #[test]
  fn block_comments_span_lines() {
    let (_, next) = run("/* opened", "rust", IN_NOTHING);
    assert_eq!(next, IN_COMMENT);
    let (spans, next) = run("still in it", "rust", IN_COMMENT);
    assert_eq!(next, IN_COMMENT);
    assert_eq!(spans.len(), 1);
    let (_, next) = run("closed */ let", "rust", IN_COMMENT);
    assert_eq!(next, IN_NOTHING);
  }

  #[test]
  fn a_stray_quote_does_not_run_away() {
    // Rust strings are not multiline in this model, so an unclosed one ends
    // with its line rather than colouring the rest of the document.
    let (_, next) = run("let s = \"oops;", "rust", IN_NOTHING);
    assert_eq!(next, IN_NOTHING);
    // Python's triple quote is, and does.
    let (_, next) = run("s = \"\"\"opened", "python", IN_NOTHING);
    assert!(next >= IN_STRING);
    let (_, next) = run("still text", "python", next);
    assert!(next >= IN_STRING);
    let (_, next) = run("done\"\"\"", "python", next);
    assert_eq!(next, IN_NOTHING);
  }

  #[test]
  fn escapes_do_not_close_a_string() {
    let (spans, _) = run(r#"let s = "a\"b"; let t = 1;"#, "rust", IN_NOTHING);
    let string = spans
      .iter()
      .find(|s| s.class == Token::Str.class())
      .expect("a string span");
    assert_eq!(string.to - string.from, r#""a\"b""#.len());
  }

  #[test]
  fn spans_are_sorted_and_within_the_line() {
    let line = "def f(x): return x + 1  # считает";
    let (spans, _) = run(line, "python", IN_NOTHING);
    let mut at = 0;
    for span in &spans {
      assert!(span.from >= at, "spans out of order");
      assert!(span.to <= line.len());
      assert!(line.is_char_boundary(span.from) && line.is_char_boundary(span.to));
      at = span.to;
    }
  }

  #[test]
  fn a_registered_language_wins() {
    register_lang(
      LangDef::new("Toy", &["toy-lang"])
        .keywords(&["wibble"])
        .line_comments(&["%"]),
    );
    let (spans, _) = run("wibble % rest", "toy-lang", IN_NOTHING);
    assert_eq!(spans[0].class, Token::Keyword.class());
    assert_eq!(spans[1].class, Token::Comment.class());
  }
}
