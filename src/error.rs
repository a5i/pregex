//! Error types for pattern parsing and matching.

use std::fmt;

/// A specialized [`Result`] for eregex operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The kind of error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// A syntax error in the pattern.
    Syntax(String),
    /// An unrecognized or unsupported escape sequence.
    BadEscape(String),
    /// An invalid character class.
    BadCharClass(String),
    /// A bad quantifier (e.g. `{3,2}` or `{}`).
    BadRepeat(String),
    /// A reference to an unknown group.
    BadGroupRef(String),
    /// A duplicate group name.
    DuplicateGroup(String),
    /// A flag-related error.
    BadFlag(String),
    /// A property name `\p{...}` is not recognized.
    BadProperty(String),
    /// The pattern is too deeply nested / too large.
    TooLarge(String),
    /// The match exceeded its time or step budget.
    Timeout,
}

/// An error produced while compiling or running a regex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// What kind of error this is.
    pub kind: ErrorKind,
    /// Byte offset into the pattern where the error was detected, if known.
    pub position: Option<usize>,
}

impl Error {
    /// Create a new syntax error at the given pattern byte offset.
    pub fn syntax_at(msg: impl Into<String>, pos: usize) -> Self {
        Error {
            kind: ErrorKind::Syntax(msg.into()),
            position: Some(pos),
        }
    }

    /// Create a new syntax error with no known position.
    pub fn syntax(msg: impl Into<String>) -> Self {
        Error {
            kind: ErrorKind::Syntax(msg.into()),
            position: None,
        }
    }

    /// Create an error of a specific kind at the given byte offset.
    pub fn at(kind: ErrorKind, pos: usize) -> Self {
        Error {
            kind,
            position: Some(pos),
        }
    }

    /// Create an error of a specific kind with no known position.
    pub fn new(kind: ErrorKind) -> Self {
        Error {
            kind,
            position: None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match &self.kind {
            ErrorKind::Syntax(s)
            | ErrorKind::BadEscape(s)
            | ErrorKind::BadCharClass(s)
            | ErrorKind::BadRepeat(s)
            | ErrorKind::BadGroupRef(s)
            | ErrorKind::DuplicateGroup(s)
            | ErrorKind::BadFlag(s)
            | ErrorKind::BadProperty(s)
            | ErrorKind::TooLarge(s) => s,
            ErrorKind::Timeout => "regex timed out",
        };
        match self.position {
            Some(p) => write!(f, "eregex error at position {p}: {msg}"),
            None => write!(f, "eregex error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
