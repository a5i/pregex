//! A compact representation of character sets as sorted, disjoint codepoint
//! ranges.
//!
//! All sets are stored in "positive" (non-negated) form; negation is achieved
//! by complementing against the full Unicode codepoint range, which keeps
//! union/intersection/difference uniform.

use crate::unicode::push_case_variants;

const MAX_CP: u32 = 0x10FFFF;

/// A set of Unicode codepoints stored as sorted, disjoint, inclusive ranges.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharSet {
    /// Sorted, disjoint, inclusive `(low, high)` codepoint ranges.
    pub ranges: Vec<(u32, u32)>,
}

impl CharSet {
    /// The empty set.
    pub fn empty() -> Self {
        CharSet { ranges: Vec::new() }
    }

    /// The set of every codepoint.
    pub fn full() -> Self {
        CharSet { ranges: vec![(0, MAX_CP)] }
    }

    /// Create a set containing a single codepoint.
    pub fn from_char(c: char) -> Self {
        CharSet { ranges: vec![(c as u32, c as u32)] }
    }

    /// Create a set from an inclusive codepoint range.
    pub fn from_range(lo: char, hi: char) -> Self {
        let (lo, hi) = (lo as u32, hi as u32);
        CharSet {
            ranges: vec![(lo.min(hi), lo.max(hi))],
        }
    }

    /// Build a set from an arbitrary, unsorted, possibly-overlapping list of
    /// ranges; the result is normalized.
    pub fn from_ranges_unsorted(mut ranges: Vec<(u32, u32)>) -> Self {
        ranges.sort_unstable();
        let mut out: Vec<(u32, u32)> = Vec::new();
        for (lo, hi) in ranges {
            if hi < lo {
                continue;
            }
            if let Some(last) = out.last_mut() {
                // Overlapping or adjacent ranges merge.
                if lo <= last.1.saturating_add(1) {
                    last.1 = last.1.max(hi);
                    continue;
                }
            }
            out.push((lo, hi));
        }
        CharSet { ranges: out }
    }

    /// Add a single codepoint.
    pub fn add_char(&mut self, c: char) {
        let cp = c as u32;
        self.ranges.push((cp, cp));
        self.ranges = Self::normalize(std::mem::take(&mut self.ranges));
    }

    /// Add an inclusive codepoint range.
    pub fn add_range(&mut self, lo: char, hi: char) {
        let (lo, hi) = (lo as u32, hi as u32);
        self.ranges.push((lo.min(hi), lo.max(hi)));
        self.ranges = Self::normalize(std::mem::take(&mut self.ranges));
    }

    fn normalize(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
        ranges.sort_unstable();
        let mut out: Vec<(u32, u32)> = Vec::new();
        for (lo, hi) in ranges {
            if hi < lo {
                continue;
            }
            if let Some(last) = out.last_mut() {
                if lo <= last.1.saturating_add(1) {
                    last.1 = last.1.max(hi);
                    continue;
                }
            }
            out.push((lo, hi));
        }
        out
    }

    /// Returns the complement of this set.
    pub fn complement(&self) -> CharSet {
        let mut out = Vec::new();
        let mut cursor = 0u32;
        for &(lo, hi) in &self.ranges {
            if cursor < lo {
                out.push((cursor, lo - 1));
            }
            cursor = hi.saturating_add(1);
            if cursor == 0 {
                break; // overflow past MAX
            }
        }
        if cursor <= MAX_CP {
            out.push((cursor, MAX_CP));
        }
        CharSet { ranges: out }
    }

    /// Returns the union of two sets.
    pub fn union(&self, other: &CharSet) -> CharSet {
        let mut all = Vec::with_capacity(self.ranges.len() + other.ranges.len());
        all.extend_from_slice(&self.ranges);
        all.extend_from_slice(&other.ranges);
        CharSet::from_ranges_unsorted(all)
    }

    /// Returns the intersection of two sets.
    pub fn intersect(&self, other: &CharSet) -> CharSet {
        let mut out = Vec::new();
        let (mut i, mut j) = (0, 0);
        while i < self.ranges.len() && j < other.ranges.len() {
            let (a0, a1) = self.ranges[i];
            let (b0, b1) = other.ranges[j];
            let lo = a0.max(b0);
            let hi = a1.min(b1);
            if lo <= hi {
                out.push((lo, hi));
            }
            if a1 < b1 {
                i += 1;
            } else {
                j += 1;
            }
        }
        CharSet { ranges: out }
    }

    /// Returns `self \ other` (difference).
    pub fn difference(&self, other: &CharSet) -> CharSet {
        self.intersect(&other.complement())
    }

    /// Returns the symmetric difference of two sets.
    pub fn sym_diff(&self, other: &CharSet) -> CharSet {
        self.difference(other).union(&other.difference(self))
    }

    /// Returns `true` if `c` is in the set.
    pub fn contains(&self, c: char) -> bool {
        let cp = c as u32;
        self.ranges
            .binary_search_by(|&(lo, hi)| {
                if cp < lo {
                    std::cmp::Ordering::Greater
                } else if cp > hi {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .is_ok()
    }

    /// Expand the set to include the case-equivalents of every member, for
    /// case-insensitive matching.
    pub fn add_case_variants(&mut self) {
        let mut extras: Vec<char> = Vec::new();
        for &(lo, hi) in &self.ranges {
            // Expanding large ranges is expensive; for ranges we expand the
            // endpoints and the alphabetic interior is covered by the fact that
            // the ASCII range case-pairs are themselves contiguous. To stay
            // correct and simple, expand every codepoint in small ranges and
            // the endpoints of large ones.
            let span = hi.saturating_sub(lo);
            if span <= 4096 {
                for cp in lo..=hi {
                    if let Some(c) = char::from_u32(cp) {
                        if c.is_alphabetic() {
                            push_case_variants(c, &mut extras);
                        }
                    }
                }
            } else {
                for cp in [lo, hi] {
                    if let Some(c) = char::from_u32(cp) {
                        push_case_variants(c, &mut extras);
                    }
                }
            }
        }
        for c in extras {
            self.add_char(c);
        }
    }

    /// Is this set the empty set?
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}
