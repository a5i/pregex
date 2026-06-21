//! Tests transcribed from `.local/ex-partial-testcases.md`.
//!
//! That document describes matching an STM32 part number
//! (`STM32F407VGT6`) that may be **split by noise** in the input, e.g.
//! `"Microcontroller STM32F dutyu7 8 407VGT6   "`. Three behaviours are
//! exercised:
//!
//! * **Strict contiguous** ([`Regex::find`]) and **end-anchored partial**
//!   ([`Regex::find_partial`]) — handled directly by the engine: cases
//!   #1, #3, #4, #5, #6, #7, #8.
//! * **Gap-tolerant** — the noise-split cases #2, #9, #10. The engine does not
//!   (yet) support in-pattern gap matching (fuzzy/approximate matching is on
//!   the roadmap), but the *same observable behaviour* is achievable in user
//!   space: decompose the target into ordered sub-patterns, find each with
//!   [`Regex::find_at`] while advancing a cursor, and bound the noise between
//!   consecutive segments with `max_gap`. The small `gap_find` helper below
//!   implements exactly this; `examples/gap_match.rs` carries a richer demo.
//!
//! Pattern used throughout: `(STM32)(F[0-9]{3})([A-Z0-9]{4})` with groups
//! `STM32` / `F407` / `VGT6`. All haystacks are ASCII, so byte offsets equal
//! char offsets.
//!
//! # A note on the document's byte indices
//!
//! The source document's ruler is off-by-one past the first segment: it lists
//! the gap as `22..33` and the second segment as `33..40`, but the actual
//! string `"Microcontroller STM32F dutyu7 8 407VGT6   "` has the gap at
//! `22..32` (10 bytes) and `407VGT6` at `32..39` (7 bytes). The assertions
//! below use the engine-verified offsets and re-slice the gap/segments
//! directly out of the haystack to make the disagreement explicit.

use eregex::{MatchStatus, Regex};

fn re(p: &str) -> Regex {
    Regex::new(p).unwrap_or_else(|_| panic!("failed to compile {p:?}"))
}

const PAT: &str = r"(STM32)(F[0-9]{3})([A-Z0-9]{4})";

// ===========================================================================
// Gap-tolerant search helper (user-space workaround for fuzzy matching).
// A richer version lives in examples/gap_match.rs.
// ===========================================================================

/// A decomposed target: an ordered sub-pattern with a label.
struct Segment {
    re: Regex,
    label: &'static str,
}

/// The outcome of a successful gap-tolerant search.
struct GapMatch {
    /// The stitched-back target, e.g. `"STM32F407VGT6"`.
    reconstructed: String,
    /// `(label, start, end)` per segment, in haystack byte offsets.
    segments: Vec<(&'static str, usize, usize)>,
    /// `(start, end)` of each skipped noise gap, in haystack byte offsets.
    skipped: Vec<(usize, usize)>,
}

/// Find `segments` in order, allowing bounded noise between them.
///
/// * The first segment may start anywhere.
/// * Each later segment must start within `max_gap` bytes of the previous
///   segment's end; exceeding the limit returns `None` (the "gap too long"
///   case #9).
/// * Any non-empty gap between two consecutive segments is recorded in
///   `skipped`.
fn gap_find(haystack: &str, segments: &[Segment], max_gap: usize) -> Option<GapMatch> {
    let mut reconstructed = String::new();
    let mut hits = Vec::with_capacity(segments.len());
    let mut skipped = Vec::new();
    let mut cursor = 0usize; // byte offset to search from
    for (i, seg) in segments.iter().enumerate() {
        let m = seg.re.find_at(haystack, cursor)?;
        let (s, e) = (m.start(), m.end());
        if i > 0 {
            let gap = s - cursor;
            if gap > max_gap {
                return None;
            }
            if gap > 0 {
                skipped.push((cursor, s));
            }
        }
        hits.push((seg.label, s, e));
        reconstructed.push_str(m.as_str());
        cursor = e;
    }
    Some(GapMatch {
        reconstructed,
        segments: hits,
        skipped,
    })
}

/// The STM32 target decomposed at the only place noise is allowed: between the
/// series letter `F` and the three digits. This makes group 2 (`F407`) the
/// "split" group: `F` comes from segment 1's tail, `407` from segment 2's head.
fn stm32_segments() -> Vec<Segment> {
    vec![
        Segment {
            re: re(r"STM32F"),
            label: "STM32F",
        },
        Segment {
            re: re(r"[0-9]{3}[A-Z0-9]{4}"),
            label: "407VGT6",
        },
    ]
}

// ===========================================================================
// Strict / end-anchored cases (engine-native)
// ===========================================================================

// #1 — Ideal contiguous string: happy path, full match with all three groups.
#[test]
fn stm32_contiguous_full_match() {
    let r = re(PAT);
    let hay = "Microcontroller STM32F407VGT6";
    // The plain unanchored search finds the contiguous match.
    let m = r.find(hay).expect("full match");
    assert_eq!(m.as_str(), "STM32F407VGT6");
    assert_eq!(m.start(), 16);
    assert_eq!(m.group(1), Some("STM32"));
    assert_eq!(m.group(2), Some("F407"));
    assert_eq!(m.group(3), Some("VGT6"));
    // The end-anchored search agrees: this input *is* a full match.
    let p = r.find_partial(hay).expect("partial-or-full");
    assert!(p.is_full());
    assert_eq!(p.matched, "STM32F407VGT6");
}

// #3 — The noisy input must NOT match under strict semantics. This guards
// against confusing strict matching with the gap mode exercised below.
#[test]
fn stm32_noisy_input_strict_regex_no_match() {
    let r = re(PAT);
    let hay = "Microcontroller STM32F dutyu7 8 407VGT6   ";
    assert!(r.find(hay).is_none());
    assert!(!r.is_match(hay));
    assert!(r.find_partial(hay).is_none());
}

// #4 — Partial: input ends right after the series letter `F`.
#[test]
fn stm32_partial_after_series_letter() {
    let r = re(PAT);
    let m = r.find_partial("Microcontroller STM32F").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "STM32F");
    assert_eq!(m.start, 16);
    assert_eq!(m.end, 22);
    assert_eq!(m.group(1), Some("STM32"));
    assert!(m.group_matched(1));
    assert_eq!(m.group(2), Some("F"));
    assert!(m.group_partial(2));
    assert!(m.group_none(3));
}

// #5 — Partial: input ends mid-way through the series digits (`F40`).
#[test]
fn stm32_partial_inside_series_digits() {
    let r = re(PAT);
    let m = r.find_partial("Microcontroller STM32F40").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "STM32F40");
    assert_eq!(m.group(1), Some("STM32"));
    assert!(m.group_matched(1));
    assert_eq!(m.group(2), Some("F40"));
    assert!(m.group_partial(2));
    assert!(m.group_none(3));
}

// #6 — Partial: series complete, suffix incomplete (`VG`).
#[test]
fn stm32_partial_suffix() {
    let r = re(PAT);
    let m = r
        .find_partial("Microcontroller STM32F407VG")
        .expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "STM32F407VG");
    assert_eq!(m.group(1), Some("STM32"));
    assert!(m.group_matched(1));
    assert_eq!(m.group(2), Some("F407"));
    assert!(m.group_matched(2));
    assert_eq!(m.group(3), Some("VG"));
    assert!(m.group_partial(3));
}

// #7 — A wrong character inside the series (`X` where a digit is expected)
// rules out any continuation -> NoMatch, NOT Partial.
#[test]
fn stm32_wrong_char_inside_series() {
    let r = re(PAT);
    let hay = "Microcontroller STM32FX07VGT6";
    assert!(r.find(hay).is_none());
    assert!(r.find_partial(hay).is_none());
}

// #8 — Wrong family (`STM8` instead of `STM32`) -> NoMatch.
#[test]
fn stm32_wrong_family_no_match() {
    let r = re(PAT);
    let hay = "Microcontroller STM8F407VGT6";
    assert!(r.find(hay).is_none());
    assert!(r.find_partial(hay).is_none());
}

// ===========================================================================
// Gap-tolerant cases (user-space via gap_find — engine-native fuzzy is roadmap)
// ===========================================================================

// #2 — The main case: real noise between `F` and `407`. Group 2 (`F407`) is
// reconstructed from two non-contiguous haystack ranges.
#[test]
fn stm32_split_by_noise_between_f_and_407() {
    let hay = "Microcontroller STM32F dutyu7 8 407VGT6   ";
    let gm = gap_find(hay, &stm32_segments(), 16).expect("gap match");

    // The reconstruction stitches the segments back into the clean target.
    assert_eq!(gm.reconstructed, "STM32F407VGT6");

    // Segment ranges (ASCII ⇒ byte offsets == char offsets). NOTE: the source
    // document's ruler is off-by-one here — the engine-verified offsets are
    // 16..22 / 32..39, not 16..22 / 33..40.
    assert_eq!(gm.segments, vec![("STM32F", 16, 22), ("407VGT6", 32, 39)]);
    assert_eq!(&hay[16..22], "STM32F");
    assert_eq!(&hay[32..39], "407VGT6");

    // The skipped noise (10 bytes; the document's 22..33 is off-by-one).
    assert_eq!(gm.skipped, vec![(22, 32)]);
    assert_eq!(&hay[22..32], " dutyu7 8 ");

    // Re-validate against the full contiguous pattern: the reconstruction
    // must be a genuine Full match with clean capture groups.
    let full = re(PAT);
    let p = full
        .find_partial(&gm.reconstructed)
        .expect("reconstruction re-matches");
    assert_eq!(p.status, MatchStatus::Full);
    assert_eq!(p.group(1), Some("STM32"));
    assert_eq!(p.group(2), Some("F407"));
    assert_eq!(p.group(3), Some("VGT6"));

    // Group 2 (`F407`) is the "split" group: `F` from segment 1's tail and
    // `407` from segment 2's head — two non-contiguous haystack ranges.
    let (_, _, seg1_end) = gm.segments[0];
    let (_, seg2_start, _) = gm.segments[1];
    assert_eq!(&hay[seg1_end - 1..seg1_end], "F");
    assert_eq!(&hay[seg2_start..seg2_start + 3], "407");
}

// #9 — Noise too long for the configured max gap -> NoMatch even in gap mode.
#[test]
fn stm32_gap_too_long_no_match() {
    let hay = "Microcontroller STM32F very very very long unrelated text 407VGT6";
    // The noise run between `F` and `407` is far longer than 8 bytes.
    assert!(gap_find(hay, &stm32_segments(), 8).is_none());
    // Contrast: a generous budget accepts the same input.
    assert!(gap_find(hay, &stm32_segments(), 64).is_some());
}

// #10 — Gap allowed between `F` and the digits (same shape as #2, clean tail).
#[test]
fn stm32_gap_allowed_between_f_and_digits() {
    let hay = "Microcontroller STM32F dutyu7 8 407VGT6";
    let gm = gap_find(hay, &stm32_segments(), 16).expect("gap match");

    assert_eq!(gm.reconstructed, "STM32F407VGT6");
    assert_eq!(gm.segments, vec![("STM32F", 16, 22), ("407VGT6", 32, 39)]);
    assert_eq!(gm.skipped, vec![(22, 32)]);

    // Re-validation: clean Full match with the expected groups.
    let full = re(PAT);
    let p = full
        .find_partial(&gm.reconstructed)
        .expect("reconstruction re-matches");
    assert_eq!(p.status, MatchStatus::Full);
    assert_eq!(p.group(1), Some("STM32"));
    assert_eq!(p.group(2), Some("F407"));
    assert_eq!(p.group(3), Some("VGT6"));
}
