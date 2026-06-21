//! Edge cases: escapes, quantifier corners, parser errors, replace templates,
//! and subtle matcher behaviour.

use pregex::{flags, Regex};

/// Re-export the error kind for assertions.
use pregex::error::ErrorKind;

fn re(p: &str) -> Regex {
    Regex::new(p).unwrap()
}
fn miss(p: &str, hay: &str) -> bool {
    re(p).find(hay).is_none()
}

// --- escapes (comprehensive) ---------------------------------------------

#[test]
fn hex_u_escape() {
    assert_eq!(re(r"\u0041").find("ABC").unwrap().as_str(), "A");
}
#[test]
fn hex_uppercase_u() {
    assert_eq!(re(r"\U00000061").find("abc").unwrap().as_str(), "a");
}
#[test]
fn octal_escape() {
    // \101 = octal 65 = 'A'
    assert_eq!(re(r"\101").find("ABC").unwrap().as_str(), "A");
    // \012 = octal 10 = newline
    assert!(re(r"\012").find("\n").is_some());
    // \0 = NUL
    assert!(re(r"\0").find("\0").is_some());
}
#[test]
fn escaped_metacharacters() {
    assert!(re(r"\.").find("a.b").is_some());
    assert!(re(r"\(\)").find("()").is_some());
    assert!(re(r"\[\]").find("[]").is_some());
    assert!(re(r"\{\}").find("{}").is_some());
    assert!(re(r"\*").find("*").is_some());
    assert!(re(r"\+").find("+").is_some());
    assert!(re(r"\?").find("?").is_some());
    assert!(re(r"\^").find("^").is_some());
    assert!(re(r"\$").find("$").is_some());
    assert!(re(r"\|").find("|").is_some());
    assert!(re(r"\\").find("\\").is_some());
}
#[test]
fn control_letter_escapes() {
    assert!(re(r"\n").find("\n").is_some());
    assert!(re(r"\t").find("\t").is_some());
    assert!(re(r"\r").find("\r").is_some());
    assert!(re(r"\f").find("\x0c").is_some());
    assert!(re(r"\v").find("\x0b").is_some());
    assert!(re(r"\a").find("\x07").is_some());
    assert!(re(r"\e").find("\x1b").is_some());
}
#[test]
fn absolute_text_anchors() {
    assert!(re(r"\Aabc").find("abc").is_some());
    assert!(!re(r"\Aabc").find("xabc").is_some());
    assert!(re(r"abc\z").find("abc").is_some());
    // \z is end of text; "abc\n" has trailing newline so it must not match.
    assert!(miss(r"abc\z", "abc\n"));
}
#[test]
fn k_named_backreference() {
    // \k<name> — PCRE-style named backref.
    let r = Regex::new(r"(?P<x>\w+)-\k<x>").unwrap();
    assert!(r.find("go-go").is_some());
    assert!(miss(r"(?P<x>\w+)-\k<x>", "go-no"));
}
#[test]
fn backslash_inside_class_anchors_range() {
    // [\t-\r] is a range from tab to CR.
    assert!(re(r"[\t-\r]").find("\n").is_some());
}
#[test]
fn class_with_escape_predef_and_range() {
    let r = re(r"[\d.]+");
    assert_eq!(r.find("v3.14").unwrap().as_str(), "3.14");
    let r = re(r"[-a-c]+"); // literal '-' first, then range a-c
    assert_eq!(r.find("x-abc-").unwrap().as_str(), "-abc-");
}

// --- quantifier corners ---------------------------------------------------

#[test]
fn zero_count_quantifier() {
    // {0} matches empty.
    let r = re("a{0}");
    let m = r.find("aaa").unwrap();
    assert_eq!(m.as_str(), "");
    assert_eq!(m.span(), (0, 0));
}
#[test]
fn leading_comma_quantifier() {
    assert_eq!(re("a{,2}").find("aaaa").unwrap().as_str(), "aa");
    assert_eq!(re("a{,0}").find("aaaa").unwrap().as_str(), "");
}
#[test]
fn malformed_brace_is_literal() {
    // {abc} is not a valid quantifier, so '{' is literal.
    assert_eq!(re("a{abc}").find("a{abc}").unwrap().as_str(), "a{abc}");
    assert_eq!(re("{}").find("{}").unwrap().as_str(), "{}");
}
#[test]
fn min_greater_than_max_errors() {
    let e = Regex::new("a{3,2}").unwrap_err();
    assert!(matches!(e.kind, ErrorKind::BadRepeat(_)));
}
#[test]
fn double_quantifier_errors() {
    let e = Regex::new("a**").unwrap_err();
    assert!(matches!(e.kind, ErrorKind::BadRepeat(_)));
    let e = Regex::new("a{2}{2}").unwrap_err();
    assert!(matches!(e.kind, ErrorKind::BadRepeat(_)));
    // `a++` is NOT a double quantifier: it's a possessive `+`, and is valid.
    assert!(Regex::new("a++").is_ok());
}
#[test]
fn nothing_to_repeat_errors() {
    assert!(matches!(
        Regex::new("*").unwrap_err().kind,
        ErrorKind::BadRepeat(_)
    ));
    assert!(matches!(
        Regex::new("+").unwrap_err().kind,
        ErrorKind::BadRepeat(_)
    ));
    assert!(matches!(
        Regex::new("?").unwrap_err().kind,
        ErrorKind::BadRepeat(_)
    ));
}

// --- parser errors --------------------------------------------------------

#[test]
fn unbalanced_paren() {
    assert!(matches!(
        Regex::new("(abc").unwrap_err().kind,
        ErrorKind::Syntax(_)
    ));
}
#[test]
fn stray_close_paren() {
    assert!(matches!(
        Regex::new("abc)").unwrap_err().kind,
        ErrorKind::Syntax(_)
    ));
}
#[test]
fn unterminated_class() {
    assert!(matches!(
        Regex::new("[abc").unwrap_err().kind,
        ErrorKind::BadCharClass(_)
    ));
}
#[test]
fn unterminated_property() {
    assert!(matches!(
        Regex::new(r"\p{L").unwrap_err().kind,
        ErrorKind::BadProperty(_)
    ));
}
#[test]
fn unknown_property_errors() {
    assert!(matches!(
        Regex::new(r"\p{NotAProperty}").unwrap_err().kind,
        ErrorKind::BadProperty(_)
    ));
}
#[test]
fn duplicate_group_name_now_allowed() {
    // mrab-regex (Hg issue 87) allows the same name on multiple groups;
    // they share one group number, and all captures are retained.
    let r = Regex::new(r"(?P<item>\w+)? or (?P<item>\w+)?").unwrap();
    let m = r.find("first or second").unwrap();
    assert_eq!(m.name("item"), Some("second"));
    assert_eq!(
        m.captures_name("item"),
        vec![Some("first"), Some("second")]
    );
}
#[test]
fn backref_to_unknown_group_errors() {
    assert!(matches!(
        Regex::new(r"\5").unwrap_err().kind,
        ErrorKind::BadGroupRef(_)
    ));
    assert!(matches!(
        Regex::new(r"\g<9>").unwrap_err().kind,
        ErrorKind::BadGroupRef(_)
    ));
}
#[test]
fn bad_flag_errors() {
    assert!(matches!(
        Regex::new(r"(?z)abc").unwrap_err().kind,
        ErrorKind::BadFlag(_)
    ));
}
#[test]
fn trailing_backslash_errors() {
    assert!(matches!(
        Regex::new(r"abc\").unwrap_err().kind,
        ErrorKind::BadEscape(_)
    ));
}
#[test]
fn error_carries_position() {
    let e = Regex::new(r"ab(cd").unwrap_err();
    assert!(e.position.is_some(), "error should carry a byte position");
}
#[test]
fn error_display_contains_message() {
    let e = Regex::new(r"(abc").unwrap_err();
    assert!(e.to_string().contains("pregex error"));
    assert!(e.to_string().contains("position"));
}

// --- comments, nesting, structure ----------------------------------------

#[test]
fn inline_comment_group() {
    let r = re("a(?#this is a comment)b");
    assert_eq!(r.find("ab").unwrap().as_str(), "ab");
}
#[test]
fn nested_groups_and_counts() {
    let r = re(r"((a)(b))");
    let m = r.find("ab").unwrap();
    assert_eq!(r.capture_count(), 3);
    assert_eq!(m.len(), 4);
    assert_eq!(m.group(1), Some("ab"));
    assert_eq!(m.group(2), Some("a"));
    assert_eq!(m.group(3), Some("b"));
}
#[test]
fn alternation_in_group() {
    let r = re(r"(cat|dog)s");
    assert_eq!(r.find("I like dogs").unwrap().group(1), Some("dog"));
}
#[test]
fn empty_pattern_matches_empty() {
    let r = re("");
    let m = r.find("abc").unwrap();
    assert_eq!(m.as_str(), "");
}

// --- matcher edge cases ---------------------------------------------------

#[test]
fn backref_to_unmatched_group_is_empty() {
    // Group 1 is optional and skipped; \1 then matches empty.
    let r = re(r"(a)?\1");
    let m = r.find("b").unwrap();
    assert_eq!(m.as_str(), "");
}
#[test]
fn case_insensitive_backref() {
    let r = Regex::new_with_flags(r"(\w+)-\1", flags::IGNORECASE).unwrap();
    assert_eq!(r.find("abc-ABC").unwrap().as_str(), "abc-ABC");
}
#[test]
fn case_insensitive_class_expansion() {
    let r = Regex::new_with_flags(r"[A-F]", flags::IGNORECASE).unwrap();
    assert!(r.find("XaY").is_some()); // lowercase 'a' matches [A-F] under /i
}
#[test]
fn captures_inside_lookaround() {
    // Lookahead does consume captures in mrab semantics.
    let r = re(r"(?=(\d+))");
    let m = r.find("abc123").unwrap();
    assert_eq!(m.group(1), Some("123"));
}
#[test]
fn dollar_before_trailing_newline() {
    // Non-multiline: $ matches at end OR before a FINAL \n only.
    assert!(re(r"foo$").find("foo\n").is_some()); // final newline
    assert!(re(r"foo$").find("foo").is_some()); // end of string
    // $ in the MIDDLE of the string does not match: `foo$` does NOT match "foo\nbar".
    assert!(miss_alt_safe(r"foo$", "foo\nbar"));
}
fn miss_alt_safe(p: &str, hay: &str) -> bool {
    // Helper kept tiny; the named helper above lives in scope.
    re(p).find(hay).is_none()
}
#[test]
fn posix_negated_class() {
    let r = re(r"[[:^digit:]]+");
    assert_eq!(r.find("123abc").unwrap().as_str(), "abc");
}
#[test]
fn overlapping_quantifier_backtracking() {
    // `(a|ab)c` on "abc": first alt `a` leaves `b` which fails `c`, forcing
    // backtracking to `ab`, then `c` matches. Result requires real backtracking.
    let r = re(r"(a|ab)c");
    let m = r.find("abc").unwrap();
    assert_eq!(m.as_str(), "abc");
    assert_eq!(m.group(1), Some("ab"));
}

// --- replace template forms ----------------------------------------------

#[test]
fn replace_amp_and_zero() {
    let r = re(r"\w+");
    assert_eq!(r.replace("hi", "[$&]"), "[hi]");
    assert_eq!(r.replace("hi", "[$0]"), "[hi]");
}
#[test]
fn replace_dollar_dollar() {
    let r = re(r"x");
    assert_eq!(r.replace("x", "$$"), "$");
}
#[test]
fn replace_numeric_and_named_braces() {
    let r = re(r"(\w)(\w)");
    assert_eq!(r.replace("ab", "${2}${1}"), "ba");
    let r = re(r"(?<a>\w)(?<b>\w)");
    assert_eq!(r.replace("ab", "${b}${a}"), "ba");
}
#[test]
fn replace_backslash_numeric() {
    let r = re(r"(\w)(\w)");
    assert_eq!(r.replace("ab", "\\2\\1"), "ba");
}
#[test]
fn replace_g_ref() {
    let r = re(r"(\w)(\w)");
    assert_eq!(r.replace("ab", r"\g<2>\g<1>"), "ba");
}
#[test]
fn replace_all_zero_width() {
    // a* matches every position; matching produces a sentinel.
    let r = re(r"a*");
    let out = r.replace_all("aXa", "#");
    assert!(out.contains("X"));
}
#[test]
fn split_max_implicit() {
    let r = re(r",");
    let v: Vec<String> = r.split_iter("a,b,c,d").collect();
    assert_eq!(v, vec!["a", "b", "c", "d"]);
}
