//! Partial-matching tests for [`Regex::find_partial`].
//!
//! Transcribed from `.local/test-cases.md`. Partial matching here is
//! *end-anchored*: a match must consume the haystack to its end. `None` means
//! the input cannot be a prefix of any match (a hard mismatch occurred before
//! end-of-input); `Some(Full)` means the pattern was fully satisfied; and
//! `Some(Partial)` means the pattern consumed everything but still wanted more.
//!
//! Guiding invariant (from `.local/test-cases.md`):
//! *partial = "the current input does not contradict the pattern and could
//! become a full match if more input arrives" — it is NOT "almost matched".*
//! A wrong character (cases #8, #22, #16) yields `NoMatch`, never `Partial`.
//!
//! Note on "next group not started": in cases #19, #21, #24 the group that the
//! markdown draft labels `partial("")` is in fact *never entered*, because the
//! preceding literal (`.`, `-`, `,`) fails at end-of-input before the group is
//! reached. The engine therefore reports it as [`GroupMatch::None`] rather than
//! `Partial("")` — this is the truthful result and is asserted below.

use pregex::Regex;

fn re(p: &str) -> Regex {
    Regex::new(p).expect(&format!("failed to compile {p:?}"))
}

// #1 — literal prefix matched; required tail (`-[0-9]{4}`) missing.
//   (Second variant from the markdown: input ends right after `ABCD`.)
#[test]
fn tc01_partial_literal_prefix() {
    let r = re(r"ABCD-[0-9]{4}");
    let m = r.find_partial("noise noise START: ABCD").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "ABCD");
    assert_eq!(m.start, 19);
}

// #2 — first capture group already complete, second not yet started.
#[test]
fn tc02_group_complete_next_missing() {
    let r = re(r"user:([a-z]+)@([a-z]+)\.com");
    let m = r.find_partial("prefix user:john@").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "user:john@");
    assert_eq!(m.group(1), Some("john"));
    assert!(m.group_matched(1));
    // Group 2 was entered (after `@` matched) but consumed nothing.
    assert!(m.group_partial(2));
    assert_eq!(m.group(2), Some(""));
}

// #3 — partial *inside* a capture group.
#[test]
fn tc03_partial_inside_group() {
    let r = re(r"order:(ORD-[0-9]{4})");
    let m = r.find_partial("prefix order:ORD-12").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "order:ORD-12");
    assert!(m.group_partial(1));
    assert_eq!(m.group(1), Some("ORD-12"));
}

// #4 — several groups: some complete, some partial.
#[test]
fn tc04_mixed_complete_and_partial_groups() {
    let r = re(r"id=([0-9]+) user=([a-z]{5})");
    let m = r.find_partial("log id=42 user=ali").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "id=42 user=ali");
    assert!(m.group_matched(1));
    assert_eq!(m.group(1), Some("42"));
    assert!(m.group_partial(2));
    assert_eq!(m.group(2), Some("ali"));
}

// #5 — full literal prefix, then a digit group that has not started yet.
#[test]
fn tc05_partial_empty_second_group() {
    let r = re(r"card=([0-9]{4})-([0-9]{4})");
    let m = r.find_partial("payment card=4242-").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "card=4242-");
    assert!(m.group_matched(1));
    assert_eq!(m.group(1), Some("4242"));
    assert!(m.group_partial(2));
    assert_eq!(m.group(2), Some(""));
}

// #8 — a wrong character after a matching prefix: NoMatch, NOT Partial.
#[test]
fn tc08_wrong_char_means_no_match() {
    let r = re(r"token=([a-z]+)([0-9]+)");
    // '!' proves the continuation can never succeed.
    assert!(r.find_partial("xxx token=abc!").is_none());
}

// #9 — partial because input ended before a required part (no bad char).
#[test]
fn tc09_partial_input_ended_before_required() {
    let r = re(r"token=([a-z]+)([0-9]+)");
    let m = r.find_partial("xxx token=abc").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "token=abc");
    assert!(m.group_matched(1));
    assert_eq!(m.group(1), Some("abc"));
    assert!(m.group_partial(2));
    assert_eq!(m.group(2), Some(""));
}

// #10 — alternation: every branch is still partial.
#[test]
fn tc10_alternation_all_branches_partial() {
    let r = re(r"cmd=(start|stop|status)");
    let m = r.find_partial("cmd=sta").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "cmd=sta");
    assert!(m.group_partial(1));
    assert_eq!(m.group(1), Some("sta"));
}

// #12 (partial half) — last repeated element is partial.
#[test]
fn tc12_repetition_last_element_partial() {
    let r = re(r"ids: ([0-9]{3})(,[0-9]{3})*");
    let m = r.find_partial("ids: 123,456,7").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "ids: 123,456,7");
    assert!(m.group_matched(1));
    assert_eq!(m.group(1), Some("123"));
    // The repeated group's last iteration is the partial one.
    assert!(m.group_partial(2));
    assert_eq!(m.group(2), Some(",7"));
}

// #13 — separator present, next element not yet started.
#[test]
fn tc13_repetition_separator_only() {
    let r = re(r"ids: ([0-9]{3})(,[0-9]{3})*");
    let m = r.find_partial("ids: 123,").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "ids: 123,");
    assert!(m.group_matched(1));
    assert_eq!(m.group(1), Some("123"));
    assert!(m.group_partial(2));
    assert_eq!(m.group(2), Some(","));
}

// #14 — search must locate a partial further along the haystack.
#[test]
fn tc14_search_finds_partial_later() {
    let r = re(r"order:([A-Z]{4})");
    let m = r.find_partial("junk foo junk order:AB").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "order:AB");
    assert_eq!(m.start, 14);
    assert_eq!(m.end, 22);
    assert!(m.group_partial(1));
    assert_eq!(m.group(1), Some("AB"));
}

// #16 — first candidate is invalid (broken by a wrong char), second is
// partial. The invalid one must NOT be reported as partial.
#[test]
fn tc16_invalid_then_partial() {
    let r = re(r"id=([0-9]{3})");
    let m = r.find_partial("id=12x other id=45").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "id=45");
    assert!(m.group_partial(1));
    assert_eq!(m.group(1), Some("45"));
}

// #17 — word boundary: the word started but the right boundary is unknown.
#[test]
fn tc17_boundary_word_started() {
    let r = re(r"\bhello\b");
    let m = r.find_partial("say hel").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "hel");
}

// #19 — email with several groups; TLD group not yet started.
//   (The markdown speculated group 3 = partial(""), but group 3 is never
//   entered — the `.` before it fails at end-of-input. Reported as None.)
#[test]
fn tc19_email_tld_partial() {
    let r = re(r"([a-z]+(?:\.[a-z]+)*)@([a-z]+)\.([a-z]{2,})");
    let m = r.find_partial("contact: john.doe@example").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "john.doe@example");
    assert!(m.group_matched(1));
    assert_eq!(m.group(1), Some("john.doe"));
    assert!(m.group_matched(2));
    assert_eq!(m.group(2), Some("example"));
    assert!(m.group_none(3));
}

// #21 — date: day present, day-of-month missing.
//   (The markdown speculated group 3 = partial(""), but the second `-` before
//   it fails at end-of-input, so group 3 is never entered. Reported as None.)
#[test]
fn tc21_date_partial_day_missing() {
    let r = re(r"created_at=([0-9]{4})-([0-9]{2})-([0-9]{2})");
    let m = r.find_partial("created_at=2026-06").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "created_at=2026-06");
    assert!(m.group_matched(1));
    assert_eq!(m.group(1), Some("2026"));
    assert!(m.group_matched(2));
    assert_eq!(m.group(2), Some("06"));
    assert!(m.group_none(3));
}

// #22 — wrong separator: NoMatch, NOT Partial.
#[test]
fn tc22_wrong_separator_no_match() {
    let r = re(r"created_at=([0-9]{4})-([0-9]{2})-([0-9]{2})");
    assert!(r.find_partial("created_at=2026/06").is_none());
}

// #23 (partial half) — named groups, second group partial.
#[test]
fn tc23_named_groups_second_partial() {
    let r = re(r"user=(?P<user>[a-z]+) action=(?P<action>login|logout)");
    let m = r.find_partial("event user=alice action=lo").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.name("user"), Some("alice"));
    assert_eq!(m.group(1), Some("alice"));
    // `action` was entered and partially matched "lo".
    assert_eq!(m.name("action"), Some("lo"));
    assert_eq!(m.group(2), Some("lo"));
    assert!(m.group_partial(2));
}

// #24 — nested groups; the group after the second comma never starts.
//   (The markdown speculated group 3 = partial(""), but the second `,` before
//   it fails at end-of-input, so group 3 is never entered. Reported as None.)
#[test]
fn tc24_nested_groups_last_partial() {
    let r = re(r"rgb\(([0-9]{1,3}),([0-9]{1,3}),([0-9]{1,3})\)");
    let m = r.find_partial("color: rgb(255,12").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "rgb(255,12");
    assert_eq!(m.group(1), Some("255"));
    assert!(m.group_matched(1));
    assert_eq!(m.group(2), Some("12"));
    assert!(m.group_matched(2));
    assert!(m.group_none(3));
}

// #26 — optional group started but partial.
#[test]
fn tc26_optional_group_started_partial() {
    let r = re(r"user:([a-z]+)(?: role=(admin|editor|viewer))?");
    let m = r.find_partial("user:alice role=ad").expect("partial");
    assert!(m.is_partial());
    assert!(m.group_matched(1));
    assert_eq!(m.group(1), Some("alice"));
    assert!(m.group_partial(2));
    assert_eq!(m.group(2), Some("ad"));
}

// #27 — greedy group before a required suffix; suffix missing.
#[test]
fn tc27_greedy_before_required_suffix() {
    let r = re(r"title: (.+) END");
    let m = r.find_partial("title: hello wor").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "title: hello wor");
    // Greedy group captured everything available.
    assert_eq!(m.group(1), Some("hello wor"));
}

// #28 — greedy group with a suffix-like continuation that is NOT the expected
// suffix. In streaming mode this is still partial: more input could bring the
// real ` END`. Our single-mode engine therefore reports Partial.
#[test]
fn tc28_greedy_suffix_like_continuation() {
    let r = re(r"title: (.+) END");
    let m = r.find_partial("title: hello wor STOP").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "title: hello wor STOP");
}

// #30 — anchored pattern: required chars missing before end-of-input.
#[test]
fn tc30_anchored_required_chars_missing() {
    let r = re(r"code=([A-Z]{3})$");
    let m = r.find_partial("code=AB").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "code=AB");
    assert!(m.group_partial(1));
    assert_eq!(m.group(1), Some("AB"));
}

// Extra: the "full match reaching end-of-input" counterpart of #30.
#[test]
fn tc30_full_counterpart() {
    let r = re(r"code=([A-Z]{3})$");
    let m = r.find_partial("code=ABC").expect("full");
    assert!(m.is_full());
    assert_eq!(m.matched, "code=ABC");
    assert!(m.group_matched(1));
    assert_eq!(m.group(1), Some("ABC"));
}

// Extra: contrast with the unanchored `find`. On "id=123 other id=45",
// `find` returns the leftmost full match "id=123", whereas `find_partial`
// (end-anchored) reports the trailing "id=45" as Partial — confirming the
// two methods have genuinely different semantics.
#[test]
fn find_partial_contrasts_with_find() {
    let r = re(r"id=([0-9]{3})");
    // Unanchored: leftmost full match.
    assert_eq!(r.find("id=123 other id=45").unwrap().as_str(), "id=123");
    // End-anchored: the trailing partial.
    let m = r.find_partial("id=123 other id=45").expect("partial");
    assert!(m.is_partial());
    assert_eq!(m.matched, "id=45");
    assert_eq!(m.start, 13);
    assert!(m.group_partial(1));
    assert_eq!(m.group(1), Some("45"));
}
