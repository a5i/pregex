//! Unit tests for flag bitset operations and module-level flag constants.

use eregex::flags::{self, Flags};

#[test]
fn flag_constants_distinct() {
    let all = [
        flags::IGNORECASE,
        flags::MULTILINE,
        flags::DOTALL,
        flags::UNICODE,
        flags::ASCII,
        flags::VERBOSE,
        flags::FULLCASE,
        flags::WORD,
        flags::LOCALE,
        flags::VERSION0,
        flags::VERSION1,
    ];
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a.bits(), b.bits(), "two flag constants share bits");
        }
    }
}

#[test]
fn contains_intersects() {
    let f = flags::IGNORECASE | flags::MULTILINE;
    assert!(f.contains(flags::IGNORECASE));
    assert!(f.contains(flags::MULTILINE));
    assert!(!f.contains(flags::DOTALL));
    assert!(f.intersects(flags::IGNORECASE | flags::DOTALL)); // shares IGNORECASE
    assert!(!f.intersects(flags::DOTALL | flags::VERBOSE));
}

#[test]
fn empty_and_is_empty() {
    assert!(Flags::NONE.is_empty());
    assert!(!flags::IGNORECASE.is_empty());
}

#[test]
fn insert_remove() {
    let mut f = Flags::NONE;
    f.insert(flags::IGNORECASE);
    assert!(f.contains(flags::IGNORECASE));
    f.insert(flags::MULTILINE | flags::DOTALL);
    assert!(f.contains(flags::MULTILINE) && f.contains(flags::DOTALL));
    f.remove(flags::IGNORECASE);
    assert!(!f.contains(flags::IGNORECASE));
    assert!(f.contains(flags::MULTILINE));
}

#[test]
fn union_intersection_difference() {
    let a = flags::IGNORECASE | flags::MULTILINE | flags::DOTALL;
    let b = flags::MULTILINE | flags::DOTALL | flags::VERBOSE;
    assert_eq!(a.union(b), a | b);
    assert_eq!(a.intersection(b), a & b);
    assert!(a.intersection(b).contains(flags::MULTILINE | flags::DOTALL));
    assert!(!a.intersection(b).contains(flags::IGNORECASE));
    let d = a.difference(b);
    assert!(d.contains(flags::IGNORECASE));
    assert!(!d.contains(flags::MULTILINE));
}

#[test]
fn scoped_vs_global_split() {
    let scoped_only = flags::IGNORECASE;
    assert!(scoped_only.scoped().contains(flags::IGNORECASE));
    assert!(scoped_only.global().is_empty());

    let global_only = flags::VERSION1;
    assert!(global_only.global().contains(flags::VERSION1));
    assert!(global_only.scoped().is_empty());

    let mixed = flags::IGNORECASE | flags::VERSION0 | flags::MULTILINE;
    assert!(
        mixed
            .scoped()
            .contains(flags::IGNORECASE | flags::MULTILINE)
    );
    assert!(mixed.global().contains(flags::VERSION0));
}

#[test]
fn bitwise_operators() {
    let a = flags::IGNORECASE;
    let b = flags::MULTILINE;
    assert_eq!((a | b).bits(), a.bits() | b.bits());
    let mut x = a;
    x |= b;
    assert_eq!(x, a | b);
    let mut y = a | b;
    y &= a;
    assert_eq!(y, a);
    assert_eq!((a ^ b).bits(), a.bits() ^ b.bits());
    // Not flips all bits, but the meaningful ones round-trip via intersection.
    assert_eq!((a).intersection(!Flags::NONE), a);
}

// --- semantic effect of flags --------------------------------------------

#[test]
fn ascii_flag_changes_d_and_w() {
    use eregex::Regex;
    // In ASCII mode \d does not match fullwidth digit.
    let r = Regex::new_with_flags(r"\d", flags::ASCII).unwrap();
    assert!(r.find("5").is_some());
    assert!(r.find("\u{FF15}").is_none());
    // Unicode (default) matches it.
    let r = Regex::new(r"\d").unwrap();
    assert!(r.find("\u{FF15}").is_some());
}

#[test]
fn default_resolves_to_unicode_and_version1() {
    use eregex::Regex;
    let r = Regex::new("a").unwrap();
    let f = r.flags();
    assert!(f.contains(flags::UNICODE), "UNICODE should default on");
    assert!(f.contains(flags::VERSION1), "VERSION1 should default on");
}
