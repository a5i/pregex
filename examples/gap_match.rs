//! Gap-tolerant matching built on top of `eregex`.
//!
//! The engine matches patterns *contiguously*. Real-world inputs (OCR output,
//! noisy logs) sometimes split an intended target across junk, e.g.
//!
//! ```text
//! "Microcontroller STM32F dutyu7 8 407VGT6   "
//! ```
//!
//! where the part number `STM32F407VGT6` is broken by the noise `" dutyu7 8 "`.
//! A plain `find` (and even the end-anchored `find_partial`) returns `None`
//! here, because the space after `F` is a hard mismatch and the match can
//! never reach end-of-input through the noise.
//!
//! True fuzzy / gap matching is on the roadmap (`Fuzzy / approximate matching`
//! in `README.md`). But you can get the **same observable behaviour today**
//! with a small "shift-and-resume" search:
//!
//! 1. Decompose the target into ordered sub-patterns (segments).
//! 2. Find each segment with [`Regex::find_at`], advancing a cursor.
//! 3. Bound the noise between consecutive segments with `max_gap`.
//! 4. Stitch the segment texts into the reconstructed target.
//!
//! [`Regex::find_partial`] is then used to (a) confirm the reconstruction is a
//! genuine *full* match (groups and all) and (b) detect the related end-of-input
//! *partial* cases (`STM32F`, `STM32F40`, `STM32F407VG`).
//!
//! Run with: `cargo run --example gap_match`
//!
//! # Caveats (read these before adopting the pattern)
//!
//! * Segment boundaries are **user-defined** — you decide where noise may
//!   occur. Picking them at group boundaries is usually right, but the choice
//!   is pattern-specific.
//! * Gaps are allowed **between** segments, never inside one. A wrong
//!   character *inside* a segment (e.g. `STM32FX07...`) makes that segment
//!   unfindable, yielding `None` — which happens to match the desired NoMatch,
//!   but for a slightly different reason than true fuzzy matching would give.
//! * This is a **heuristic reconstruction**, not a proof of equivalence with
//!   the original pattern. It can produce false positives at segment edges;
//!   always re-validate the stitched result with the full pattern (we do).

use eregex::{MatchStatus, Regex};

/// A decomposed target: ordered sub-patterns searched left-to-right.
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
    /// `(start, end)` of each skipped noise gap.
    skipped: Vec<(usize, usize)>,
}

/// Search `haystack` for `segments` in order.
///
/// * The first segment may start anywhere.
/// * Each later segment must start within `max_gap` bytes of the previous
///   segment's end. Exceeding the limit returns `None` (the "gap too long"
///   case #9).
/// * Any non-empty gap between two consecutive segments is recorded in
///   `skipped`.
fn gap_find(haystack: &str, segments: &[Segment], max_gap: usize) -> Option<GapMatch> {
    let mut reconstructed = String::new();
    let mut hits = Vec::with_capacity(segments.len());
    let mut skipped = Vec::new();
    let mut cursor = 0usize; // byte offset to search from

    for (i, seg) in segments.iter().enumerate() {
        // Shift-and-resume: find this segment at or after the cursor.
        let m = seg.re.find_at(haystack, cursor)?;
        let (s, e) = (m.start(), m.end());
        if i > 0 {
            let gap = s - cursor;
            if gap > max_gap {
                return None; // noise too long → NoMatch (#9)
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

/// The STM32 target, decomposed at the only place noise is allowed: right
/// after the series letter `F`, before the three digits. This makes group 2
/// (`F407`) the "split" group: `F` from segment 1, `407` from segment 2.
fn stm32_segments() -> Vec<Segment> {
    vec![
        Segment {
            re: Regex::new(r"STM32F").unwrap(),
            label: "STM32F",
        },
        Segment {
            re: Regex::new(r"[0-9]{3}[A-Z0-9]{4}").unwrap(),
            label: "407VGT6",
        },
    ]
}

/// The full, contiguous pattern used to re-validate the reconstruction and to
/// recover clean capture groups (STM32 / F407 / VGT6).
const FULL: &str = r"(STM32)(F[0-9]{3})([A-Z0-9]{4})";

fn report(name: &str, hay: &str, max_gap: usize) {
    println!("\n=== {name} ===");
    println!("haystack: {hay:?}");

    // (1) Show what the bare engine does — both strict and end-anchored partial.
    let full = Regex::new(FULL).unwrap();
    println!(
        "  find         : {:?}",
        full.find(hay).map(|m| m.as_str().to_string())
    );
    println!(
        "  find_partial : {:?}",
        match full.find_partial(hay) {
            None => "None".to_string(),
            Some(p) => format!("{:?} matched={:?}", p.status, p.matched),
        }
    );

    // (2) Gap-tolerant shift-and-resume search.
    match gap_find(hay, &stm32_segments(), max_gap) {
        Some(gm) => {
            println!("  gap_find     : OK");
            println!("    reconstructed = {:?}", gm.reconstructed);
            for (label, s, e) in &gm.segments {
                println!("    segment {label:<8} = [{s}..{e}] {:?}", &hay[*s..*e]);
            }
            for (s, e) in &gm.skipped {
                println!("    skipped        = [{s}..{e}] {:?}", &hay[*s..*e]);
            }

            // (3) Re-validate the reconstruction with the FULL pattern and
            //     report clean groups. This is also where `find_partial`
            //     earns its keep: it confirms Full vs Partial.
            let p = full
                .find_partial(&gm.reconstructed)
                .expect("reconstruction must re-match the full pattern");
            assert_eq!(p.status, MatchStatus::Full, "reconstruction must be Full");
            println!(
                "    groups = [STM32={:?}, F407={:?}, VGT6={:?}]",
                p.group(1),
                p.group(2),
                p.group(3),
            );

            // The caller asked specifically about #2: group 2 (`F407`) is
            // assembled from TWO non-contiguous haystack ranges — the `F` at
            // the tail of segment 1 and the `407` at the head of segment 2.
            if name.contains("#2") {
                let (_, _, seg1_end) = gm.segments[0];
                let (_, seg2_start, _) = gm.segments[1];
                println!(
                    "    group F407 split = [{}..{}] + [{}..{}]  (\"F\" + \"407\")",
                    seg1_end - 1,
                    seg1_end,
                    seg2_start,
                    seg2_start + 3
                );
            }
        }
        None => println!("  gap_find     : None (NoMatch)"),
    }
}

fn main() {
    // #2 / #10 — the main case: real noise between `F` and `407`.
    // Gap = the " dutyu7 8 " run (≤ 16 bytes), so it is accepted.
    report(
        "#2 split by noise (max_gap=16)",
        "Microcontroller STM32F dutyu7 8 407VGT6   ",
        16,
    );

    // #9 — same shape, but max_gap=8 rejects the run.
    report(
        "#9 gap too long (max_gap=8)",
        "Microcontroller STM32F very very very long unrelated text 407VGT6",
        8,
    );

    // #7 — a wrong char *inside* the digits segment (`X` where a digit is
    // expected). No `[0-9]{3}` run exists, so the segment search fails.
    report(
        "#7 wrong char inside series",
        "Microcontroller STM32FX07VGT6",
        16,
    );

    // #8 — wrong family (`STM8` instead of `STM32`): the first segment is
    // never found.
    report("#8 wrong family", "Microcontroller STM8F407VGT6", 16);

    // #1 — the contiguous happy path: no gap, reconstruction == input tail.
    report(
        "#1 contiguous full match",
        "Microcontroller STM32F407VGT6",
        16,
    );

    println!("\n--- end-of-input partials (find_partial alone, no gap mode) ---");
    let full = Regex::new(FULL).unwrap();
    for hay in [
        "Microcontroller STM32F",
        "Microcontroller STM32F40",
        "Microcontroller STM32F407VG",
    ] {
        let p = full.find_partial(hay).unwrap();
        println!(
            "  {:<32} -> {:?} matched={:?} g1={:?} g2={:?} g3={:?}",
            hay,
            p.status,
            p.matched,
            p.group(1),
            p.group(2),
            p.group(3)
        );
    }

    println!("\nAll reconstruction checks passed.");
}
