//! Unit tests for the `CharSet` codepoint-range structure and its set ops.

use pregex::charset::CharSet;

fn ch(n: u32) -> char {
    char::from_u32(n).unwrap()
}

#[test]
fn from_char_and_contains() {
    let s = CharSet::from_char('a');
    assert!(s.contains('a'));
    assert!(!s.contains('b'));
}

#[test]
fn from_range_contains_endpoints() {
    let s = CharSet::from_range('a', 'z');
    assert!(s.contains('a'));
    assert!(s.contains('m'));
    assert!(s.contains('z'));
    assert!(!s.contains('A'));
    assert!(!s.contains('{')); // just past 'z'
}

#[test]
fn empty_and_full() {
    assert!(CharSet::empty().is_empty());
    assert!(!CharSet::full().is_empty());
    let full = CharSet::full();
    assert!(full.contains('a'));
    assert!(full.contains('\u{10FFFF}'));
}

#[test]
fn from_ranges_unsorted_merges() {
    let s = CharSet::from_ranges_unsorted(vec![(10, 20), (5, 7), (8, 9), (100, 100)]);
    assert!(s.contains(ch(5)));
    assert!(s.contains(ch(10))); // range 5..=20 should be one merged range
    assert!(s.contains(ch(20)));
    assert!(!s.contains(ch(21)));
    assert!(s.contains(ch(100)));
}

#[test]
fn add_char_and_range_grow() {
    let mut s = CharSet::from_char('a');
    s.add_char('b');
    s.add_char('z');
    s.add_range('d', 'f');
    assert!(s.contains('a'));
    assert!(s.contains('b'));
    assert!(s.contains('d'));
    assert!(s.contains('f'));
    assert!(s.contains('z'));
    assert!(!s.contains('c')); // gap
}

#[test]
fn complement() {
    let s = CharSet::from_char('a');
    let c = s.complement();
    assert!(!c.contains('a'));
    assert!(c.contains('b'));
    assert!(c.contains('\u{0}'));
    assert!(c.contains('\u{10FFFF}'));
}

#[test]
fn union() {
    let a = CharSet::from_range('a', 'f');
    let b = CharSet::from_range('d', 'k');
    let u = a.union(&b);
    for c in ['a', 'b', 'f', 'g', 'k'] {
        assert!(u.contains(c), "{c:?} should be in union");
    }
    assert!(!u.contains('l'));
}

#[test]
fn intersect() {
    let a = CharSet::from_range('a', 'f');
    let b = CharSet::from_range('d', 'k');
    let i = a.intersect(&b);
    for c in ['d', 'e', 'f'] {
        assert!(i.contains(c), "{c:?} should be in intersection");
    }
    for c in ['a', 'c', 'g', 'k'] {
        assert!(!i.contains(c), "{c:?} should NOT be in intersection");
    }
}

#[test]
fn difference() {
    let a = CharSet::from_range('a', 'k');
    let b = CharSet::from_range('d', 'f');
    let d = a.difference(&b);
    for c in ['a', 'c', 'g', 'k'] {
        assert!(d.contains(c), "{c:?} should be in difference");
    }
    for c in ['d', 'e', 'f'] {
        assert!(!d.contains(c));
    }
}

#[test]
fn symmetric_difference() {
    let a = CharSet::from_range('a', 'f');
    let b = CharSet::from_range('d', 'k');
    let sd = a.sym_diff(&b);
    // Union minus intersection.
    for c in ['a', 'c', 'g', 'k'] {
        assert!(sd.contains(c), "{c:?} should be in sym diff");
    }
    for c in ['d', 'e', 'f'] {
        assert!(!sd.contains(c));
    }
}

#[test]
fn add_case_variants_ascii() {
    let mut s = CharSet::from_char('a');
    s.add_case_variants();
    assert!(s.contains('a'));
    assert!(s.contains('A'));
    let mut s = CharSet::from_range('a', 'c');
    s.add_case_variants();
    assert!(s.contains('A') && s.contains('B') && s.contains('C'));
    assert!(s.contains('a') && s.contains('c'));
}

#[test]
fn complement_of_complement_is_identity() {
    let s = CharSet::from_ranges_unsorted(vec![(10, 20), (50, 60)]);
    let roundtrip = s.complement().complement();
    assert_eq!(s, roundtrip);
}

#[test]
fn de_morgan_union() {
    let a = CharSet::from_range('a', 'f');
    let b = CharSet::from_range('d', 'k');
    // !(a ∪ b) == !a ∩ !b
    assert_eq!(a.union(&b).complement(), a.complement().intersect(&b.complement()));
}
