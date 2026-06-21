//! Python bindings for the [`eregex`](https://docs.rs/eregex) regular
//! expression engine, generated with [PyO3](https://pyo3.rs).
//!
//! This crate is a thin adapter: all matching logic lives in the `eregex`
//! core crate, and here we only translate its Rust types into Python-friendly
//! classes and functions. The wheel is built with
//! [`maturin`](https://www.maturin.rs); see `README.md` for usage.

#![deny(unsafe_code)]

use std::collections::HashMap;

use pyo3::exceptions::{PyIndexError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

// ===========================================================================
// Error helper + conversion helpers
// ===========================================================================

/// Map a `eregex::Error` into a `ValueError` carrying its display string.
fn map_err(e: eregex::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Convert an `Option<String>` into a `Py<PyAny>` (`None` if absent).
fn opt_to_py(py: Python<'_>, opt: Option<String>) -> PyResult<Py<PyAny>> {
    match opt {
        Some(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        None => Ok(py.None()),
    }
}

/// Normalize a (possibly negative) group index against the group count.
/// Raises ``IndexError`` on out-of-range (matching Python's ``re``).
fn normalize_index(i: isize, len: usize) -> PyResult<usize> {
    let idx = if i < 0 {
        let n = i
            .checked_add(len as isize)
            .ok_or_else(|| PyIndexError::new_err(format!("no such group: {i}")))?;
        if n < 0 {
            return Err(PyIndexError::new_err(format!("no such group: {i}")));
        }
        n as usize
    } else {
        i as usize
    };
    if idx >= len {
        return Err(PyIndexError::new_err(format!("no such group: {i}")));
    }
    Ok(idx)
}

// ===========================================================================
// Regex
// ===========================================================================

/// A compiled regular expression.
///
/// Compile once with ``Regex(pattern)`` or ``Regex(pattern, flags)``, where
/// ``flags`` is a bitwise OR of the module-level flag constants (or the
/// result of ``parse_flags("ims")``), then reuse it across many inputs via
/// methods like :meth:`find`, :meth:`is_match`, :meth:`findall`,
/// :meth:`find_partial`, :meth:`replace`, :meth:`replace_all`, and
/// :meth:`split`.
#[pyclass(frozen, name = "Regex")]
pub struct PyRegex {
    re: eregex::Regex,
}

#[pymethods]
impl PyRegex {
    /// Compile ``pattern``. ``flags`` (optional) is a bitwise OR of the
    /// module-level flag constants (e.g. ``IGNORECASE | MULTILINE``).
    ///
    /// :raises ValueError: if ``pattern`` is syntactically invalid.
    #[new]
    #[pyo3(signature = (pattern, flags=None))]
    fn new(pattern: &str, flags: Option<u32>) -> PyResult<Self> {
        let f = eregex::Flags(flags.unwrap_or(0));
        let re = eregex::Regex::new_with_flags(pattern, f).map_err(map_err)?;
        Ok(Self { re })
    }

    /// The original pattern string.
    #[getter]
    fn pattern(&self) -> String {
        self.re.as_str().to_string()
    }

    /// The resolved flags as a bitset.
    #[getter]
    fn flags(&self) -> u32 {
        self.re.flags().bits()
    }

    /// The number of capturing groups (group 0 is the whole match and is not
    /// counted here).
    #[getter]
    fn capture_count(&self) -> usize {
        self.re.capture_count()
    }

    /// Names of all named groups.
    fn group_names(&self) -> Vec<String> {
        self.re.group_names().keys().cloned().collect()
    }

    /// Index (1-based) of a named group, or ``None`` if it does not exist.
    fn group_index(&self, name: &str) -> Option<usize> {
        self.re.group_index(name)
    }

    /// ``True`` if the pattern matches anywhere in ``haystack``.
    fn is_match(&self, haystack: &str) -> bool {
        self.re.is_match(haystack)
    }

    /// First match anywhere in ``haystack``, or ``None``.
    fn find(&self, haystack: &str) -> Option<PyMatch> {
        let names = self.re.group_names().clone();
        self.re
            .find(haystack)
            .map(|m| PyMatch::from_match(haystack, &m, &names))
    }

    /// First match at or after byte offset ``start``, or ``None``.
    fn find_at(&self, haystack: &str, start: usize) -> Option<PyMatch> {
        let names = self.re.group_names().clone();
        self.re
            .find_at(haystack, start)
            .map(|m| PyMatch::from_match(haystack, &m, &names))
    }

    /// Match anchored at the start of ``haystack`` (like :func:`re.match`).
    fn match_at_start(&self, haystack: &str) -> Option<PyMatch> {
        let names = self.re.group_names().clone();
        self.re
            .match_at_start(haystack)
            .map(|m| PyMatch::from_match(haystack, &m, &names))
    }

    /// Match covering the whole ``haystack`` (like :func:`re.fullmatch`).
    #[pyo3(name = "fullmatch")]
    fn full_match(&self, haystack: &str) -> Option<PyMatch> {
        let names = self.re.group_names().clone();
        self.re
            .fullmatch(haystack)
            .map(|m| PyMatch::from_match(haystack, &m, &names))
    }

    /// All non-overlapping matches.
    #[pyo3(name = "findall")]
    fn find_all(&self, haystack: &str) -> Vec<PyMatch> {
        let names = self.re.group_names().clone();
        self.re
            .find_iter(haystack)
            .map(|m| PyMatch::from_match(haystack, &m, &names))
            .collect()
    }

    /// Partial / end-anchored match. Returns ``None`` if the input cannot be
    /// a prefix of any full match.
    fn find_partial(&self, haystack: &str) -> Option<PyPartialMatch> {
        let names = self.re.group_names().clone();
        self.re
            .find_partial(haystack)
            .map(|p| PyPartialMatch::from_partial(p, &names))
    }

    /// Replace the first match using template ``repl``
    /// (``$1``, ``${name}``, ``$$``).
    fn replace(&self, haystack: &str, repl: &str) -> String {
        self.re.replace(haystack, repl)
    }

    /// Replace every non-overlapping match using template ``repl``.
    fn replace_all(&self, haystack: &str, repl: &str) -> String {
        self.re.replace_all(haystack, repl)
    }

    /// Split ``haystack`` by this pattern, returning the parts.
    fn split(&self, haystack: &str) -> Vec<String> {
        self.re.split(haystack)
    }

    /// Pretty-print the parsed AST (debug aid).
    fn dump(&self) -> String {
        self.re.dump()
    }

    fn __repr__(&self) -> String {
        format!(
            "Regex(pattern={:?}, flags=0x{:x}, groups={})",
            self.re.as_str(),
            self.re.flags().bits(),
            self.re.capture_count()
        )
    }
}

// ===========================================================================
// Match
// ===========================================================================

/// A successful match, carrying the full capture state.
///
/// Supports ``len(m)``, ``m[index]`` (integer → group by number, string →
/// named group), and ``m.matched`` / ``m.start`` / ``m.end`` etc.
#[pyclass(frozen, name = "Match")]
pub struct PyMatch {
    input: String,
    groups: Vec<Option<String>>,
    spans: Vec<Option<(usize, usize)>>,
    captures: Vec<Vec<Option<String>>>,
    named: HashMap<String, usize>,
}

impl PyMatch {
    /// Build a Python `Match` from a borrowed `eregex::Match`. All data is
    /// cloned into owned form so the Python object is self-contained.
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
        let named = names.iter().map(|(k, &v)| (k.clone(), v)).collect();
        PyMatch {
            input: haystack.to_string(),
            groups,
            spans,
            captures,
            named,
        }
    }

    /// Internal span lookup (not exposed to Python directly).
    fn span_at(&self, g: usize) -> Option<(usize, usize)> {
        self.spans.get(g).and_then(|o| *o)
    }

    /// Resolve a group key (int or str) to an owned text (or ``None`` if the
    /// group did not participate). Raises ``IndexError`` for unknown names /
    /// out-of-range ints, ``TypeError`` for unsupported key types
    /// (matching Python's ``re`` semantics).
    fn lookup(&self, key: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
        if let Ok(s) = key.extract::<String>() {
            let idx = self
                .named
                .get(&s)
                .copied()
                .ok_or_else(|| PyIndexError::new_err(format!("no such group: {s}")))?;
            return Ok(self.groups.get(idx).and_then(|o| o.clone()));
        }
        if let Ok(i) = key.extract::<isize>() {
            let idx = normalize_index(i, self.groups.len())?;
            return Ok(self.groups.get(idx).and_then(|o| o.clone()));
        }
        Err(PyTypeError::new_err("group index must be int or str"))
    }
}

#[pymethods]
impl PyMatch {
    /// The whole match text (group 0).
    #[getter]
    fn matched(&self) -> String {
        self.groups
            .get(0)
            .and_then(|o| o.clone())
            .unwrap_or_default()
    }

    /// Alias of ``matched`` for mrab-regex familiarity.
    #[getter]
    fn group0(&self) -> String {
        self.matched()
    }

    /// The original input string this match was found in.
    #[getter]
    fn input(&self) -> String {
        self.input.clone()
    }

    /// Byte offset where the whole match starts.
    #[getter]
    fn start(&self) -> usize {
        self.start_of(0)
    }

    /// Byte offset where the whole match ends.
    #[getter]
    fn end(&self) -> usize {
        self.end_of(0)
    }

    /// The ``(start, end)`` byte span of the whole match.
    #[getter]
    fn span(&self) -> (usize, usize) {
        (self.start_of(0), self.end_of(0))
    }

    /// The number of capturing groups (group 0 not counted).
    #[getter]
    fn capture_count(&self) -> usize {
        self.groups.len().saturating_sub(1)
    }

    /// Current text of every group (group 0 first). Groups that did not
    /// participate are ``None``.
    #[getter]
    fn groups(&self) -> Vec<Option<String>> {
        self.groups.clone()
    }

    /// Map of named-group name to its current text.
    #[getter]
    fn named_groups<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (name, &idx) in &self.named {
            if let Some(Some(s)) = self.groups.get(idx).cloned() {
                d.set_item(name, s)?;
            }
        }
        Ok(d)
    }

    /// Captures of every group (group 0 first); each entry is a group's
    /// repeated-capture history. Non-participating iterations are ``None``.
    #[getter]
    fn all_captures(&self) -> Vec<Vec<Option<String>>> {
        self.captures.clone()
    }

    /// Map of named-group name to its repeated-capture history.
    #[getter]
    fn captures_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (name, &idx) in &self.named {
            if let Some(v) = self.captures.get(idx).cloned() {
                d.set_item(name, v)?;
            }
        }
        Ok(d)
    }

    /// Text of one or more groups.
    ///
    /// With no args: ``(whole_match, group1, group2, ...)``.
    /// With one int: text of that group (``0`` = whole match), or ``None``.
    /// With one str: text of that named group, or ``None``.
    /// With multiple args: a tuple of the per-arg results.
    #[pyo3(signature = (*args))]
    fn group<'py>(&self, py: Python<'py>, args: &Bound<'py, PyTuple>) -> PyResult<Py<PyAny>> {
        if args.is_empty() {
            let items: Vec<Py<PyAny>> = self
                .groups
                .iter()
                .map(|o| opt_to_py(py, o.clone()))
                .collect::<PyResult<_>>()?;
            return Ok(PyTuple::new(py, items)?.unbind().into());
        }
        if args.len() == 1 {
            let one = args.get_item(0)?;
            // Single-string / single-int form: return the bare value, not a
            // tuple, matching Python's `re` semantics.
            return opt_to_py(py, self.lookup(&one)?);
        }
        let mut out: Vec<Py<PyAny>> = Vec::with_capacity(args.len());
        for item in args {
            out.push(opt_to_py(py, self.lookup(&item)?)?);
        }
        Ok(PyTuple::new(py, out)?.unbind().into())
    }

    /// All captures (repeated-capture history) of group ``index``.
    fn captures(&self, index: usize) -> Vec<Option<String>> {
        self.captures.get(index).cloned().unwrap_or_default()
    }

    /// All captures of a named group.
    fn captures_by_name(&self, name: &str) -> Vec<Option<String>> {
        match self.named.get(name) {
            Some(&idx) => self.captures.get(idx).cloned().unwrap_or_default(),
            None => Vec::new(),
        }
    }

    /// ``(start, end)`` byte span of group ``index``, or ``None`` if it did
    /// not participate.
    #[pyo3(signature = (index=0))]
    fn span_of(&self, index: usize) -> Option<(usize, usize)> {
        self.span_at(index)
    }

    /// Start byte offset of group ``index`` (default 0). Returns the input
    /// length if the group did not participate (Python semantics).
    #[pyo3(signature = (index=0))]
    fn start_of(&self, index: usize) -> usize {
        match self.span_at(index) {
            Some((s, _)) => s,
            None => self.input.len(),
        }
    }

    /// End byte offset of group ``index`` (default 0).
    #[pyo3(signature = (index=0))]
    fn end_of(&self, index: usize) -> usize {
        match self.span_at(index) {
            Some((_, e)) => e,
            None => self.input.len(),
        }
    }

    fn __len__(&self) -> usize {
        self.groups.len()
    }

    fn __getitem__<'py>(&self, py: Python<'py>, key: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        opt_to_py(py, self.lookup(key)?)
    }

    fn __repr__(&self) -> String {
        let (s, e) = self.span_at(0).unwrap_or((0, 0));
        format!(
            "Match(matched={:?}, span=({}, {}), groups={})",
            self.groups.get(0).and_then(|o| o.as_deref()).unwrap_or(""),
            s,
            e,
            self.groups.len().saturating_sub(1)
        )
    }
}

// ===========================================================================
// PartialMatch
// ===========================================================================

/// A partial (or full) end-anchored match, the result of
/// :meth:`Regex.find_partial`. (``None`` from ``find_partial`` — not this
/// object — means the input cannot be a prefix of any match.)
#[pyclass(frozen, name = "PartialMatch")]
pub struct PyPartialMatch {
    status_full: bool,
    matched: String,
    start: usize,
    end: usize,
    /// Per-group state: 0=matched, 1=partial, 2=none.
    states: Vec<u8>,
    group_text: Vec<Option<String>>,
    named: HashMap<String, usize>,
}

impl PyPartialMatch {
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
        let named = names.iter().map(|(k, &v)| (k.clone(), v)).collect();
        PyPartialMatch {
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

#[pymethods]
impl PyPartialMatch {
    /// ``"full"`` or ``"partial"``.
    #[getter]
    fn status(&self) -> &'static str {
        if self.status_full {
            "full"
        } else {
            "partial"
        }
    }

    /// ``True`` if the match is fully satisfied.
    #[getter]
    fn is_full(&self) -> bool {
        self.status_full
    }

    /// ``True`` if the match was cut short by end-of-input.
    #[getter]
    fn is_partial(&self) -> bool {
        !self.status_full
    }

    /// The whole matched text.
    #[getter]
    fn matched(&self) -> String {
        self.matched.clone()
    }

    /// Byte offset where the match starts.
    #[getter]
    fn start(&self) -> usize {
        self.start
    }

    /// Byte offset where the match ends (always the input length).
    #[getter]
    fn end(&self) -> usize {
        self.end
    }

    /// The number of capturing groups (group 0 not counted).
    #[getter]
    fn capture_count(&self) -> usize {
        self.group_text.len().saturating_sub(1)
    }

    /// Text of group ``index`` (matched or partial), or ``None`` if it did
    /// not participate.
    #[pyo3(signature = (index=0))]
    fn group(&self, index: usize) -> Option<String> {
        self.group_text.get(index).and_then(|o| o.clone())
    }

    /// Text of a named group (matched or partial), or ``None``.
    fn named_group(&self, name: &str) -> Option<String> {
        let idx = *self.named.get(name)?;
        self.group_text.get(idx).and_then(|o| o.clone())
    }

    /// ``"matched"``, ``"partial"``, or ``"none"`` for group ``index``.
    #[pyo3(signature = (index=0))]
    fn group_state(&self, index: usize) -> &'static str {
        match self.states.get(index).copied().unwrap_or(2) {
            0 => "matched",
            1 => "partial",
            _ => "none",
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PartialMatch(matched={:?}, status={:?}, span=({}, {}))",
            self.matched,
            if self.status_full { "full" } else { "partial" },
            self.start,
            self.end
        )
    }
}

// ===========================================================================
// Module-level helpers
// ===========================================================================

/// Escape ``s`` so it matches literally as a regex pattern (aggressive mode).
#[pyfunction(name = "escape")]
fn py_escape(s: &str) -> String {
    eregex::escape(s)
}

/// Like :func:`escape` but only escapes regex metacharacters, leaving other
/// punctuation alone.
#[pyfunction(name = "escape_special_only")]
fn py_escape_special_only(s: &str) -> String {
    eregex::escape_special_only(s)
}

/// Like :func:`escape` but leaves spaces unescaped.
#[pyfunction(name = "escape_literal_spaces")]
fn py_escape_literal_spaces(s: &str) -> String {
    eregex::escape_literal_spaces(s)
}

/// Convenience: ``True`` if ``pattern`` matches anywhere in ``haystack``.
///
/// :raises ValueError: if ``pattern`` is syntactically invalid.
#[pyfunction(name = "is_match")]
fn py_is_match(pattern: &str, haystack: &str) -> PyResult<bool> {
    let re = eregex::Regex::new(pattern).map_err(map_err)?;
    Ok(re.is_match(haystack))
}

/// Convert a flag string such as ``"ims"`` into a flags bitset (a bitwise OR
/// of the module-level flag constants).
///
/// Recognized letters (case-insensitive): ``i m s u a x f w l``.
/// The ``RegExp``-style ``g``, ``y``, ``d`` are accepted but ignored
/// (familiarity).
///
/// :raises ValueError: on an unknown flag character.
#[pyfunction(name = "parse_flags")]
fn py_parse_flags(flag_str: &str) -> PyResult<u32> {
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
                return Err(PyValueError::new_err(format!(
                    "unknown flag character {other:?}"
                )))
            }
        }
    }
    Ok(f.bits())
}

/// Compile ``pattern`` into a :class:`Regex`.
///
/// :raises ValueError: if ``pattern`` is syntactically invalid.
#[pyfunction(name = "compile")]
#[pyo3(signature = (pattern, flags=None))]
fn py_compile(pattern: &str, flags: Option<u32>) -> PyResult<PyRegex> {
    PyRegex::new(pattern, flags)
}

// ===========================================================================
// Module
// ===========================================================================

/// eregex — an advanced regular expression engine (Python bindings).
///
/// This module exposes the Rust `eregex` engine to Python. Compile a pattern
/// with :class:`Regex`, then use methods like :meth:`Regex.find`,
/// :meth:`Regex.is_match`, :meth:`Regex.find_partial`, :meth:`Regex.replace`
/// / :meth:`Regex.replace_all`, and :meth:`Regex.split`.
#[pymodule]
fn eregex_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_escape, m)?)?;
    m.add_function(wrap_pyfunction!(py_escape_special_only, m)?)?;
    m.add_function(wrap_pyfunction!(py_escape_literal_spaces, m)?)?;
    m.add_function(wrap_pyfunction!(py_is_match, m)?)?;
    m.add_function(wrap_pyfunction!(py_parse_flags, m)?)?;
    m.add_function(wrap_pyfunction!(py_compile, m)?)?;

    m.add_class::<PyRegex>()?;
    m.add_class::<PyMatch>()?;
    m.add_class::<PyPartialMatch>()?;

    // Flag constants. `flags` returns resolved flags (with defaults
    // UNICODE + VERSION1 added), so users compare via bitwise AND.
    m.add("IGNORECASE", eregex::flags::IGNORECASE.bits())?;
    m.add("MULTILINE", eregex::flags::MULTILINE.bits())?;
    m.add("DOTALL", eregex::flags::DOTALL.bits())?;
    m.add("UNICODE", eregex::flags::UNICODE.bits())?;
    m.add("ASCII", eregex::flags::ASCII.bits())?;
    m.add("VERBOSE", eregex::flags::VERBOSE.bits())?;
    m.add("FULLCASE", eregex::flags::FULLCASE.bits())?;
    m.add("WORD", eregex::flags::WORD.bits())?;
    m.add("LOCALE", eregex::flags::LOCALE.bits())?;
    m.add("VERSION0", eregex::flags::VERSION0.bits())?;
    m.add("VERSION1", eregex::flags::VERSION1.bits())?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
