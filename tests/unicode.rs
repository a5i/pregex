//! Unit tests for `pregex::unicode` helpers and the `\p{...}` property table.

use pregex::unicode;

#[test]
fn is_digit_ascii_vs_unicode() {
    assert!(unicode::is_digit('5', true));
    assert!(unicode::is_digit('5', false));
    assert!(!unicode::is_digit('a', true));
    // Fullwidth digit U+FF15 is a Unicode Nd but not ASCII.
    assert!(unicode::is_digit('\u{FF15}', false));
    assert!(!unicode::is_digit('\u{FF15}', true));
}

#[test]
fn is_word_ascii_vs_unicode() {
    assert!(unicode::is_word('a', true));
    assert!(unicode::is_word('_', true));
    assert!(!unicode::is_word('é', true)); // not ASCII
    assert!(unicode::is_word('é', false)); // Unicode
}

#[test]
fn is_space_ascii_vs_unicode() {
    assert!(unicode::is_space(' ', true));
    assert!(unicode::is_space('\t', true));
    assert!(unicode::is_space('\u{00A0}', false)); // NBSP, Unicode-only
    assert!(!unicode::is_space('\u{00A0}', true));
}

#[test]
fn case_eq() {
    assert!(unicode::case_eq('a', 'A'));
    assert!(unicode::case_eq('A', 'a'));
    assert!(unicode::case_eq('z', 'z'));
    assert!(!unicode::case_eq('a', 'b'));
    // Greek sigma / accented.
    assert!(unicode::case_eq('à', 'À'));
}

#[test]
fn push_case_variants() {
    let mut v = Vec::new();
    unicode::push_case_variants('a', &mut v);
    assert!(v.contains(&'a'));
    assert!(v.contains(&'A'));
}

// --- property table -------------------------------------------------------

fn matches(p: fn(char) -> bool, c: char) -> bool {
    p(c)
}

#[test]
fn property_letter() {
    let p = unicode::property("L").unwrap();
    assert!(matches(p, 'a'));
    assert!(matches(p, 'Z'));
    assert!(!matches(p, '1'));
}

#[test]
fn property_general_category_aliases() {
    // gc=nd and nd and decimalnumber are all the same.
    for name in ["Nd", "gc=nd", "general-category=nd", "decimal number"] {
        let p = unicode::property(name).expect(name);
        assert!(matches(p, '5'), "{name} should match 5");
        assert!(!matches(p, 'a'), "{name} should not match a");
    }
}

#[test]
fn property_case() {
    let lu = unicode::property("Lu").unwrap();
    let ll = unicode::property("Ll").unwrap();
    assert!(matches(lu, 'A') && !matches(lu, 'a'));
    assert!(matches(ll, 'a') && !matches(ll, 'A'));
}

#[test]
fn property_number_letter_other() {
    let n = unicode::property("N").unwrap();
    assert!(matches(n, '5'));
    assert!(!matches(n, 'a'));
}

#[test]
fn property_binary_aliases() {
    let alpha = unicode::property("Alpha").unwrap();
    assert!(matches(alpha, 'a'));
    assert!(!matches(alpha, '1'));

    let alnum = unicode::property("Alnum").unwrap();
    assert!(matches(alnum, 'a') && matches(alnum, '1'));

    let upper = unicode::property("upper").unwrap();
    assert!(matches(upper, 'A') && !matches(upper, 'a'));

    let space = unicode::property("WHITESPACE").unwrap();
    assert!(matches(space, ' '));

    let digit = unicode::property("DIGIT").unwrap();
    assert!(matches(digit, '7'));

    let ascii = unicode::property("ascii").unwrap();
    assert!(matches(ascii, 'a') && !matches(ascii, 'é'));

    let xdigit = unicode::property("xdigit").unwrap();
    assert!(matches(xdigit, 'F') && matches(xdigit, '9') && !matches(xdigit, 'g'));

    let word = unicode::property("word").unwrap();
    assert!(matches(word, 'a') && matches(word, '_') && !matches(word, '!'));
}

#[test]
fn property_normalization_strips_separators() {
    // Spaces, underscores and dashes are ignored.
    assert_eq!(
        unicode::property("General Category").is_some(),
        unicode::property("general-category").is_some(),
    );
    assert_eq!(
        unicode::property("DECIMAL_NUMBER").is_some(),
        unicode::property("decimalnumber").is_some(),
    );
}

#[test]
fn property_posix_aliases() {
    for (name, yes, no) in [
        ("posix_alnum", 'a', ' '),
        ("posix_digit", '5', 'a'),
        ("posix_xdigit", 'F', 'g'),
    ] {
        let p = unicode::property(name).unwrap();
        assert!(matches(p, yes), "{name} should match {yes:?}");
        assert!(!matches(p, no), "{name} should not match {no:?}");
    }
}

#[test]
fn property_unknown_returns_none() {
    assert!(unicode::property("not_a_real_property").is_none());
    assert!(unicode::property("").is_none());
}
