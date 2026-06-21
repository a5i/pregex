//! WebAssembly bindings for the [`eregex`](https://docs.rs/eregex) regular
//! expression engine, generated with [`wasm-bindgen`](https://rustwasm.github.io/wasm-bindgen/).
//!
//! This crate is a thin adapter over the `eregex` core: all matching logic
//! lives there, and here we only translate its Rust types into JavaScript-
//! friendly classes, objects and functions. The public API mirrors the native
//! Node bindings (`eregex-node`, via napi-rs) method-for-method so that code
//! written against one works unchanged against the other — see `README.md`.
//
// `wasm-bindgen` itself is FFI glue and uses `unsafe` internally, but none of
// the code in *this* file does. `deny` lets the generated traits override the
// lint where they must, while still flagging any `unsafe` we add ourselves.

#![deny(unsafe_code)]

use std::collections::BTreeMap;

use serde::Serialize;
use wasm_bindgen::prelude::*;

// ===========================================================================
// Helpers
// ===========================================================================

/// Serialize a plain Rust value into a JS value via `serde-wasm-bindgen`.
///
/// We route every nullable / compound return through this (rather than letting
/// `wasm-bindgen` convert `Option<T>` directly) so that `None` becomes JS
/// `null` — matching the native napi-rs bindings. `wasm-bindgen`'s native
/// `Option<T>` mapping yields `undefined`, which would break `=== null` and
/// `deepStrictEqual(..., null)` for anyone treating this package as a drop-in
/// for `eregex`.
fn to_js<T: Serialize>(value: &T) -> JsValue {
    serde_wasm_bindgen::to_value(value)
        .expect("eregex-wasm: serializing a plain value to JsValue cannot fail")
}

// ===========================================================================
// Flag constants
// ===========================================================================
//
// `wasm-bindgen` cannot export `pub const` values directly, so the flag bits
// are returned as a plain JS object from `flags()`. The package's `index.js`
// entry spreads that object onto the module so callers see the same
// `eregex.IGNORECASE` / `eregex.MULTILINE` / ... numeric properties as in the
// native Node bindings.

#[derive(Serialize)]
#[allow(non_snake_case)]
struct FlagsExport {
    IGNORECASE: u32,
    MULTILINE: u32,
    DOTALL: u32,
    UNICODE: u32,
    ASCII: u32,
    VERBOSE: u32,
    FULLCASE: u32,
    WORD: u32,
    LOCALE: u32,
    VERSION0: u32,
    VERSION1: u32,
}

/// All flag bits as a plain JS object (`{ IGNORECASE, MULTILINE, ... }`).
/// This is spread onto the package's exports by `index.js` so the constants
/// are available as `eregex.IGNORECASE` etc.
#[wasm_bindgen]
pub fn flags() -> JsValue {
    to_js(&FlagsExport {
        IGNORECASE: eregex::flags::IGNORECASE.bits(),
        MULTILINE: eregex::flags::MULTILINE.bits(),
        DOTALL: eregex::flags::DOTALL.bits(),
        UNICODE: eregex::flags::UNICODE.bits(),
        ASCII: eregex::flags::ASCII.bits(),
        VERBOSE: eregex::flags::VERBOSE.bits(),
        FULLCASE: eregex::flags::FULLCASE.bits(),
        WORD: eregex::flags::WORD.bits(),
        LOCALE: eregex::flags::LOCALE.bits(),
        VERSION0: eregex::flags::VERSION0.bits(),
        VERSION1: eregex::flags::VERSION1.bits(),
    })
}

/// Convert a flag string such as `"ims"` into a flags bitset (a bitwise OR of
/// the exported `IGNORECASE` / `MULTILINE` / ... constants).
///
/// Recognized letters (case-insensitive): `i m s u a x f w l`.
/// The `RegExp`-style `g`, `y`, `d` are accepted but ignored (familiarity).
///
/// @throws {Error} on an unknown flag character.
#[wasm_bindgen(js_name = parseFlags)]
pub fn parse_flags(flag_str: String) -> Result<u32, JsError> {
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
            other => return Err(JsError::new(&format!("unknown flag character {other:?}"))),
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
#[wasm_bindgen]
pub struct Regex {
    re: eregex::Regex,
}

#[wasm_bindgen]
impl Regex {
    /// Compile `pattern`. `flags` (optional) is a bitwise OR of the
    /// `eregex.IGNORECASE` / `MULTILINE` / ... constants (or the result of
    /// `eregex.parseFlags("ims")`).
    ///
    /// @throws {Error} if `pattern` is syntactically invalid.
    #[wasm_bindgen(constructor)]
    pub fn new(pattern: String, flags: Option<u32>) -> Result<Regex, JsError> {
        let f = eregex::Flags(flags.unwrap_or(0));
        let re =
            eregex::Regex::new_with_flags(&pattern, f).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self { re })
    }

    /// The original pattern string.
    #[wasm_bindgen(getter)]
    pub fn pattern(&self) -> String {
        self.re.as_str().to_string()
    }

    /// The resolved flags as a bitset.
    #[wasm_bindgen(getter)]
    pub fn flags(&self) -> u32 {
        self.re.flags().bits()
    }

    /// The number of capturing groups (group 0 is the whole match and is not
    /// counted here).
    #[wasm_bindgen(getter)]
    pub fn capture_count(&self) -> u32 {
        self.re.capture_count() as u32
    }

    /// Names of all named groups.
    #[wasm_bindgen(js_name = groupNames)]
    pub fn group_names(&self) -> Vec<String> {
        self.re.group_names().keys().cloned().collect()
    }

    /// Index (1-based) of a named group, or `null` if it does not exist.
    #[wasm_bindgen(js_name = groupIndex)]
    pub fn group_index(&self, name: String) -> JsValue {
        match self.re.group_index(&name) {
            Some(i) => JsValue::from(i as u32),
            None => JsValue::NULL,
        }
    }

    /// `true` if the pattern matches anywhere in `haystack`.
    #[wasm_bindgen(js_name = isMatch)]
    pub fn is_match(&self, haystack: String) -> bool {
        self.re.is_match(&haystack)
    }

    /// First match anywhere in `haystack`, or `null`.
    #[wasm_bindgen]
    pub fn find(&self, haystack: String) -> JsValue {
        let names = self.re.group_names().clone();
        match self.re.find(&haystack) {
            None => JsValue::NULL,
            Some(m) => JsValue::from(Match::from_match(&haystack, &m, &names)),
        }
    }

    /// First match at or after byte offset `start`, or `null`.
    #[wasm_bindgen(js_name = findAt)]
    pub fn find_at(&self, haystack: String, start: u32) -> JsValue {
        let names = self.re.group_names().clone();
        match self.re.find_at(&haystack, start as usize) {
            None => JsValue::NULL,
            Some(m) => JsValue::from(Match::from_match(&haystack, &m, &names)),
        }
    }

    /// Match anchored at the start of `haystack` (Python's `re.match`).
    #[wasm_bindgen(js_name = matchAtStart)]
    pub fn match_at_start(&self, haystack: String) -> JsValue {
        let names = self.re.group_names().clone();
        match self.re.match_at_start(&haystack) {
            None => JsValue::NULL,
            Some(m) => JsValue::from(Match::from_match(&haystack, &m, &names)),
        }
    }

    /// Match covering the whole `haystack` (Python's `re.fullmatch`).
    #[wasm_bindgen(js_name = fullMatch)]
    pub fn full_match(&self, haystack: String) -> JsValue {
        let names = self.re.group_names().clone();
        match self.re.fullmatch(&haystack) {
            None => JsValue::NULL,
            Some(m) => JsValue::from(Match::from_match(&haystack, &m, &names)),
        }
    }

    /// All non-overlapping matches.
    #[wasm_bindgen(js_name = findAll)]
    pub fn find_all(&self, haystack: String) -> Vec<Match> {
        let names = self.re.group_names().clone();
        self.re
            .find_iter(&haystack)
            .map(|m| Match::from_match(&haystack, &m, &names))
            .collect()
    }

    /// Partial / end-anchored match. Returns `null` if the input cannot be a
    /// prefix of any full match.
    #[wasm_bindgen(js_name = findPartial)]
    pub fn find_partial(&self, haystack: String) -> JsValue {
        let names = self.re.group_names().clone();
        match self.re.find_partial(&haystack) {
            None => JsValue::NULL,
            Some(p) => JsValue::from(PartialMatch::from_partial(p, &names)),
        }
    }

    /// Replace the first match using template `repl` (`$1`, `${name}`, `$$`).
    #[wasm_bindgen]
    pub fn replace(&self, haystack: String, repl: String) -> String {
        self.re.replace(&haystack, &repl)
    }

    /// Replace every non-overlapping match using template `repl`.
    #[wasm_bindgen(js_name = replaceAll)]
    pub fn replace_all(&self, haystack: String, repl: String) -> String {
        self.re.replace_all(&haystack, &repl)
    }

    /// Split `haystack` by this pattern, returning the parts.
    #[wasm_bindgen]
    pub fn split(&self, haystack: String) -> Vec<String> {
        self.re.split(&haystack)
    }

    /// Pretty-print the parsed AST (debug aid).
    #[wasm_bindgen]
    pub fn dump(&self) -> String {
        self.re.dump()
    }
}

// ===========================================================================
// Match
// ===========================================================================

/// A `[start, end]` byte span, the shape of `Match.span` and `Match.spanOf`.
#[derive(Serialize)]
struct SpanDto {
    start: u32,
    end: u32,
}

/// A successful match, carrying the full capture state.
#[wasm_bindgen]
pub struct Match {
    input: String,
    groups: Vec<Option<String>>,
    spans: Vec<Option<(usize, usize)>>,
    captures: Vec<Vec<Option<String>>>,
    // Named groups in declaration order (so `namedGroups` /
    // `capturesDict` preserve a stable, deterministic key order).
    named: Vec<(String, usize)>,
}

impl Match {
    /// Build a `Match` from a borrowed `eregex::Match`. All data is cloned
    /// into owned form so the JS object is self-contained.
    fn from_match(
        haystack: &str,
        m: &eregex::Match,
        names: &std::collections::HashMap<String, usize>,
    ) -> Self {
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
        let named = names.iter().map(|(k, &v)| (k.clone(), v)).collect();
        Match {
            input: haystack.to_string(),
            groups,
            spans,
            captures,
            named,
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

#[wasm_bindgen]
impl Match {
    /// The whole match text (group 0).
    #[wasm_bindgen(getter)]
    pub fn matched(&self) -> String {
        self.groups
            .first()
            .and_then(|o| o.clone())
            .unwrap_or_default()
    }

    /// The original input string this match was found in.
    #[wasm_bindgen(getter)]
    pub fn input(&self) -> String {
        self.input.clone()
    }

    /// Byte offset where the whole match starts.
    #[wasm_bindgen(getter)]
    pub fn start(&self) -> u32 {
        self.span_start(0)
    }

    /// Byte offset where the whole match ends.
    #[wasm_bindgen(getter)]
    pub fn end(&self) -> u32 {
        self.span_end(0)
    }

    /// The `{ start, end }` byte span of the whole match.
    #[wasm_bindgen(getter)]
    pub fn span(&self) -> JsValue {
        to_js(&SpanDto {
            start: self.span_start(0),
            end: self.span_end(0),
        })
    }

    /// The number of capturing groups (group 0 not counted).
    #[wasm_bindgen(getter)]
    pub fn capture_count(&self) -> u32 {
        self.groups.len().saturating_sub(1) as u32
    }

    /// Current text of every group (group 0 first). Groups that did not
    /// participate are `null`.
    #[wasm_bindgen(getter)]
    pub fn groups(&self) -> JsValue {
        to_js(&self.groups)
    }

    /// Map of named-group name to its current text.
    #[wasm_bindgen(getter, js_name = namedGroups)]
    pub fn named_groups(&self) -> JsValue {
        let map: BTreeMap<&str, &str> = self
            .named
            .iter()
            .filter_map(|(name, idx)| {
                self.groups
                    .get(*idx)
                    .and_then(|o| o.as_ref().map(|s| (name.as_str(), s.as_str())))
            })
            .collect();
        to_js(&map)
    }

    /// Captures of every group (group 0 first); each entry is a group's
    /// repeated-capture history. Non-participating iterations are `null`.
    #[wasm_bindgen(getter, js_name = allCaptures)]
    pub fn all_captures(&self) -> JsValue {
        to_js(&self.captures)
    }

    /// Map of named-group name to its repeated-capture history.
    #[wasm_bindgen(getter, js_name = capturesDict)]
    pub fn captures_dict(&self) -> JsValue {
        let map: BTreeMap<&str, &[Option<String>]> = self
            .named
            .iter()
            .filter_map(|(name, idx)| {
                self.captures
                    .get(*idx)
                    .map(|v| (name.as_str(), v.as_slice()))
            })
            .collect();
        to_js(&map)
    }

    /// Text of group `index` (0 = whole match), or `null` if it did not
    /// participate.
    #[wasm_bindgen]
    pub fn group(&self, index: u32) -> JsValue {
        to_js(&self.groups.get(index as usize).and_then(|o| o.clone()))
    }

    /// Text of a named group, or `null`.
    #[wasm_bindgen(js_name = namedGroup)]
    pub fn named_group(&self, name: String) -> JsValue {
        let value = self
            .named
            .iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, idx)| self.groups.get(*idx).and_then(|o| o.clone()));
        to_js(&value)
    }

    /// All captures (repeated-capture history) of group `index`.
    #[wasm_bindgen]
    pub fn captures(&self, index: u32) -> JsValue {
        let value = self
            .captures
            .get(index as usize)
            .cloned()
            .unwrap_or_default();
        to_js(&value)
    }

    /// All captures of a named group.
    #[wasm_bindgen(js_name = capturesByName)]
    pub fn captures_by_name(&self, name: String) -> JsValue {
        let value = self
            .named
            .iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, idx)| self.captures.get(*idx).cloned())
            .unwrap_or_default();
        to_js(&value)
    }

    /// `{ start, end }` byte span of group `index`, or `null` if it did not
    /// participate.
    #[wasm_bindgen(js_name = spanOf)]
    pub fn span_of(&self, index: u32) -> JsValue {
        let value = self
            .spans
            .get(index as usize)
            .and_then(|o| *o)
            .map(|(s, e)| SpanDto {
                start: s as u32,
                end: e as u32,
            });
        to_js(&value)
    }
}

// ===========================================================================
// PartialMatch
// ===========================================================================

/// A partial (or full) end-anchored match, the result of
/// [`Regex::find_partial`]. (`null` from `findPartial` — not this object —
/// means the input cannot be a prefix of any match.)
#[wasm_bindgen]
pub struct PartialMatch {
    status_full: bool,
    matched: String,
    start: usize,
    end: usize,
    states: Vec<u8>, // 0=matched, 1=partial, 2=none
    group_text: Vec<Option<String>>,
    named: Vec<(String, usize)>,
}

impl PartialMatch {
    fn from_partial(
        p: eregex::PartialMatch,
        names: &std::collections::HashMap<String, usize>,
    ) -> Self {
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
        let named = names.iter().map(|(k, &v)| (k.clone(), v)).collect();
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

#[wasm_bindgen]
impl PartialMatch {
    /// `"full"` or `"partial"`.
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> String {
        if self.status_full {
            "full".into()
        } else {
            "partial".into()
        }
    }

    /// `true` if the match is fully satisfied.
    #[wasm_bindgen(getter, js_name = isFull)]
    pub fn is_full(&self) -> bool {
        self.status_full
    }

    /// `true` if the match was cut short by end-of-input.
    #[wasm_bindgen(getter, js_name = isPartial)]
    pub fn is_partial(&self) -> bool {
        !self.status_full
    }

    /// The whole matched text.
    #[wasm_bindgen(getter)]
    pub fn matched(&self) -> String {
        self.matched.clone()
    }

    /// Byte offset where the match starts.
    #[wasm_bindgen(getter)]
    pub fn start(&self) -> u32 {
        self.start as u32
    }

    /// Byte offset where the match ends (always the input length).
    #[wasm_bindgen(getter)]
    pub fn end(&self) -> u32 {
        self.end as u32
    }

    /// The number of capturing groups (group 0 not counted).
    #[wasm_bindgen(getter)]
    pub fn capture_count(&self) -> u32 {
        self.group_text.len().saturating_sub(1) as u32
    }

    /// Text of group `index` (matched or partial), or `null` if it did not
    /// participate.
    #[wasm_bindgen]
    pub fn group(&self, index: u32) -> JsValue {
        to_js(&self.group_text.get(index as usize).and_then(|o| o.clone()))
    }

    /// Text of a named group (matched or partial), or `null`.
    #[wasm_bindgen(js_name = namedGroup)]
    pub fn named_group(&self, name: String) -> JsValue {
        let value = self
            .named
            .iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, idx)| self.group_text.get(*idx).and_then(|o| o.clone()));
        to_js(&value)
    }

    /// `"matched"`, `"partial"`, or `"none"` for group `index`.
    #[wasm_bindgen(js_name = groupState)]
    pub fn group_state(&self, index: u32) -> String {
        match self.states.get(index as usize).copied().unwrap_or(2) {
            0 => "matched".into(),
            1 => "partial".into(),
            _ => "none".into(),
        }
    }
}

// ===========================================================================
// Module-level helpers
// ===========================================================================

/// Escape `s` so it matches literally as a regex pattern (aggressive mode).
#[wasm_bindgen]
pub fn escape(s: String) -> String {
    eregex::escape(&s)
}

/// Like `escape` but only escapes regex metacharacters, leaving other
/// punctuation alone.
#[wasm_bindgen(js_name = escapeSpecialOnly)]
pub fn escape_special_only(s: String) -> String {
    eregex::escape_special_only(&s)
}

/// Like `escape` but leaves spaces unescaped.
#[wasm_bindgen(js_name = escapeLiteralSpaces)]
pub fn escape_literal_spaces(s: String) -> String {
    eregex::escape_literal_spaces(&s)
}

/// Convenience: `true` if `pattern` matches anywhere in `haystack`.
///
/// @throws {Error} if `pattern` is syntactically invalid.
#[wasm_bindgen(js_name = isMatch)]
pub fn is_match(pattern: String, haystack: String) -> Result<bool, JsError> {
    let re = eregex::Regex::new(&pattern).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(re.is_match(&haystack))
}
