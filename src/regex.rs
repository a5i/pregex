//! The [`Regex`] type: a compiled pattern and its matching API.

use std::collections::HashMap;

use crate::ast::Node;
use crate::error::Result;
use crate::flags::Flags;
use crate::match_obj::{FindIter, GroupMatch, Match, MatchStatus, PartialMatch};
use crate::matcher;
use crate::state::State;

/// A compiled regular expression.
///
/// A `Regex` is obtained by compiling a pattern with [`Regex::new`] (default
/// flags) or [`Regex::new_with_flags`], then used to search text via methods
/// like [`find`](Self::find), [`is_match`](Self::is_match),
/// [`find_iter`](Self::find_iter), [`find_partial`](Self::find_partial),
/// [`replace`](Self::replace) and [`split`](Self::split).
///
/// Compilation is somewhat expensive; compile once and reuse the same
/// `Regex` across many inputs. A `Regex` is `Send` + `Sync` once compiled
/// (it owns no thread-local state) and is cheap to share by reference.
///
/// # Examples
///
/// ```
/// use eregex::{flags, Regex};
///
/// let re = Regex::new(r"(\w+)@(\w+)")?;
/// let m = re.find("ping alice@work").unwrap();
/// assert_eq!(m.group(1), Some("alice"));
/// assert_eq!(m.group(2), Some("work"));
/// # Ok::<(), eregex::Error>(())
/// ```
pub struct Regex {
    pattern: String,
    ast: Box<Node>,
    flags: Flags,
    n_groups: usize,
    names: HashMap<String, usize>,
}

impl Regex {
    /// Compile `pattern` with the default flags.
    ///
    /// # Errors
    ///
    /// Returns [`Error`](crate::Error) if `pattern` is syntactically invalid (unbalanced
    /// parentheses, bad escape sequences, invalid quantifiers, unknown
    /// group names, etc.). See [`ErrorKind`](crate::error::ErrorKind) for the
    /// full list.
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::Regex;
    ///
    /// let re = Regex::new(r"\d{3}-\d{2}")?;
    /// assert!(re.is_match("123-45"));
    /// assert!(Regex::new(r"(").is_err());
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn new(pattern: &str) -> Result<Regex> {
        Self::new_with_flags(pattern, Flags::NONE)
    }

    /// Compile `pattern` with the given [`Flags`].
    ///
    /// # Errors
    ///
    /// Returns [`Error`](crate::Error) on the same conditions as [`Regex::new`].
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::{flags, Regex};
    ///
    /// let re = Regex::new_with_flags(r"hello", flags::IGNORECASE)?;
    /// assert_eq!(re.find("HELLO, World").unwrap().as_str(), "HELLO");
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn new_with_flags(pattern: &str, flags: Flags) -> Result<Regex> {
        let parsed = crate::parser::parse(pattern, flags)?;
        Ok(Regex {
            pattern: pattern.to_string(),
            ast: Box::new(parsed.node),
            flags: parsed.flags,
            n_groups: parsed.n_groups,
            names: parsed.names,
        })
    }

    /// The original pattern string.
    pub fn as_str(&self) -> &str {
        &self.pattern
    }

    /// The resolved flags in effect.
    pub fn flags(&self) -> Flags {
        self.flags
    }

    /// The number of capturing groups.
    pub fn capture_count(&self) -> usize {
        self.n_groups
    }

    /// A map from group name to group index.
    pub fn group_names(&self) -> &HashMap<String, usize> {
        &self.names
    }

    /// Look up a group index by name.
    pub fn group_index(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }

    pub(crate) fn names_clone(&self) -> HashMap<String, usize> {
        self.names.clone()
    }

    /// Pretty-print the parsed AST (debug aid).
    pub fn dump(&self) -> String {
        let mut s = String::new();
        let _ = self.ast.dump(&mut s, 0);
        s
    }

    // -- internal helpers --------------------------------------------------

    fn build_state(&self, haystack: &str) -> State {
        let mut chars = Vec::with_capacity(haystack.len());
        let mut c2b = Vec::with_capacity(haystack.len() + 1);
        for (b, c) in haystack.char_indices() {
            chars.push(c);
            c2b.push(b);
        }
        c2b.push(haystack.len());
        State::new(chars, c2b, self.n_groups)
    }

    fn match_from_state<'h>(&self, haystack: &'h str, st: &State) -> Match<'h> {
        Match {
            haystack,
            char_to_byte: st.char_to_byte.clone(),
            caps: st.caps.clone(),
            log: st.log.clone(),
            names: self.names_clone(),
        }
    }

    /// Search forward from `from` (char index). On success the [`State`] is
    /// left with group 0 (and all participating captures) filled.
    pub(crate) fn find_from(&self, st: &mut State, from: usize) -> Option<(usize, usize)> {
        let n = st.len();
        let mut start = from;
        while start <= n {
            st.reset_for_search(start);
            if matcher::try_match(&self.ast, st) {
                let end = st.pos;
                st.caps[0] = Some((start, end));
                st.log[0].push((start, end));
                return Some((start, end));
            }
            start += 1;
        }
        None
    }

    // -- matching API ------------------------------------------------------

    /// Returns `true` if the pattern matches anywhere in `haystack`.
    ///
    /// Cheaper than [`find`](Self::find) when the match text is not needed —
    /// it short-circuits as soon as any match is found.
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::Regex;
    /// let re = Regex::new(r"\d+")?;
    /// assert!(re.is_match("abc 123"));
    /// assert!(!re.is_match("no digits here"));
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn is_match(&self, haystack: &str) -> bool {
        let mut st = self.build_state(haystack);
        self.find_from(&mut st, 0).is_some()
    }

    /// Search for the first match, anywhere in `haystack`. Returns [`None`] if
    /// there is no match. The returned [`Match`] carries the whole-match span
    /// and all capture groups.
    ///
    /// For end-anchored / partial matching use [`find_partial`](Self::find_partial).
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::Regex;
    /// let re = Regex::new(r"([a-z]+)(\d+)")?;
    /// let m = re.find("x abc42 y").unwrap();
    /// assert_eq!(m.as_str(), "abc42");
    /// assert_eq!(m.start(), 2);
    /// assert_eq!(m.group(2), Some("42"));
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn find<'h>(&self, haystack: &'h str) -> Option<Match<'h>> {
        let mut st = self.build_state(haystack);
        self.find_from(&mut st, 0)?;
        Some(self.match_from_state(haystack, &st))
    }

    /// Search for the first match at or after byte offset `start`.
    ///
    /// `start` is clamped to `haystack.len()`; it counts bytes, not chars,
    /// but may fall in the middle of a UTF-8 sequence only if it lies beyond
    /// the haystack end. Useful for resuming a scan after a previous match.
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::Regex;
    /// let re = Regex::new(r"\d+")?;
    /// let hay = "a1 b2 c3";
    /// let first = re.find(hay).unwrap();
    /// let next = re.find_at(hay, first.end()).unwrap();
    /// assert_eq!(next.as_str(), "2");
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn find_at<'h>(&self, haystack: &'h str, start: usize) -> Option<Match<'h>> {
        let mut st = self.build_state(haystack);
        let clamp = start.min(haystack.len());
        let start_char = haystack[..clamp].chars().count();
        self.find_from(&mut st, start_char)?;
        Some(self.match_from_state(haystack, &st))
    }

    /// Like [`find`](Self::find) — included for symmetry with `captures_iter`.
    pub fn captures<'h>(&self, haystack: &'h str) -> Option<Match<'h>> {
        self.find(haystack)
    }

    /// Require a match anchored at the **start** of the haystack (like
    /// Python's `re.match`). The match need not reach the end.
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::Regex;
    /// let re = Regex::new(r"\d+")?;
    /// assert_eq!(re.match_at_start("123abc").unwrap().as_str(), "123");
    /// assert!(re.match_at_start("abc123").is_none());
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn match_at_start<'h>(&self, haystack: &'h str) -> Option<Match<'h>> {
        let mut st = self.build_state(haystack);
        st.reset_for_search(0);
        if matcher::try_match(&self.ast, &mut st) {
            let end = st.pos;
            st.caps[0] = Some((0, end));
            st.log[0].push((0, end));
            Some(self.match_from_state(haystack, &st))
        } else {
            None
        }
    }

    /// Require a match that covers the **entire** haystack (Python's
    /// `re.fullmatch`).
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::Regex;
    /// let re = Regex::new(r"\d{3}")?;
    /// assert!(re.fullmatch("123").is_some());
    /// assert!(re.fullmatch("1234").is_none());
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn fullmatch<'h>(&self, haystack: &'h str) -> Option<Match<'h>> {
        let mut st = self.build_state(haystack);
        let n = st.len();
        st.reset_for_search(0);
        if matcher::try_match_to(&self.ast, &mut st, n) {
            let end = st.pos;
            st.caps[0] = Some((0, end));
            st.log[0].push((0, end));
            Some(self.match_from_state(haystack, &st))
        } else {
            None
        }
    }

    /// Iterate over non-overlapping matches.
    ///
    /// Zero-width matches are yielded once per position and do not loop
    /// infinitely: after one, the iterator advances past the next character.
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::Regex;
    /// let re = Regex::new(r"\d+")?;
    /// let ms: Vec<_> = re.find_iter("a1 bb 22").map(|m| m.as_str().to_string()).collect();
    /// assert_eq!(ms, vec!["1", "22"]);
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn find_iter<'r, 'h>(&'r self, haystack: &'h str) -> FindIter<'r, 'h> {
        FindIter {
            re: self,
            haystack,
            st: self.build_state(haystack),
            pos: 0,
            last_end: None,
        }
    }

    /// Iterate over non-overlapping matches (alias of [`find_iter`](Self::find_iter)).
    pub fn captures_iter<'r, 'h>(&'r self, haystack: &'h str) -> FindIter<'r, 'h> {
        self.find_iter(haystack)
    }

    /// Partial / end-anchored match.
    ///
    /// Unlike [`find`](Self::find), the match must consume the haystack all the
    /// way to its end. The result is:
    ///
    /// * `None` — the input cannot be a prefix of any match (a hard mismatch
    ///   occurred before end-of-input), or nothing matched at all.
    /// * `Some(Full)` — the pattern matched and consumed the entire haystack.
    /// * `Some(Partial)` — the pattern consumed the entire haystack but was
    ///   still asking for more input; in other words, the haystack is a prefix
    ///   of some full match.
    ///
    /// Group state within a [`PartialMatch`] distinguishes groups that
    /// completed ([`GroupMatch::Matched`]) from the group that was entered but
    /// not completed ([`GroupMatch::Partial`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::{MatchStatus, Regex};
    /// let re = Regex::new(r"token=([a-z]+)([0-9]+)")?;
    ///
    /// // Incomplete input — more could turn it into a full match.
    /// let p = re.find_partial("x token=abc").unwrap();
    /// assert_eq!(p.status, MatchStatus::Partial);
    /// assert_eq!(p.group(1), Some("abc")); // fully matched
    /// assert_eq!(p.group(2), Some(""));    // entered but empty
    ///
    /// // A wrong character rules out any continuation -> no match at all.
    /// assert!(re.find_partial("x token=abc!").is_none());
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn find_partial<'h>(&self, haystack: &'h str) -> Option<PartialMatch<'h>> {
        let mut st = self.build_state(haystack);
        let n = st.len();
        let char_to_byte = st.char_to_byte.clone();
        for start in 0..=n {
            st.reset_for_search(start);
            st.partial_mode = true;
            if matcher::try_match_to(&self.ast, &mut st, n) {
                st.caps[0] = Some((start, n));
                // Represent the completed match as a candidate with no open
                // groups, so build_partial_match has a single code path.
                let full = crate::state::PartialCandidate {
                    end: n,
                    completed: st.caps.iter().filter(|c| c.is_some()).count(),
                    caps: st.caps.clone(),
                    open: Vec::new(),
                };
                return Some(self.build_partial_match(
                    haystack,
                    &char_to_byte,
                    MatchStatus::Full,
                    start,
                    n,
                    &full,
                ));
            }
            if let Some(cand) = st.partial_best.take() {
                // Partial blocks only ever happen at end-of-input, i.e. at `n`.
                // Require a non-empty match (something was actually consumed).
                if cand.end == n && n > start {
                    return Some(self.build_partial_match(
                        haystack,
                        &char_to_byte,
                        MatchStatus::Partial,
                        start,
                        cand.end,
                        &cand,
                    ));
                }
            }
        }
        None
    }

    /// Build a [`PartialMatch`] from a [`PartialCandidate`].
    fn build_partial_match<'h>(
        &self,
        haystack: &'h str,
        char_to_byte: &[usize],
        status: MatchStatus,
        start_char: usize,
        end_char: usize,
        cand: &crate::state::PartialCandidate,
    ) -> PartialMatch<'h> {
        let bs = char_to_byte[start_char];
        let be = char_to_byte[end_char];
        let matched = &haystack[bs..be];
        let groups = (0..=self.n_groups)
            .map(|g| {
                if g == 0 {
                    // Group 0 is the whole (consumed) match.
                    return GroupMatch::Matched(matched);
                }
                // A partial (open) group wins over a completed capture — this
                // matters for repeated groups whose last iteration is partial.
                if let Some(&(_, gstart)) = cand.open.iter().rev().find(|(idx, _)| *idx == g) {
                    let gs = char_to_byte[gstart];
                    return GroupMatch::Partial(&haystack[gs..be]);
                }
                if let Some(Some((s, e))) = cand.caps.get(g).copied() {
                    return GroupMatch::Matched(&haystack[char_to_byte[s]..char_to_byte[e]]);
                }
                GroupMatch::None
            })
            .collect();
        PartialMatch {
            status,
            matched,
            start: bs,
            end: be,
            groups,
            names: self.names_clone(),
        }
    }

    // -- substitution / splitting -----------------------------------------

    /// Replace the first match with the expansion of `repl`.
    ///
    /// `repl` supports `$1`/`${name}`/`$&`/`$$` templates (see the module docs
    /// for the full syntax). If nothing matches, `haystack` is returned
    /// unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::Regex;
    /// let re = Regex::new(r"(\w+) (\w+)")?;
    /// assert_eq!(re.replace("hello world", "$2 $1"), "world hello");
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn replace(&self, haystack: &str, repl: &str) -> String {
        let mut st = self.build_state(haystack);
        match self.find_from(&mut st, 0) {
            Some((s, e)) => {
                let m = self.match_from_state(haystack, &st);
                let bs = st.char_to_byte[s];
                let be = st.char_to_byte[e];
                let mut out = String::with_capacity(haystack.len());
                out.push_str(&haystack[..bs]);
                out.push_str(&expand(repl, &m));
                out.push_str(&haystack[be..]);
                out
            }
            None => haystack.to_string(),
        }
    }

    /// Replace every non-overlapping match with the expansion of `repl`.
    ///
    /// See [`replace`](Self::replace) for the template syntax.
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::Regex;
    /// let re = Regex::new(r"(?P<a>\d)(?P<b>\d)")?;
    /// assert_eq!(re.replace_all("12 34", "${b}${a}"), "21 43");
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn replace_all(&self, haystack: &str, repl: &str) -> String {
        let mut out = String::with_capacity(haystack.len());
        let mut st = self.build_state(haystack);
        let mut cursor = 0usize; // char index
        loop {
            match self.find_from(&mut st, cursor) {
                Some((s, e)) => {
                    let bs = st.char_to_byte[s];
                    let m = self.match_from_state(haystack, &st);
                    // Append text before the match.
                    out.push_str(&haystack[st.char_to_byte[cursor]..bs]);
                    out.push_str(&expand(repl, &m));
                    if e == s {
                        // Zero-width match: emit the char we skip over, then advance.
                        if e < st.len() {
                            let skip_bs = st.char_to_byte[e];
                            let skip_be = st.char_to_byte[e + 1];
                            out.push_str(&haystack[skip_bs..skip_be]);
                        }
                        cursor = e + 1;
                    } else {
                        cursor = e;
                    }
                    if cursor > st.len() {
                        break;
                    }
                }
                None => {
                    out.push_str(&haystack[st.char_to_byte[cursor]..]);
                    break;
                }
            }
        }
        out
    }

    /// Split `haystack` by this pattern, including capturing-group text
    /// between parts (matching Python's `re.split` semantics).
    ///
    /// # Examples
    ///
    /// ```
    /// use eregex::Regex;
    /// let re = Regex::new(r"\s+")?;
    /// assert_eq!(re.split("a  b c"), vec!["a", "b", "c"]);
    /// # Ok::<(), eregex::Error>(())
    /// ```
    pub fn split(&self, haystack: &str) -> Vec<String> {
        self.split_iter(haystack).collect()
    }

    /// An iterator yielding the pieces produced by [`split`](Self::split).
    pub fn split_iter<'r, 'h>(&'r self, haystack: &'h str) -> SplitIter<'r, 'h> {
        SplitIter {
            re: self,
            haystack,
            st: self.build_state(haystack),
            cursor: 0,
            pending: Vec::new(),
            done: false,
        }
    }
}

impl std::fmt::Debug for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Regex")
            .field("pattern", &self.pattern)
            .field("flags", &self.flags.bits())
            .field("groups", &self.n_groups)
            .finish()
    }
}

/// Expand a replacement template against a match.
///
/// Supported syntax:
/// * `$1`, `$12` — group by number
/// * `${name}` or `${12}` — group by name or number
/// * `$&` or `$0` — the whole match
/// * `$$` — a literal `$`
/// * `\1`..`\9` — group by number (mrab/Python style)
/// * `\g<n>` / `\g<name>` / `\g'n'` — group by number or name
/// * `\\`, `\n`, `\t`, `\r` — escaped specials
fn expand(repl: &str, m: &Match<'_>) -> String {
    let chars: Vec<char> = repl.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '$' => {
                i += 1;
                if i >= chars.len() {
                    out.push('$');
                    break;
                }
                match chars[i] {
                    '$' => {
                        out.push('$');
                        i += 1;
                    }
                    '&' => {
                        out.push_str(m.as_str());
                        i += 1;
                    }
                    '{' => {
                        i += 1;
                        let mut name = String::new();
                        while i < chars.len() && chars[i] != '}' {
                            name.push(chars[i]);
                            i += 1;
                        }
                        if i < chars.len() {
                            i += 1; // consume '}'
                        }
                        append_group(&mut out, m, &name);
                    }
                    d if d.is_ascii_digit() => {
                        let mut num = String::new();
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            num.push(chars[i]);
                            i += 1;
                        }
                        append_group(&mut out, m, &num);
                    }
                    other => {
                        out.push('$');
                        out.push(other);
                        i += 1;
                    }
                }
            }
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    out.push('\\');
                    break;
                }
                match chars[i] {
                    '\\' => {
                        out.push('\\');
                        i += 1;
                    }
                    'n' => {
                        out.push('\n');
                        i += 1;
                    }
                    't' => {
                        out.push('\t');
                        i += 1;
                    }
                    'r' => {
                        out.push('\r');
                        i += 1;
                    }
                    'g' => {
                        i += 1;
                        if i < chars.len() && (chars[i] == '<' || chars[i] == '\'') {
                            let close = if chars[i] == '<' { '>' } else { '\'' };
                            i += 1;
                            let mut name = String::new();
                            while i < chars.len() && chars[i] != close {
                                name.push(chars[i]);
                                i += 1;
                            }
                            if i < chars.len() {
                                i += 1;
                            }
                            append_group(&mut out, m, &name);
                        }
                    }
                    d if d.is_ascii_digit() => {
                        let mut num = String::new();
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            num.push(chars[i]);
                            i += 1;
                        }
                        append_group(&mut out, m, &num);
                    }
                    other => {
                        out.push('\\');
                        out.push(other);
                        i += 1;
                    }
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

fn append_group(out: &mut String, m: &Match<'_>, name: &str) {
    if name.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(n) = name.parse::<usize>() {
            if let Some(s) = m.group(n) {
                out.push_str(s);
            }
            return;
        }
    }
    if let Some(s) = m.name(name) {
        out.push_str(s);
    }
}

// ---------------------------------------------------------------------------
// Split iterator
// ---------------------------------------------------------------------------

/// Iterator produced by [`Regex::split_iter`].
pub struct SplitIter<'r, 'h> {
    re: &'r Regex,
    haystack: &'h str,
    st: State,
    cursor: usize, // char index
    pending: Vec<String>,
    done: bool,
}

impl<'r, 'h> Iterator for SplitIter<'r, 'h> {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        if let Some(p) = self.pending.pop() {
            return Some(p);
        }
        if self.done {
            return None;
        }
        match self.re.find_from(&mut self.st, self.cursor) {
            Some((s, e)) => {
                let bs = self.st.char_to_byte[self.cursor];
                let be = self.st.char_to_byte[s];
                let piece = self.haystack[bs..be].to_string();
                // Queue the capturing groups of this match (Python semantics).
                for g in 1..=self.re.n_groups {
                    let m = self.re.match_from_state(self.haystack, &self.st);
                    let grp = m.group(g).map(str::to_string).unwrap_or_default();
                    self.pending.insert(0, grp);
                }
                if e == s {
                    // Zero-width split: keep the skipped char and advance.
                    if e < self.st.len() {
                        let skip = self.haystack
                            [self.st.char_to_byte[e]..self.st.char_to_byte[e + 1]]
                            .to_string();
                        self.pending.insert(0, skip);
                    }
                    self.cursor = e + 1;
                } else {
                    self.cursor = e;
                }
                Some(piece)
            }
            None => {
                let bs = self.st.char_to_byte[self.cursor];
                let piece = self.haystack[bs..].to_string();
                self.done = true;
                Some(piece)
            }
        }
    }
}

// flags module re-export for `Regex::flags()` consumers.
pub use crate::flags as flags_module;
#[allow(unused_imports)]
use flags_module as _flags;
