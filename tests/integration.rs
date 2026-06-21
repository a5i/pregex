//! Integration tests exercising the parser + matcher end to end.

use eregex::{Regex, flags};

fn re(p: &str) -> Regex {
    Regex::new(p).expect(&format!("failed to compile {p:?}"))
}

fn re_flags(p: &str, f: flags::Flags) -> Regex {
    Regex::new_with_flags(p, f).expect(&format!("failed to compile {p:?}"))
}

// --- literals & basics ----------------------------------------------------

#[test]
fn literal_match() {
    let r = re("abc");
    assert_eq!(r.find("xxabcyy").unwrap().as_str(), "abc");
    assert!(r.is_match("abc"));
    assert!(!r.is_match("axx"));
}

#[test]
fn dot_any() {
    let r = re("a.c");
    assert_eq!(r.find("axc").unwrap().as_str(), "axc");
    assert!(r.find("a\ncc").is_none()); // '.' does not cross newline
    let r = re_flags("a.c", flags::DOTALL);
    assert_eq!(r.find("a\nc").unwrap().as_str(), "a\nc");
}

#[test]
fn alternation() {
    let r = re("cat|dog|bird");
    assert_eq!(r.find("I have a dog").unwrap().as_str(), "dog");
    assert_eq!(r.find("a bird flew").unwrap().as_str(), "bird");
}

#[test]
fn empty_alternative() {
    let r = re("a|");
    assert_eq!(r.find("xyz").unwrap().span(), (0, 0));
}

// --- anchors --------------------------------------------------------------

#[test]
fn start_end_anchors() {
    let r = re("^abc$");
    assert!(r.find("abc").is_some());
    assert!(r.find("abcd").is_none());
    assert!(r.find("xabc").is_none());
}

#[test]
fn multiline_anchors() {
    let r = re_flags("^foo", flags::MULTILINE);
    let hay = "bar\nfoo\nbaz";
    let m = r.find(hay).unwrap();
    assert_eq!(m.start(), 4);
}

#[test]
fn word_boundary() {
    let r = re(r"\bcat\b");
    assert!(r.find("the cat sat").is_some());
    assert!(r.find("concatenate").is_none());
    let r = re(r"\Bcat");
    assert!(r.find("concatenate").is_some());
    assert!(r.find("the cat").is_none());
}

// --- character classes ----------------------------------------------------

#[test]
fn char_class_ranges() {
    let r = re("[a-zA-Z]+");
    assert_eq!(r.find("abc123DEF").unwrap().as_str(), "abc");
}

#[test]
fn negated_class() {
    let r = re(r"[^0-9]+");
    assert_eq!(r.find("123abc456").unwrap().as_str(), "abc");
}

#[test]
fn predefined_classes() {
    assert_eq!(re(r"\d+").find("abc123").unwrap().as_str(), "123");
    assert_eq!(
        re(r"\w+").find("hello_world!").unwrap().as_str(),
        "hello_world"
    );
    assert_eq!(re(r"\s+").find("a   b").unwrap().as_str(), "   ");
    assert_eq!(re(r"\D+").find("123abc").unwrap().as_str(), "abc");
}

#[test]
fn class_with_predef() {
    let r = re(r"[\d.]+");
    assert_eq!(r.find("v3.14").unwrap().as_str(), "3.14");
}

#[test]
fn posix_class() {
    let r = re(r"[[:alpha:]]+");
    assert_eq!(r.find("123abc").unwrap().as_str(), "abc");
}

#[test]
fn unicode_property() {
    let r = re(r"\p{L}+");
    assert_eq!(r.find("123abc").unwrap().as_str(), "abc");
    let r = re(r"\p{N}+");
    assert_eq!(r.find("abc123").unwrap().as_str(), "123");
}

// --- quantifiers ----------------------------------------------------------

#[test]
fn greedy_lazy() {
    let greedy = re("a.*b");
    let lazy = re("a.*?b");
    let hay = "aXbYb";
    assert_eq!(greedy.find(hay).unwrap().as_str(), "aXbYb");
    assert_eq!(lazy.find(hay).unwrap().as_str(), "aXb");
}

#[test]
fn plus_star_question() {
    assert_eq!(re("ab+c").find("ac").is_none(), true);
    assert_eq!(re("ab+c").find("abc").unwrap().as_str(), "abc");
    assert_eq!(re("ab?c").find("ac").unwrap().as_str(), "ac");
    assert_eq!(re("colou?r").find("color").unwrap().as_str(), "color");
}

#[test]
fn bounded_quantifier() {
    assert_eq!(re("a{3}").find("aaaa").unwrap().as_str(), "aaa");
    assert_eq!(re("a{2,}").find("aaaa").unwrap().as_str(), "aaaa");
    assert_eq!(re("a{2,3}").find("aaaa").unwrap().as_str(), "aaa");
    assert_eq!(re("a{2,3}?").find("aaaa").unwrap().as_str(), "aa");
}

#[test]
fn possessive_no_backtrack() {
    // Possessive quantifier refuses to give back; trailing required char fails.
    assert!(re("a++a").find("aaa").is_none());
    assert!(re("(?>a+)a").find("aaa").is_none());
    // Normal greedy can backtrack and succeed.
    assert!(re("a+a").find("aaa").is_some());
}

// --- groups ---------------------------------------------------------------

#[test]
fn capturing_groups() {
    let r = re(r"(\d+)-(\d+)");
    let m = r.find("phone 12-345").unwrap();
    assert_eq!(m.group(1), Some("12"));
    assert_eq!(m.group(2), Some("345"));
    assert_eq!(m.groups(), vec![Some("12-345"), Some("12"), Some("345")]);
}

#[test]
fn named_groups() {
    let r = re(r"(?P<year>\d{4})-(?P<month>\d{2})");
    let m = r.find("2024-06").unwrap();
    assert_eq!(m.name("year"), Some("2024"));
    assert_eq!(m.name("month"), Some("06"));
}

#[test]
fn angle_named_group() {
    let r = re(r"(?<word>\w+)");
    let m = r.find("hello!").unwrap();
    assert_eq!(m.name("word"), Some("hello"));
}

#[test]
fn non_capturing() {
    let r = re("(?:ab)+");
    let m = r.find("ababab").unwrap();
    assert_eq!(m.as_str(), "ababab");
    assert_eq!(m.len(), 1); // only group 0
}

#[test]
fn atomic_group() {
    // Without atomic, the engine can backtrack from `bc` to `b`.
    let r = re("a(bc|b)c");
    assert_eq!(r.find("abc").unwrap().as_str(), "abc");
    // With atomic, once `bc` matches it is frozen and the trailing `c` fails.
    let r = re("a(?>bc|b)c");
    assert!(r.find("abc").is_none());
    // ... but it matches when there really is a trailing `c`.
    assert_eq!(r.find("abcc").unwrap().as_str(), "abcc");
}

// --- backreferences -------------------------------------------------------

#[test]
fn numeric_backref() {
    let r = re(r"(\w)\1");
    assert_eq!(r.find("aab").unwrap().as_str(), "aa");
    assert!(r.find("ab").is_none());
}

#[test]
fn named_backref() {
    let r = re(r"(?P<x>\w+)-(?P=x)");
    assert_eq!(r.find("go-go").unwrap().as_str(), "go-go");
}

#[test]
fn g_ref_backref() {
    let r = re(r"(\w+) \g<1>");
    assert_eq!(r.find("hi hi").unwrap().as_str(), "hi hi");
}

// --- lookaround -----------------------------------------------------------

#[test]
fn lookahead() {
    let r = re(r"\d+(?= dollars)");
    let m = r.find("100 dollars").unwrap();
    assert_eq!(m.as_str(), "100");
    assert_eq!(m.end(), 3);
    assert!(r.find("100 euros").is_none());
}

#[test]
fn negative_lookahead() {
    let r = re(r"\d+(?! dollars)");
    assert!(r.find("100 dollars").is_none() || r.find("100 dollars").unwrap().as_str() == "10");
    let m = r.find("100 euros").unwrap();
    assert_eq!(m.as_str(), "100");
}

#[test]
fn lookbehind_fixed() {
    let r = re(r"(?<=\$)\d+");
    let m = r.find("price $42 now").unwrap();
    assert_eq!(m.as_str(), "42");
    assert!(r.find("price 42").is_none());
}

#[test]
fn lookbehind_variable() {
    // Variable-length lookbehind: behind is "foo" OR "foobar".
    let r = re(r"(?<=foo|foobar)X");
    assert_eq!(r.find("fooX").unwrap().as_str(), "X");
    assert_eq!(r.find("foobarX").unwrap().as_str(), "X");
    assert!(r.find("barX").is_none());
}

#[test]
fn negative_lookbehind() {
    let r = re(r"(?<!a)b");
    // 'b' not preceded by 'a'.
    let m = r.find("xb").unwrap();
    assert_eq!(m.start(), 1);
    assert!(r.find("ab").is_none());
}

// --- flags ----------------------------------------------------------------

#[test]
fn ignorecase() {
    let r = re_flags("hello", flags::IGNORECASE);
    assert_eq!(r.find("HELLO").unwrap().as_str(), "HELLO");
    let r = re("(?i)hello");
    assert_eq!(r.find("HeLLo").unwrap().as_str(), "HeLLo");
}

#[test]
fn inline_flag_scope() {
    let r = re("(?i:ab)c");
    assert_eq!(r.find("ABc").unwrap().as_str(), "ABc");
    // 'c' is outside the case-insensitive scope.
    assert!(r.find("ABC").is_none());
}

#[test]
fn verbose_mode() {
    let p = r"(?x)
        \d+    # the number
        \s+    # whitespace
        \w+    # the word
    ";
    let r = re(p);
    let m = r.find("123 hello").unwrap();
    assert_eq!(m.as_str(), "123 hello");
}

// --- escapes --------------------------------------------------------------

#[test]
fn hex_escapes() {
    let r = re(r"\x41\x42");
    assert_eq!(r.find("ABC").unwrap().as_str(), "AB");
    let r = re(r"\x{1F600}");
    assert!(r.find("😀").is_some());
}

#[test]
fn q_e_quoting() {
    let r = re(r"\Q.*\E");
    assert_eq!(r.find("a.*b").unwrap().as_str(), ".*");
}

// --- repeated captures ----------------------------------------------------

#[test]
fn repeated_captures() {
    let r = re(r"(\w)+");
    let m = r.find("abc").unwrap();
    assert_eq!(m.group(1), Some("c")); // last
    assert_eq!(m.captures(1), vec![Some("a"), Some("b"), Some("c")]);
}

#[test]
fn repeated_named_captures() {
    let r = re(r"(?:(?P<word>\w+) )+");
    let m = r.find("one two three ").unwrap();
    assert_eq!(
        m.captures_name("word"),
        vec![Some("one"), Some("two"), Some("three")]
    );
}

// --- find_iter / split / replace ------------------------------------------

#[test]
fn find_iter_non_overlapping() {
    let r = re(r"\d+");
    let ms: Vec<_> = r
        .find_iter("a1 bb 22 ccc 333")
        .map(|m| m.as_str().to_string())
        .collect();
    assert_eq!(ms, vec!["1", "22", "333"]);
}

#[test]
fn find_iter_empty_matches() {
    let r = re("a*");
    let ms: Vec<_> = r.find_iter("aba").map(|m| m.as_str().to_string()).collect();
    assert_eq!(ms, vec!["a", "", "a", ""]);
}

#[test]
fn replace_first() {
    let r = re(r"(\w+) (\w+)");
    assert_eq!(r.replace("hello world", "$2 $1"), "world hello");
}

#[test]
fn replace_all_named() {
    let r = re(r"(?P<a>\d)(?P<b>\d)");
    assert_eq!(r.replace_all("12 34", "${b}${a}"), "21 43");
}

#[test]
fn split_basic() {
    let r = re(r"\s+");
    assert_eq!(r.split("a  b c"), vec!["a", "b", "c"]);
}

#[test]
fn split_with_groups() {
    let r = re(r"(-)");
    assert_eq!(r.split("a-b-c"), vec!["a", "-", "b", "-", "c"]);
}

// --- escape helper --------------------------------------------------------

#[test]
fn escape_helper() {
    assert_eq!(eregex::escape("a.b"), r"a\.b");
    assert_eq!(eregex::escape_special_only("a.b!"), r"a\.b!");
}

// --- error handling -------------------------------------------------------

#[test]
fn unbalanced_paren_errors() {
    assert!(Regex::new("(abc").is_err());
}

#[test]
fn bad_escape_errors() {
    assert!(Regex::new(r"\").is_err());
}

#[test]
fn nothing_to_repeat_errors() {
    assert!(Regex::new("*abc").is_err());
}

// --- cases derived from .local/test-cases.md (full-match subset) ---------
//
// The companion file describes a future *partial* matching API. The cases
// below are the subset that the current (full-match-only) engine can already
// express. Partial-only variants live in `tests/partial.rs` behind
// `#[ignore]`.

// #6 — full match with two capture groups inside a longer string.
#[test]
fn tc06_full_match_inside_long_string() {
    let r = re(r"token=([a-z]+)([0-9]+)");
    let m = r.find("xxx token=abc123 yyy").unwrap();
    assert_eq!(m.as_str(), "token=abc123");
    assert_eq!(m.start(), 4);
    assert_eq!(m.end(), 16);
    assert_eq!(m.group(1), Some("abc"));
    assert_eq!(m.group(2), Some("123"));
}

// #7 — no full match, and the engine must not report a spurious partial.
#[test]
fn tc07_no_match_anywhere() {
    let r = re(r"token=([a-z]+)([0-9]+)");
    // "tok=" can never become "token=", and there is no other start.
    assert!(r.find("xxx tok=abc123 yyy").is_none());
    assert!(!r.is_match("xxx tok=abc123 yyy"));
}

// #11A — alternation where the shorter branch is already a complete match.
// Leftmost-first semantics: `stop` wins before `stopped` is tried.
#[test]
fn tc11a_alternation_short_branch_full() {
    let r = re(r"cmd=(stop|stopped)");
    let m = r.find("cmd=stop").unwrap();
    assert_eq!(m.as_str(), "cmd=stop");
    assert_eq!(m.group(1), Some("stop"));
}

// #15 — search must return the first *full* match, not a later partial one.
#[test]
fn tc15_search_picks_first_full_match() {
    let r = re(r"id=([0-9]{3})");
    let m = r.find("id=123 other id=45").unwrap();
    assert_eq!(m.as_str(), "id=123");
    assert_eq!(m.group(1), Some("123"));
}

// #15 (find_iter) — incomplete trailing candidate is skipped entirely.
#[test]
fn tc15_find_iter_skips_incomplete() {
    let r = re(r"id=([0-9]{3})");
    let ms: Vec<_> = r
        .find_iter("id=123 other id=45")
        .map(|m| m.as_str().to_string())
        .collect();
    assert_eq!(ms, vec!["id=123"]);
}

// #18 — full word between word boundaries.
#[test]
fn tc18_word_boundary_full_word() {
    let r = re(r"\bhello\b");
    let m = r.find("say hello").unwrap();
    assert_eq!(m.as_str(), "hello");
    assert_eq!(m.start(), 4);
    assert_eq!(m.end(), 9);
}

// #20 — full URL match with an optional path group participating.
#[test]
fn tc20_url_full_with_optional_path() {
    let r = re(r"https://([a-z0-9.-]+)(/[a-z0-9/_-]+)?");
    let m = r.find("open https://example.com/api/us").unwrap();
    assert_eq!(m.as_str(), "https://example.com/api/us");
    assert_eq!(m.start(), 5);
    assert_eq!(m.group(1), Some("example.com"));
    assert_eq!(m.group(2), Some("/api/us"));
}

// #25 — optional group that did not start: its slot stays `None`.
#[test]
fn tc25_optional_group_not_started() {
    let r = re(r"user:([a-z]+)(?: role=([a-z]+))?");
    let m = r.find("user:alice").unwrap();
    assert_eq!(m.as_str(), "user:alice");
    assert_eq!(m.group(1), Some("alice"));
    assert_eq!(m.group(2), None);
}

// #29 — anchored pattern that reaches the end of input.
#[test]
fn tc29_anchored_end_full_match() {
    let r = re(r"code=([A-Z]{3})$");
    let m = r.find("code=ABC").unwrap();
    assert_eq!(m.as_str(), "code=ABC");
    assert_eq!(m.group(1), Some("ABC"));
}

// #12 (full-match half) — repeated capture of the separator group.
#[test]
fn tc12_repetition_full_list() {
    let r = re(r"ids: ([0-9]{3})(,[0-9]{3})*");
    let m = r.find("ids: 123,456,789").unwrap();
    assert_eq!(m.as_str(), "ids: 123,456,789");
    assert_eq!(m.group(1), Some("123"));
    // group 2 is the last capture; captures(2) keeps the full history.
    assert_eq!(m.captures(2), vec![Some(",456"), Some(",789")]);
}

// #23 (full-match half) — named groups with alternation, fully satisfied.
#[test]
fn tc23_named_groups_alternation_full() {
    let r = re(r"user=(?P<user>[a-z]+) action=(?P<action>login|logout)");
    let m = r.find("event user=alice action=login").unwrap();
    assert_eq!(m.name("user"), Some("alice"));
    assert_eq!(m.name("action"), Some("login"));
}
