//! Tests for `pregex::escape` and its variants.

use pregex::escape;

#[test]
fn escape_metachars() {
    assert_eq!(escape("."), r"\.");
    assert_eq!(escape("a*b+c?"), r"a\*b\+c\?");
    assert_eq!(escape("(a)"), r"\(a\)");
    assert_eq!(escape("[a]"), r"\[a\]");
    assert_eq!(escape("a|b"), r"a\|b");
    assert_eq!(escape(r"a\b"), r"a\\b");
}

#[test]
fn escape_special_only() {
    use pregex::escape_special_only;
    // Only regex-special chars are escaped.
    assert_eq!(escape_special_only("a.b!"), r"a\.b!");
    // Space IS escaped under special_only (matches mrab's literal_spaces=False);
    // use escape_literal_spaces to keep spaces.
    assert_eq!(escape_special_only("a b"), r"a\ b");
    assert_eq!(escape_special_only("x*y!"), r"x\*y!");
}

#[test]
fn escape_literal_spaces() {
    use pregex::escape_literal_spaces;
    // Spaces are preserved literally; metachars still escaped.
    assert_eq!(escape_literal_spaces("a b.c"), "a b\\.c");
    // Default escape would escape the space.
    assert_ne!(escape("a b"), escape_literal_spaces("a b"));
}

#[test]
fn escape_empty() {
    assert_eq!(escape(""), "");
}

#[test]
fn escape_unicode_safe() {
    // Non-ASCII passes through unchanged.
    assert_eq!(escape("café"), "caf\\é".to_string().replace("\\é", "é"));
    let _ = escape("😀"); // must not panic
}
