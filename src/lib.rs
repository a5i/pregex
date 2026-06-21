//! `pregex` — an advanced regular expression engine for Rust.
//!
//! Inspired by the Python [`regex`](https://github.com/mrabarnett/mrab-regex)
//! module (mrab-regex), this crate provides a richer regex feature set than the
//! standard Rust `regex` crate, while keeping an ergonomic API.
//!
//! # Quick start
//!
//! ```
//! use pregex::Regex;
//!
//! let re = Regex::new(r"(\w+)\s+(\w+)").unwrap();
//! let m = re.find("hello world").unwrap();
//! assert_eq!(m.group(1), Some("hello"));
//! assert_eq!(m.group(2), Some("world"));
//! ```
//!
//! See the [`Regex`](crate::Regex) type and the module-level functions for the
//! full API. The [`flags`](crate::flags) module exposes the flag constants.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod charset;
pub mod error;
pub mod escape;
pub mod flags;
pub mod matcher;
pub mod unicode;

mod ast;
mod match_obj;
mod parser;
mod regex;
mod state;

pub use error::{Error, Result};
pub use escape::{escape, escape_literal_spaces, escape_special_only};
pub use flags::Flags;
pub use match_obj::{CaptureMatches, FindIter, Match};
pub use regex::Regex;

// ---------------------------------------------------------------------------
// Module-level convenience functions
// ---------------------------------------------------------------------------

/// Compile a pattern with the default flags.
///
/// ```
/// assert!(pregex::is_match(r"\d+", "abc 123"));
/// ```
pub fn new(pattern: &str) -> Result<Regex> {
    Regex::new(pattern)
}

/// Compile a pattern with the given flags.
pub fn new_with_flags(pattern: &str, flags: Flags) -> Result<Regex> {
    Regex::new_with_flags(pattern, flags)
}

/// Returns `true` if `pattern` matches anywhere in `haystack`.
pub fn is_match(pattern: &str, haystack: &str) -> bool {
    match Regex::new(pattern) {
        Ok(re) => re.is_match(haystack),
        Err(_) => false,
    }
}

/// Search for the first match of `pattern` in `haystack`.
pub fn find<'h>(pattern: &str, haystack: &'h str) -> Option<Match<'h>> {
    Regex::new(pattern).ok()?.find(haystack)
}

/// Collect every non-overlapping match of `pattern` in `haystack`.
///
/// (A borrowing iterator is available on [`Regex::find_iter`]; this
/// module-level helper collects, since the pattern is compiled transiently.)
pub fn find_all<'h>(pattern: &str, haystack: &'h str) -> Result<Vec<Match<'h>>> {
    let re = Regex::new(pattern)?;
    Ok(re.find_iter(haystack).collect())
}

/// Replace the first match of `pattern` in `haystack` using the template
/// `repl` (`$1`, `${name}`, `$$`).
pub fn replace(pattern: &str, haystack: &str, repl: &str) -> Result<String> {
    let re = Regex::new(pattern)?;
    Ok(re.replace(haystack, repl))
}

/// Replace all non-overlapping matches of `pattern` in `haystack`.
pub fn replace_all(pattern: &str, haystack: &str, repl: &str) -> Result<String> {
    let re = Regex::new(pattern)?;
    Ok(re.replace_all(haystack, repl))
}

/// Split `haystack` by `pattern`, returning the parts.
pub fn split(pattern: &str, haystack: &str) -> Result<Vec<String>> {
    let re = Regex::new(pattern)?;
    Ok(re.split(haystack))
}
