//! Character-property helpers built on top of `std`'s built-in Unicode tables.
//!
//! This avoids pulling in a heavyweight Unicode-data crate while still being
//! correct for the common properties. Full `\p{...}` coverage and full
//! case-folding are roadmap items.

/// Whether `c` is a digit (`\d`).
///
/// In ASCII mode this is `[0-9]`; in Unicode mode it is the Unicode
/// general category `Nd` (decimal number). Rust's `std` does not expose
/// `General_Category` directly, so the Unicode branch uses
/// [`char::is_numeric`], which covers `Nd + Nl + No`. This is slightly
/// broader than strict `Nd` (it also matches e.g. `½` and Roman numerals);
/// tightening this to exactly `Nd` is a roadmap item pending a Unicode
/// category table.
#[inline]
pub fn is_digit(c: char, ascii: bool) -> bool {
    if ascii {
        c.is_ascii_digit()
    } else {
        c.is_numeric()
    }
}

/// Whether `c` is a "word" character (`\w`).
#[inline]
pub fn is_word(c: char, ascii: bool) -> bool {
    if ascii {
        c.is_ascii_alphanumeric() || c == '_'
    } else {
        c.is_alphanumeric() || c == '_'
    }
}

/// Whether `c` is whitespace (`\s`).
#[inline]
pub fn is_space(c: char, ascii: bool) -> bool {
    if ascii {
        matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r')
    } else {
        // `char::is_whitespace` is Unicode White_Space aware.
        c.is_whitespace()
    }
}

/// Case-insensitive equality of two characters using simple Unicode casefolding.
///
/// Note: this uses `to_lowercase`; full casefolding (e.g. ß ↔ "ss") is a
/// roadmap item.
#[inline]
pub fn case_eq(a: char, b: char) -> bool {
    if a == b {
        return true;
    }
    let la = a.to_lowercase();
    let lb = b.to_lowercase();
    la.eq(lb)
}

/// Add the case-equivalents of `c` to `out`. Used to expand character classes
/// for case-insensitive matching.
pub fn push_case_variants(c: char, out: &mut Vec<char>) {
    out.push(c);
    for u in c.to_uppercase() {
        if u != c {
            out.push(u);
        }
    }
    for l in c.to_lowercase() {
        if l != c {
            out.push(l);
        }
    }
}

/// A character predicate returned by [`property`].
pub type PropFn = fn(char) -> bool;

/// Lookup a `\p{...}` / `\P{...}` property by its (normalized, lowercased,
/// no-spaces-or-underscores) name.
///
/// Returns a predicate over `char`. A curated subset of properties is
/// supported. Unknown names return `None`.
pub fn property(name: &str) -> Option<PropFn> {
    // Normalize: drop spaces, underscores and dashes; lowercase.
    let key: String = name
        .chars()
        .filter(|c| !matches!(c, ' ' | '_' | '-'))
        .flat_map(|c| c.to_lowercase())
        .collect();

    // Allow forms like "gc=nd" and "generalcategory=nd".
    let (prop, value) = match key.split_once('=') {
        Some((p, v)) => (p, v),
        None => ("", key.as_str()),
    };

    let pred: PropFn = match (prop, value) {
        ("", "l") | ("gc", "l") | ("", "letter") => |c| c.is_alphabetic(),
        ("", "lu") | ("gc", "lu") | ("", "uppercaseletter") => |c| c.is_uppercase(),
        ("", "ll") | ("gc", "ll") | ("", "lowercaseletter") => |c| c.is_lowercase(),
        ("", "n") | ("gc", "n") | ("", "number") => |c| c.is_numeric(),
        ("", "nd") | ("gc", "nd") | ("generalcategory", "nd") | ("", "decimalnumber") => |c| c.is_numeric(),
        ("", "nl") | ("gc", "nl") | ("", "letternumber") => |c| c.is_numeric(),
        ("", "no") | ("gc", "no") | ("", "othernumber") => |c| c.is_numeric(),
        ("", "p") | ("gc", "p") | ("", "punctuation") => {
            |c| matches!(c, '?'|'!'|'.'|','|';'|':'|'-'|'—'|'(' | ')'|'['|']'|'{'|'}'|'"'|'\''|'`'|'/'|'\\'|'…'|'‥')
        }
        ("", "z") | ("gc", "z") | ("", "separator") => {
            |c| matches!(c, ' ' | '\u{00a0}' | '\u{2028}' | '\u{2029}')
        }
        ("", "c") | ("gc", "c") | ("", "other") => |c| !c.is_alphanumeric() && !c.is_whitespace(),
        // Binary properties / aliases.
        ("", "alpha" | "alphabetic") => |c| c.is_alphabetic(),
        ("", "alnum" | "alphanumeric") => |c| c.is_alphanumeric(),
        ("", "upper" | "uppercase") => |c| c.is_uppercase(),
        ("", "lower" | "lowercase") => |c| c.is_lowercase(),
        ("", "space" | "whitespace") => |c| c.is_whitespace(),
        ("", "digit") => |c| c.is_numeric(),
        ("", "ascii") => |c| c.is_ascii(),
        ("", "blank") => |c| matches!(c, ' ' | '\t'),
        ("", "cntrl" | "control") => |c| c.is_control(),
        ("", "print" | "printable") => |c| !c.is_control(),
        ("", "graph") => |c| c.is_alphanumeric() || c.is_ascii_punctuation(),
        ("", "word") => |c| c.is_alphanumeric() || c == '_',
        ("", "xdigit" | "hexdigit") => |c| c.is_ascii_hexdigit(),
        // POSIX-style aliases (mrab-specific forms), normalized to no-underscore.
        ("", "posixalnum") => |c| c.is_alphanumeric(),
        ("", "posixdigit") => |c| c.is_numeric(),
        ("", "posixpunct") => |c| c.is_ascii_punctuation() || "\u{2010}\u{2011}\u{2012}\u{2013}\u{2014}\u{2015}\u{2212}".contains(c),
        ("", "posixxdigit") => |c| c.is_ascii_hexdigit(),
        _ => return None,
    };
    Some(pred)
}
