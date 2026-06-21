//! Pattern flags.
//!
//! Flags come in two flavours, matching mrab-regex:
//!
//! * **Scoped** flags may apply to only part of a pattern and can be turned on
//!   or off: `ASCII`, `FULLCASE`, `IGNORECASE`, `LOCALE`, `MULTILINE`,
//!   `DOTALL`, `UNICODE`, `VERBOSE`, `WORD`.
//! * **Global** flags apply to the entire pattern and can only be turned on:
//!   `VERSION0`, `VERSION1`.
//!
//! Inline syntax `(?im)`, `(?i-m:...)` toggles scoped flags for a subpattern.

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Not};

/// A bitset of zero or more [`Flags`](self) values.
///
/// Individual flag constants such as [`IGNORECASE`](Flags::IGNORECASE) live on
/// the `Flags` type itself. Use `a | b` to combine them and `a.contains(b)` to
/// test membership.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Flags(pub u32);

impl Flags {
    /// No flags set.
    pub const NONE: Flags = Flags(0);

    // ---- Scoped flags ----
    /// `(?i)` — case-insensitive matching.
    pub const IGNORECASE: Flags = Flags(1 << 0);
    /// `(?m)` — `^` and `$` match at line boundaries.
    pub const MULTILINE: Flags = Flags(1 << 1);
    /// `(?s)` — `.` matches any character, including newlines.
    pub const DOTALL: Flags = Flags(1 << 2);
    /// `(?u)` — use Unicode semantics for `\d \w \s \b`. (Default.)
    pub const UNICODE: Flags = Flags(1 << 3);
    /// `(?a)` — use ASCII-only semantics for `\d \w \s \b`.
    pub const ASCII: Flags = Flags(1 << 4);
    /// `(?x)` — free-spacing mode; whitespace and `#` comments are ignored.
    pub const VERBOSE: Flags = Flags(1 << 5);
    /// `(?f)` — full case-folding for case-insensitive matches.
    pub const FULLCASE: Flags = Flags(1 << 6);
    /// `(?w)` — Unicode default word-boundary semantics for `\b`/`\B`.
    pub const WORD: Flags = Flags(1 << 7);
    /// `(?L)` — locale-sensitive (legacy, limited support).
    pub const LOCALE: Flags = Flags(1 << 8);

    // ---- Global flags ----
    /// `(?V0)` — version 0 (legacy `re`-compatible) behaviour.
    pub const VERSION0: Flags = Flags(1 << 16);
    /// `(?V1)` — version 1 (enhanced) behaviour. This is the default.
    pub const VERSION1: Flags = Flags(1 << 17);

    /// Returns `true` if all the bits in `other` are set in `self`.
    #[inline]
    pub const fn contains(self, other: Flags) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns `true` if any of the bits in `other` are set in `self`.
    #[inline]
    pub const fn intersects(self, other: Flags) -> bool {
        (self.0 & other.0) != 0
    }

    /// Returns `true` if no flags are set.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Insert `other` into `self` (in place).
    #[inline]
    pub fn insert(&mut self, other: Flags) {
        self.0 |= other.0;
    }

    /// Remove `other` from `self` (in place).
    #[inline]
    pub fn remove(&mut self, other: Flags) {
        self.0 &= !other.0;
    }

    /// Returns the union of `self` and `other`.
    #[inline]
    pub const fn union(self, other: Flags) -> Flags {
        Flags(self.0 | other.0)
    }

    /// Returns the intersection of `self` and `other`.
    #[inline]
    pub const fn intersection(self, other: Flags) -> Flags {
        Flags(self.0 & other.0)
    }

    /// Returns `self` with the bits in `other` cleared.
    #[inline]
    pub const fn difference(self, other: Flags) -> Flags {
        Flags(self.0 & !other.0)
    }

    /// Raw bits.
    #[inline]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Returns the set of flags that are *scoped* (toggleable inline).
    pub const fn scoped(self) -> Flags {
        Flags(self.0 & Self::SCOPED_BITS)
    }

    /// Returns the set of flags that are *global* (whole-pattern only).
    pub const fn global(self) -> Flags {
        Flags(self.0 & Self::GLOBAL_BITS)
    }

    const SCOPED_BITS: u32 = 0x1FF; // bits 0..=8
    const GLOBAL_BITS: u32 = (1 << 16) | (1 << 17);
}

impl BitOr for Flags {
    type Output = Flags;
    #[inline]
    fn bitor(self, rhs: Flags) -> Flags {
        Flags(self.0 | rhs.0)
    }
}
impl BitOrAssign for Flags {
    #[inline]
    fn bitor_assign(&mut self, rhs: Flags) {
        self.0 |= rhs.0;
    }
}
impl BitAnd for Flags {
    type Output = Flags;
    #[inline]
    fn bitand(self, rhs: Flags) -> Flags {
        Flags(self.0 & rhs.0)
    }
}
impl BitAndAssign for Flags {
    #[inline]
    fn bitand_assign(&mut self, rhs: Flags) {
        self.0 &= rhs.0;
    }
}
impl BitXor for Flags {
    type Output = Flags;
    #[inline]
    fn bitxor(self, rhs: Flags) -> Flags {
        Flags(self.0 ^ rhs.0)
    }
}
impl Not for Flags {
    type Output = Flags;
    #[inline]
    fn not(self) -> Flags {
        Flags(!self.0)
    }
}

/// Resolve default behaviour for the ASCII/Unicode/LOCALE trio.
///
/// If none of `ASCII`, `UNICODE`, `LOCALE` is set, `UNICODE` is assumed.
pub(crate) fn resolve_defaults(mut f: Flags) -> Flags {
    if !f.intersects(Flags::ASCII | Flags::UNICODE | Flags::LOCALE) {
        f |= Flags::UNICODE;
    }
    if !f.intersects(Flags::VERSION0 | Flags::VERSION1) {
        f |= Flags::VERSION1; // enhanced behaviour is the default
    }
    f
}

// --- Free-standing flag aliases (mrab-style `flags::IGNORECASE`) ----------
//
// These mirror the associated constants on `Flags` so that patterns like
// `flags::IGNORECASE | flags::MULTILINE` read naturally.

/// `(?i)`
pub const IGNORECASE: Flags = Flags::IGNORECASE;
/// `(?m)`
pub const MULTILINE: Flags = Flags::MULTILINE;
/// `(?s)`
pub const DOTALL: Flags = Flags::DOTALL;
/// `(?u)`
pub const UNICODE: Flags = Flags::UNICODE;
/// `(?a)`
pub const ASCII: Flags = Flags::ASCII;
/// `(?x)`
pub const VERBOSE: Flags = Flags::VERBOSE;
/// `(?f)`
pub const FULLCASE: Flags = Flags::FULLCASE;
/// `(?w)`
pub const WORD: Flags = Flags::WORD;
/// `(?L)`
pub const LOCALE: Flags = Flags::LOCALE;
/// `(?V0)`
pub const VERSION0: Flags = Flags::VERSION0;
/// `(?V1)`
pub const VERSION1: Flags = Flags::VERSION1;
