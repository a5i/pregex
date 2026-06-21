//! Node.js bindings for the [`eregex`](https://docs.rs/eregex) regular
//! expression engine, generated with [napi-rs](https://napi.rs).
//!
//! This crate is a thin adapter: all matching logic lives in the `eregex`
//! core crate, and here we only translate its Rust types into JavaScript-
//! friendly classes, objects and functions. See `README.md` for a usage
//! overview.
//
// This is an FFI binding: `napi`/`napi-derive` legitimately use `unsafe` to
// talk to Node, so we use `deny` (overridable by the generated code) rather
// than `forbid`. We still catch any accidental `unsafe` we write ourselves.

#![deny(unsafe_code)]

use std::collections::HashMap;

use napi::bindgen_prelude::*;
use napi_derive::napi;

// ===========================================================================
// Flag constants
// ===========================================================================

/// `(?i)` — case-insensitive matching. Combine with bitwise OR, e.g.
/// `eregex.IGNORECASE | eregex.MULTILINE`.
#[napi]
pub const IGNORECASE: u32 = eregex::flags::IGNORECASE.bits();
/// `(?m)` — `^` and `$` match at line boundaries.
#[napi]
pub const MULTILINE: u32 = eregex::flags::MULTILINE.bits();
/// `(?s)` — `.` matches any character including newlines.
#[napi]
pub const DOTALL: u32 = eregex::flags::DOTALL.bits();
/// `(?u)` — use Unicode semantics for `\d \w \s \b` (the default).
#[napi]
pub const UNICODE: u32 = eregex::flags::UNICODE.bits();
/// `(?a)` — use ASCII-only semantics for `\d \w \s \b`.
#[napi]
pub const ASCII: u32 = eregex::flags::ASCII.bits();
/// `(?x)` — free-spacing mode; whitespace and `#` comments are ignored.
#[napi]
pub const VERBOSE: u32 = eregex::flags::VERBOSE.bits();
/// `(?f)` — full case-folding for case-insensitive matches.
#[napi]
pub const FULLCASE: u32 = eregex::flags::FULLCASE.bits();
/// `(?w)` — Unicode default word-boundary semantics for `\b`/`\B`.
#[napi]
pub const WORD: u32 = eregex::flags::WORD.bits();
/// `(?L)` — locale-sensitive (legacy, limited support).
#[napi]
pub const LOCALE: u32 = eregex::flags::LOCALE.bits();
/// `(?V0)` — version 0 (legacy `re`-compatible) behaviour.
#[napi]
pub const VERSION0: u32 = eregex::flags::VERSION0.bits();
/// `(?V1)` — version 1 (enhanced) behaviour (the default).
#[napi]
pub const VERSION1: u32 = eregex::flags::VERSION1.bits();

/// Convert a flag string such as `"ims"` into a flags bitset (a bitwise OR of
/// the exported `IGNORECASE` / `MULTILINE` / ... constants).
///
/// Recognized letters (case-insensitive): `i m s u a x f w l`.
/// The `RegExp`-style `g`, `y`, `d` are accepted but ignored (familiarity).
///
/// @throws {Error} on an unknown flag character.
#[napi]
pub fn parse_flags(flag_str: String) -> Result<u32> {
    let mut f = eregex::Flags::NONE;
    for c in flag_str.chars() {
        match c.to_ascii_lowercase() {
            'i' => f |= eregex::flags::IGNORECASE,
            'm' => f |= eregex::flags::MULTILINE,
            's' => f |= eregex::flags::DOTALL,
            'u' => f |= eregex::flags::UNICODE,
            'a' => f |= eregex::flags::ASCII,
            'x' => f |= eregex::flags::VERBOSE,
            'f' => f |= eregex::flags::FULLCASE,
            'w' => f |= eregex::flags::WORD,
            'l' => f |= eregex::flags::LOCALE,
            'g' | 'y' | 'd' => {}
            other => {
                return Err(Error::from_reason(format!(
                    "unknown flag character {other:?}"
                )))
            }
        }
    }
    Ok(f.bits())
}

// ===========================================================================
// Regex
// ===========================================================================

/// A compiled regular expression.
///
/// Compile once with `new Regex(pattern)` or `new Regex(pattern, flags)`,
/// where `flags` is a bitwise OR of the exported flag constants, then reuse
/// it across many inputs via methods like `find`, `isMatch`, `findAll`,
/// `findPartial`, `replace`, `replaceAll`, and `split`.
#[napi]
pub struct Regex {
    re: eregex::Regex,
}

#[napi]
impl Regex {
    /// Compile `pattern`. `flags` (optional) is a bitwise OR of the
    /// `eregex.IGNORECASE` / `MULTILINE` / ... constants (or the result of
    /// `eregex.parseFlags("ims")`).
    ///
    /// @throws {Error} if `pattern` is syntactically invalid.
    #[napi(constructor)]
    pub fn new(pattern: String, flags: Option<u32>) -> Result<Self> {
        let f = eregex::Flags(flags.unwrap_or(0));
        let re = eregex::Regex::new_with_flags(&pattern, f)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self { re })
    }

    /// The original pattern string.
    #[napi(getter)]
    pub fn pattern(&self) -> String {
        self.re.as_str().to_string()
    }

    /// The resolved flags as a bitset.
    #[napi(getter)]
    pub fn flags(&self) -> u32 {
        self.re.flags().bits()
    }

    /// The number of capturing groups (group 0 is the whole match and is not
    /// counted here).
    #[napi(getter)]
    pub fn capture_count(&self) -> u32 {
        self.re.capture_count() as u32
    }

    /// Names of all named groups.
    #[napi]
    pub fn group_names(&self) -> Vec<String> {
        self.re.group_names().keys().cloned().collect()
    }

    /// Index (1-based) of a named group, or `null` if it does not exist.
    #[napi]
    pub fn group_index(&self, name: String) -> Option<u32> {
        self.re.group_index(&name).map(|i| i as u32)
    }

    /// `true` if the pattern matches anywhere in `haystack`.
    #[napi]
    pub fn is_match(&self, haystack: String) -> bool {
        self.re.is_match(&haystack)
    }

    /// First match anywhere in `haystack`, or `null`.
    #[napi]
    pub fn find(&self, haystack: String) -> Option<Match> {
        let names = self.re.group_names().clone();
        let m = self.re.find(&haystack)?;
        Some(Match::from_match(&haystack, &m, &names))
    }

    /// First match at or after byte offset `start`, or `null`.
    #[napi]
    pub fn find_at(&self, haystack: String, start: u32) -> Option<Match> {
        let names = self.re.group_names().clone();
        let m = self.re.find_at(&haystack, start as usize)?;
        Some(Match::from_match(&haystack, &m, &names))
    }

    /// Match anchored at the start of `haystack` (Python's `re.match`).
    #[napi]
    pub fn match_at_start(&self, haystack: String) -> Option<Match> {
        let names = self.re.group_names().clone();
        let m = self.re.match_at_start(&haystack)?;
        Some(Match::from_match(&haystack, &m, &names))
    }

    /// Match covering the whole `haystack` (Python's `re.fullmatch`).
    #[napi]
    pub fn full_match(&self, haystack: String) -> Option<Match> {
        let names = self.re.group_names().clone();
        let m = self.re.fullmatch(&haystack)?;
        Some(Match::from_match(&haystack, &m, &names))
    }

    /// All non-overlapping matches.
    #[napi]
    pub fn find_all(&self, haystack: String) -> Vec<Match> {
        let names = self.re.group_names().clone();
        self.re
            .find_iter(&haystack)
            .map(|m| Match::from_match(&haystack, &m, &names))
            .collect()
    }

    /// Partial / end-anchored match. Returns `null` if the input cannot be a
    /// prefix of any full match.
    #[napi]
    pub fn find_partial(&self, haystack: String) -> Option<PartialMatch> {
        let names = self.re.group_names().clone();
        let p = self.re.find_partial(&haystack)?;
        Some(PartialMatch::from_partial(p, &names))
    }

    /// Replace the first match using template `repl` (`$1`, `${name}`, `$$`).
    #[napi]
    pub fn replace(&self, haystack: String, repl: String) -> String {
        self.re.replace(&haystack, &repl)
    }

    /// Replace every non-overlapping match using template `repl`.
    #[napi]
    pub fn replace_all(&self, haystack: String, repl: String) -> String {
        self.re.replace_all(&haystack, &repl)
    }

    /// Split `haystack` by this pattern, returning the parts.
    #[napi]
    pub fn split(&self, haystack: String) -> Vec<String> {
        self.re.split(&haystack)
    }

    /// Pretty-print the parsed AST (debug aid).
    #[napi]
    pub fn dump(&self) -> String {
        self.re.dump()
    }
}

// ===========================================================================
// Match
// ===========================================================================

/// A `[start, end]` byte span, returned by `Match.span` and `Match.spanOf`.
#[napi(object)]
pub struct Span {
    /// Byte offset where the span starts.
    pub start: u32,
    /// Byte offset where the span ends.
    pub end: u32,
}

/// A successful match, carrying the full capture state.
#[napi]
pub struct Match {
    input: String,
    groups: Vec<Option<String>>,
    spans: Vec<Option<(usize, usize)>>,
    captures: Vec<Vec<Option<String>>>,
    named: HashMap<String, u32>,
}

impl Match {
    /// Build a JS `Match` from a borrowed `eregex::Match`. All data is cloned
    /// into owned form so the JS object is self-contained.
    fn from_match(haystack: &str, m: &eregex::Match, names: &HashMap<String, usize>) -> Self {
        let n = m.len();
        let groups = (0..n).map(|g| m.group(g).map(str::to_string)).collect();
        let spans = (0..n)
            .map(|g| match m.span_of(g) {
                (s, e) if m.group(g).is_some() => Some((s, e)),
                _ => None,
            })
            .collect();
        let captures = (0..n)
            .map(|g| {
                m.captures(g)
                    .into_iter()
                    .map(|o| o.map(str::to_string))
                    .collect()
            })
            .collect();
        let named = names.iter().map(|(k, &v)| (k.clone(), v as u32)).collect();
        Match {
            input: haystack.to_string(),
            groups,
            spans,
            captures,
            named,
        }
    }
}

#[napi]
impl Match {
    /// The whole match text (group 0).
    #[napi(getter)]
    pub fn matched(&self) -> String {
        self.groups
            .get(0)
            .and_then(|o| o.clone())
            .unwrap_or_default()
    }

    /// The original input string this match was found in.
    #[napi(getter)]
    pub fn input(&self) -> String {
        self.input.clone()
    }

    /// Byte offset where the whole match starts.
    #[napi(getter)]
    pub fn start(&self) -> u32 {
        self.span_start(0)
    }

    /// Byte offset where the whole match ends.
    #[napi(getter)]
    pub fn end(&self) -> u32 {
        self.span_end(0)
    }

    /// The `{ start, end }` byte span of the whole match.
    #[napi(getter)]
    pub fn span(&self) -> Span {
        Span {
            start: self.span_start(0),
            end: self.span_end(0),
        }
    }

    /// The number of capturing groups (group 0 not counted).
    #[napi(getter)]
    pub fn capture_count(&self) -> u32 {
        self.groups.len().saturating_sub(1) as u32
    }

    /// Current text of every group (group 0 first). Groups that did not
    /// participate are `null`.
    #[napi(getter)]
    pub fn groups(&self) -> Vec<Option<String>> {
        self.groups.clone()
    }

    /// Map of named-group name to its current text.
    #[napi(getter)]
    pub fn named_groups(&self) -> HashMap<String, String> {
        self.named
            .iter()
            .filter_map(|(name, &idx)| {
                self.groups
                    .get(idx as usize)
                    .and_then(|o| o.as_ref().map(|s| (name.clone(), s.clone())))
            })
            .collect()
    }

    /// Captures of every group (group 0 first); each entry is a group's
    /// repeated-capture history. Non-participating iterations are `null`.
    #[napi(getter)]
    pub fn all_captures(&self) -> Vec<Vec<Option<String>>> {
        self.captures.clone()
    }

    /// Map of named-group name to its repeated-capture history.
    #[napi(getter)]
    pub fn captures_dict(&self) -> HashMap<String, Vec<Option<String>>> {
        self.named
            .iter()
            .filter_map(|(name, &idx)| {
                self.captures
                    .get(idx as usize)
                    .map(|v| (name.clone(), v.clone()))
            })
            .collect()
    }

    /// Text of group `index` (0 = whole match), or `null` if it did not
    /// participate.
    #[napi]
    pub fn group(&self, index: u32) -> Option<String> {
        self.groups.get(index as usize).and_then(|o| o.clone())
    }

    /// Text of a named group, or `null`.
    #[napi]
    pub fn named_group(&self, name: String) -> Option<String> {
        let idx = *self.named.get(&name)?;
        self.groups.get(idx as usize).and_then(|o| o.clone())
    }

    /// All captures (repeated-capture history) of group `index`.
    #[napi]
    pub fn captures(&self, index: u32) -> Vec<Option<String>> {
        self.captures
            .get(index as usize)
            .cloned()
            .unwrap_or_default()
    }

    /// All captures of a named group.
    #[napi]
    pub fn captures_by_name(&self, name: String) -> Vec<Option<String>> {
        match self.named.get(&name) {
            Some(&idx) => self.captures.get(idx as usize).cloned().unwrap_or_default(),
            None => Vec::new(),
        }
    }

    /// `{ start, end }` byte span of group `index`, or `null` if it did not
    /// participate.
    #[napi]
    pub fn span_of(&self, index: u32) -> Option<Span> {
        match self.spans.get(index as usize).and_then(|o| *o) {
            Some((s, e)) => Some(Span {
                start: s as u32,
                end: e as u32,
            }),
            None => None,
        }
    }

    fn span_start(&self, g: usize) -> u32 {
        match self.spans.get(g).and_then(|o| *o) {
            Some((s, _)) => s as u32,
            None => self.input.len() as u32,
        }
    }

    fn span_end(&self, g: usize) -> u32 {
        match self.spans.get(g).and_then(|o| *o) {
            Some((_, e)) => e as u32,
            None => self.input.len() as u32,
        }
    }
}

// ===========================================================================
// PartialMatch
// ===========================================================================

/// A partial (or full) end-anchored match, the result of
/// [`Regex::find_partial`]. (`null` from `findPartial` — not this object —
/// means the input cannot be a prefix of any match.)
#[napi]
pub struct PartialMatch {
    status_full: bool,
    matched: String,
    start: usize,
    end: usize,
    states: Vec<u8>, // 0=matched, 1=partial, 2=none
    group_text: Vec<Option<String>>,
    named: HashMap<String, u32>,
}

impl PartialMatch {
    fn from_partial(p: eregex::PartialMatch, names: &HashMap<String, usize>) -> Self {
        let status_full = matches!(p.status, eregex::MatchStatus::Full);
        let matched = p.matched.to_string();
        let start = p.start;
        let end = p.end;
        let mut states = Vec::with_capacity(p.groups.len());
        let mut group_text = Vec::with_capacity(p.groups.len());
        for g in &p.groups {
            match g {
                eregex::GroupMatch::Matched(s) => {
                    states.push(0);
                    group_text.push(Some(s.to_string()));
                }
                eregex::GroupMatch::Partial(s) => {
                    states.push(1);
                    group_text.push(Some(s.to_string()));
                }
                eregex::GroupMatch::None => {
                    states.push(2);
                    group_text.push(None);
                }
            }
        }
        let named = names.iter().map(|(k, &v)| (k.clone(), v as u32)).collect();
        PartialMatch {
            status_full,
            matched,
            start,
            end,
            states,
            group_text,
            named,
        }
    }
}

#[napi]
impl PartialMatch {
    /// `"full"` or `"partial"`.
    #[napi(getter)]
    pub fn status(&self) -> String {
        if self.status_full {
            "full".into()
        } else {
            "partial".into()
        }
    }

    /// `true` if the match is fully satisfied.
    #[napi(getter)]
    pub fn is_full(&self) -> bool {
        self.status_full
    }

    /// `true` if the match was cut short by end-of-input.
    #[napi(getter)]
    pub fn is_partial(&self) -> bool {
        !self.status_full
    }

    /// The whole matched text.
    #[napi(getter)]
    pub fn matched(&self) -> String {
        self.matched.clone()
    }

    /// Byte offset where the match starts.
    #[napi(getter)]
    pub fn start(&self) -> u32 {
        self.start as u32
    }

    /// Byte offset where the match ends (always the input length).
    #[napi(getter)]
    pub fn end(&self) -> u32 {
        self.end as u32
    }

    /// The number of capturing groups (group 0 not counted).
    #[napi(getter)]
    pub fn capture_count(&self) -> u32 {
        self.group_text.len().saturating_sub(1) as u32
    }

    /// Text of group `index` (matched or partial), or `null` if it did not
    /// participate.
    #[napi]
    pub fn group(&self, index: u32) -> Option<String> {
        self.group_text.get(index as usize).and_then(|o| o.clone())
    }

    /// Text of a named group (matched or partial), or `null`.
    #[napi]
    pub fn named_group(&self, name: String) -> Option<String> {
        let idx = *self.named.get(&name)?;
        self.group_text.get(idx as usize).and_then(|o| o.clone())
    }

    /// `"matched"`, `"partial"`, or `"none"` for group `index`.
    #[napi]
    pub fn group_state(&self, index: u32) -> String {
        match self.states.get(index as usize).copied().unwrap_or(2) {
            0 => "matched",
            1 => "partial",
            _ => "none",
        }
        .into()
    }
}

// ===========================================================================
// Module-level helpers
// ===========================================================================

/// Escape `s` so it matches literally as a regex pattern (aggressive mode).
#[napi]
pub fn escape(s: String) -> String {
    eregex::escape(&s)
}

/// Like `escape` but only escapes regex metacharacters, leaving other
/// punctuation alone.
#[napi]
pub fn escape_special_only(s: String) -> String {
    eregex::escape_special_only(&s)
}

/// Like `escape` but leaves spaces unescaped.
#[napi]
pub fn escape_literal_spaces(s: String) -> String {
    eregex::escape_literal_spaces(&s)
}

/// Convenience: `true` if `pattern` matches anywhere in `haystack`.
///
/// @throws {Error} if `pattern` is syntactically invalid.
#[napi]
pub fn is_match(pattern: String, haystack: String) -> Result<bool> {
    let re = eregex::Regex::new(&pattern).map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(re.is_match(&haystack))
}
