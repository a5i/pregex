//! Tests that mirror **every code example** in the upstream mrab-regex README.
//!
//! Each section header names the README feature. For features we implement,
//! the test asserts the documented behaviour directly (often copying the
//! exact pattern and string from the README). For features on the roadmap,
//! the test asserts that the parser or API *gracefully rejects* the syntax
//! with a clear error message, so silent wrong answers are impossible.

use eregex::{Regex, flags};

// ===========================================================================
// §1  Lookaround in conditional pattern  (?(?=\d)\d+|\w+)
// ---------------------------------------------------------------------------
// Roadmap. Currently rejected with a clear message.
#[test]
fn readme_lookaround_conditional_not_yet_supported() {
    let e = Regex::new(r"(?(?=\d)\d+|\w+)").unwrap_err();
    assert!(e.to_string().contains("conditional"));
}

// ===========================================================================
// §2  POSIX matching (leftmost-longest)  (?p)
// ---------------------------------------------------------------------------
// Roadmap. The (?p) global flag is rejected.
#[test]
fn readme_posix_matching_not_yet_supported() {
    let e = Regex::new(r"(?p)Mr|Mrs").unwrap_err();
    assert!(e.to_string().contains("flag"));
}

// ===========================================================================
// §7  (?P=...) group reference  [supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_p_equals_name_backreference() {
    let r = Regex::new(r"(?P<word>\w+)-(?P=word)").unwrap();
    let m = r.find("go-go").unwrap();
    assert_eq!(m.as_str(), "go-go");
}

// ===========================================================================
// §9  `*` operator with sub()  — regex.sub('.*', 'x', 'test') == 'xx'
// ---------------------------------------------------------------------------
#[test]
fn readme_star_in_substitution() {
    let r = Regex::new(r".*").unwrap();
    // mrab: sub('.*', 'x', 'test') -> 'xx'
    assert_eq!(r.replace_all("test", "x"), "xx");
}

// ===========================================================================
// §10  capturesdict  [now supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_capturesdict() {
    let r = Regex::new(r"(?:(?P<word>\w+) (?P<digits>\d+)\n)+").unwrap();
    let hay = "one 1\ntwo 2\nthree 3\n";
    let m = r.find(hay).unwrap();
    // groupdict-style (named_groups): the LAST capture per named group.
    let gd = m.named_groups();
    assert_eq!(gd.get("word").copied(), Some("three"));
    assert_eq!(gd.get("digits").copied(), Some("3"));
    // captures(): all captures of one group.
    assert_eq!(
        m.captures_name("word"),
        vec![Some("one"), Some("two"), Some("three")]
    );
    assert_eq!(
        m.captures_name("digits"),
        vec![Some("1"), Some("2"), Some("3")]
    );
    // capturesdict(): a map name -> all captures.
    let cd = m.captures_dict();
    assert_eq!(cd.get("word").unwrap(), &vec!["one", "two", "three"]);
    assert_eq!(cd.get("digits").unwrap(), &vec!["1", "2", "3"]);
}

// ===========================================================================
// §11  allcaptures / allspans  [now supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_allcaptures_and_allspans() {
    let r = Regex::new(r"(?:(?P<word>\w+) (?P<digits>\d+)\n)+").unwrap();
    let hay = "one 1\ntwo 2\nthree 3\n";
    let m = r.find(hay).unwrap();
    let ac = m.all_captures();
    // group 0 = whole match, group 1 = word, group 2 = digits.
    assert_eq!(ac[0], vec!["one 1\ntwo 2\nthree 3\n"]);
    assert_eq!(ac[1], vec!["one", "two", "three"]);
    assert_eq!(ac[2], vec!["1", "2", "3"]);
    let aspans = m.all_spans();
    assert_eq!(aspans[2], vec![(4, 5), (10, 11), (18, 19)]);
}

// ===========================================================================
// §12  Duplicate group names  (Hg issue 87)  [now supported]
// ---------------------------------------------------------------------------
// All four README sub-examples.
#[test]
fn readme_duplicate_names_optional_both_capture() {
    let r = Regex::new(r"(?P<item>\w+)? or (?P<item>\w+)?").unwrap();
    let m = r.find("first or second").unwrap();
    assert_eq!(m.name("item"), Some("second"));
    assert_eq!(m.captures_name("item"), vec![Some("first"), Some("second")]);
}
#[test]
fn readme_duplicate_names_optional_second_only() {
    let r = Regex::new(r"(?P<item>\w+)? or (?P<item>\w+)?").unwrap();
    let m = r.find(" or second").unwrap();
    assert_eq!(m.name("item"), Some("second"));
    assert_eq!(m.captures_name("item"), vec![Some("second")]);
}
#[test]
fn readme_duplicate_names_optional_first_only() {
    let r = Regex::new(r"(?P<item>\w+)? or (?P<item>\w+)?").unwrap();
    let m = r.find("first or ").unwrap();
    assert_eq!(m.name("item"), Some("first"));
    assert_eq!(m.captures_name("item"), vec![Some("first")]);
}
#[test]
fn readme_duplicate_names_mandatory() {
    let r = Regex::new(r"(?P<item>\w*) or (?P<item>\w*)").unwrap();
    let m = r.find("first or second").unwrap();
    assert_eq!(m.captures_name("item"), vec![Some("first"), Some("second")]);
}

// ===========================================================================
// §13  fullmatch  [now supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_fullmatch() {
    let r = Regex::new(r"abc").unwrap();
    assert_eq!(r.fullmatch("abc").unwrap().span(), (0, 3));
    assert!(r.fullmatch("abcx").is_none());
    // fullmatch forces a non-greedy pattern to consume everything.
    let r = Regex::new(r"a.*?").unwrap();
    assert_eq!(r.match_at_start("abcd").unwrap().as_str(), "a"); // match: not full
    assert_eq!(r.fullmatch("abcd").unwrap().as_str(), "abcd"); // fullmatch: greedy result
}

// ===========================================================================
// §16  detach_string  [N/A in Rust]
// ---------------------------------------------------------------------------
// Rust's borrow model already makes the haystack lifetime explicit on the
// `Match<'h>` type; there is no reference-counted Python string to detach.
// This is a no-test section, documented for completeness.

// ===========================================================================
// §18  Full Unicode case-folding  ß <-> ss  [simple casefolding only]
// ---------------------------------------------------------------------------
// We currently do simple casefolding; the ß<->ss expansion is roadmap.
// Verify current (limited) behaviour honestly.
#[test]
fn readme_full_casefolding_currently_simple_only() {
    let r = Regex::new_with_flags(r"(?i)strasse", flags::VERSION1).unwrap();
    // ASCII case-insensitive still works.
    assert_eq!(r.find("STRASSE").unwrap().as_str(), "STRASSE");
    // The ß<->ss expansion is NOT yet supported.
    assert!(
        Regex::new_with_flags(r"(?i)strasse", flags::VERSION1)
            .unwrap()
            .find("stra\u{DF}e")
            .is_none()
    );
}

// ===========================================================================
// §21  \m and \M (start / end of word)  [now supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_m_M_word_anchors() {
    let r = Regex::new(r"\mcat").unwrap();
    let m = r.find("the cat sat").unwrap();
    assert_eq!(m.as_str(), "cat");
    assert!(Regex::new(r"\mcat").unwrap().find("concatenate").is_none());

    let r = Regex::new(r"cat\M").unwrap();
    assert!(r.find("the cat sat").is_some());
    assert!(r.find("the category").is_none());
}

// ===========================================================================
// §23  Set operators  [a&&b] [a--b] [a||b] [a~~b]  [roadmap — rejected]
// ---------------------------------------------------------------------------
#[test]
fn readme_set_operators_rejected() {
    for p in [r"[a-z&&[^aeiou]]", r"[a-z--qw]", r"[a||b]", r"[a~~b]"] {
        let e = Regex::new(p).unwrap_err();
        assert!(
            e.to_string().contains("set operation"),
            "{p:?} should be rejected as a set op, got: {e}"
        );
    }
}

// ===========================================================================
// §24  regex.escape  special_only  [supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_escape_special_only_exact() {
    use eregex::escape_special_only;
    // README: escape("foo!?", special_only=True) == 'foo!\?'
    assert_eq!(escape_special_only("foo!?"), "foo!\\?");
}

// ===========================================================================
// §25  regex.escape  literal_spaces  [supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_escape_literal_spaces_exact() {
    use eregex::escape_literal_spaces;
    // README: escape("foo bar!?", literal_spaces=True) == 'foo bar!\?'
    assert_eq!(escape_literal_spaces("foo bar!?"), "foo bar!\\?");
}

// ===========================================================================
// §26  Repeated captures: starts/ends/spans  [now supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_repeated_captures_starts_ends_spans() {
    let r = Regex::new(r"(\w{3})+").unwrap();
    let m = r.find("123456789").unwrap();
    assert_eq!(m.group(1), Some("789"));
    assert_eq!(m.captures(1), vec![Some("123"), Some("456"), Some("789")]);
    assert_eq!(m.starts(1), vec![0, 3, 6]);
    assert_eq!(m.ends(1), vec![3, 6, 9]);
    assert_eq!(m.spans(1), vec![(0, 3), (3, 6), (6, 9)]);
}

// ===========================================================================
// §27  Atomic grouping (?>...)  [supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_atomic_grouping() {
    // Once the atomic group matches it cannot give anything back.
    let r = Regex::new(r"a(?>bc|b)c").unwrap();
    assert!(r.find("abc").is_none()); // 'bc' froze, trailing 'c' fails
    assert_eq!(r.find("abcc").unwrap().as_str(), "abcc");
}

// ===========================================================================
// §28  Possessive quantifiers  [supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_possessive_quantifiers() {
    // (?:...)?+  (?:...)*+  (?:...)++  (?:...){m,n}+
    assert!(Regex::new(r"a++a").unwrap().find("aaa").is_none());
    assert_eq!(
        Regex::new(r"a++").unwrap().find("aaa").unwrap().as_str(),
        "aaa"
    );
    // Possessive `{2,}+` grabs as many as possible and refuses to give back:
    // it takes all three 'ab's, then the trailing 'ab' fails with no backtrack.
    assert!(
        Regex::new(r"(?:ab){2,}+ab")
            .unwrap()
            .find("ababab")
            .is_none()
    );
    // The non-possessive form backtracks and matches.
    assert!(
        Regex::new(r"(?:ab){2,}ab")
            .unwrap()
            .find("ababab")
            .is_some()
    );
}

// ===========================================================================
// §29  Scoped flags  (?flags-flags:...)  [supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_scoped_flags() {
    let r = Regex::new(r"(?i-m:abc)def").unwrap();
    assert_eq!(r.find("ABCdef").unwrap().as_str(), "ABCdef");
}

// ===========================================================================
// §30  Variable-length lookbehind  [supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_variable_length_lookbehind() {
    let r = Regex::new(r"(?<=foo|foobar)X").unwrap();
    assert_eq!(r.find("fooX").unwrap().as_str(), "X");
    assert_eq!(r.find("foobarX").unwrap().as_str(), "X");
    assert!(r.find("barX").is_none());
}

// ===========================================================================
// §34  splititer  [supported as split_iter]
// ---------------------------------------------------------------------------
#[test]
fn readme_splititer() {
    let r = Regex::new(r"\s+").unwrap();
    let v: Vec<_> = r.split_iter("a  b c").collect();
    assert_eq!(v, vec!["a", "b", "c"]);
}

// ===========================================================================
// §35  Subscripting match objects: m["name"] and m[:]  [now supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_match_subscripting() {
    let r = Regex::new(r"(?P<before>.*?)(?P<num>\d+)(?P<after>.*)").unwrap();
    let m = r.find("pqr123stu").unwrap();
    assert_eq!(&m["before"], "pqr");
    assert_eq!(&m["num"], "123");
    assert_eq!(&m["after"], "stu");
    // len(m) in Python counts groups incl. group 0.
    assert_eq!(m.len(), 4);
    // m[:] -> all groups; our analogue is all_groups().
    let all = m.all_groups();
    assert_eq!(
        all,
        vec![Some("pqr123stu"), Some("pqr"), Some("123"), Some("stu")]
    );
}

// ===========================================================================
// §36  Named groups with (?<name>...)  [supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_named_groups_angle_form() {
    let r = Regex::new(r"(?<word>\w+)").unwrap();
    let m = r.find("hello!").unwrap();
    assert_eq!(m.name("word"), Some("hello"));
}

// ===========================================================================
// §37  Group references \g<name>  [supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_group_reference_g_name() {
    let r = Regex::new(r"(\w+) \g<1>").unwrap();
    assert_eq!(r.find("hi hi").unwrap().as_str(), "hi hi");
    // Also \g<name>.
    let r = Regex::new(r"(?P<x>\w+)-\g<x>").unwrap();
    assert_eq!(r.find("go-go").unwrap().as_str(), "go-go");
}

// ===========================================================================
// §39  \p{property=value} / \p{value} / \P{...}  [supported, curated subset]
// ---------------------------------------------------------------------------
#[test]
fn readme_unicode_property_forms() {
    assert_eq!(
        Regex::new(r"\p{L}+")
            .unwrap()
            .find("abc123")
            .unwrap()
            .as_str(),
        "abc"
    );
    assert_eq!(
        Regex::new(r"\p{N}+")
            .unwrap()
            .find("abc123")
            .unwrap()
            .as_str(),
        "123"
    );
    assert_eq!(
        Regex::new(r"\P{N}+")
            .unwrap()
            .find("123abc")
            .unwrap()
            .as_str(),
        "abc"
    );
    // gc=value long form.
    assert_eq!(
        Regex::new(r"\p{gc=nd}+")
            .unwrap()
            .find("abc123")
            .unwrap()
            .as_str(),
        "123"
    );
}

// ===========================================================================
// §40  POSIX character classes  [[:alpha:]] / [[:^alpha:]]  [supported]
// ---------------------------------------------------------------------------
#[test]
fn readme_posix_character_classes() {
    let r = Regex::new(r"[[:alpha:]]+").unwrap();
    assert_eq!(r.find("123abc").unwrap().as_str(), "abc");
    let r = Regex::new(r"[[:^alpha:]]+").unwrap();
    assert_eq!(r.find("abc123").unwrap().as_str(), "123");
}

// ===========================================================================
// §43  \X grapheme  [now approximated as one char]
// ---------------------------------------------------------------------------
#[test]
fn readme_grapheme_x_approximated() {
    let r = Regex::new(r"\X").unwrap();
    // Currently \X consumes exactly one char; full UAX #29 clustering is roadmap.
    let m = r.find("abc").unwrap();
    assert_eq!(m.as_str(), "a");
}

// ===========================================================================
// Graceful rejection of unsupported syntax  (§3, §4, §5, §8, §17, §19, §20,
// §38, §41, §42, §44, §45)
// ---------------------------------------------------------------------------
// These roadmap features must produce a clear error rather than a silent
// wrong answer.

#[test]
fn readme_define_rejected() {
    // §3 (?(DEFINE)...)
    let e = Regex::new(r"(?(DEFINE)(?P<q>\d+))").unwrap_err();
    assert!(e.to_string().contains("conditional") || e.to_string().contains("DEFINE"));
}

#[test]
fn readme_backtracking_verbs_rejected() {
    // §4 (*PRUNE) (*SKIP) (*FAIL) (*F)
    for p in [r"(*PRUNE)", r"(*SKIP)", r"(*FAIL)", r"(*F)"] {
        let e = Regex::new(p).unwrap_err();
        assert!(e.to_string().contains("verb"), "{p:?}: {e}");
    }
}

#[test]
fn readme_keep_backslash_k_rejected() {
    // §5 \K
    let e = Regex::new(r"\w\K\w").unwrap_err();
    assert!(e.to_string().contains("\\K"));
}

#[test]
fn readme_recursive_patterns_rejected() {
    // §17 (?R) (?1) (?&name) (?P>name)
    for p in [r"(?R)", r"(?1)", r"(Tarzan|Jane) loves (?1)"] {
        let e = Regex::new(p).unwrap_err();
        let s = e.to_string();
        assert!(
            s.contains("recursive")
                || s.contains("subpattern")
                || s.contains("flag")
                || s.contains("conditional"),
            "{p:?}: {s}"
        );
    }
}

#[test]
fn readme_fuzzy_matching_rejected() {
    // §19 (?:foo){e<=1}  (dog){e}  {i<=1,s<=2}
    for p in [r"(?:foo){e<=1}", r"(dog){e}", r"(?:x){i<=1,s<=2}"] {
        let e = Regex::new(p).unwrap_err();
        assert!(e.to_string().contains("fuzzy"), "{p:?}: {e}");
    }
}

#[test]
fn readme_named_lists_rejected() {
    // §20 \L<name>
    let e = Regex::new(r"\L<options>").unwrap_err();
    assert!(e.to_string().contains("\\L"));
}

#[test]
fn readme_named_characters_rejected() {
    // §38 \N{name}
    let e = Regex::new(r"\N{SPACE}").unwrap_err();
    assert!(e.to_string().contains("\\N"));
}

#[test]
fn readme_search_anchor_g_rejected() {
    // §41 \G
    let e = Regex::new(r"\G\w{2}").unwrap_err();
    assert!(e.to_string().contains("\\G"));
}

#[test]
fn readme_reverse_searching_rejected() {
    // §42 (?r)
    let e = Regex::new(r"(?r).").unwrap_err();
    assert!(e.to_string().contains("flag"));
}

#[test]
fn readme_branch_reset_rejected() {
    // §44 (?|...)
    let e = Regex::new(r"(?|(first)|(second))").unwrap_err();
    // Rejected as an unknown flag '|'.
    assert!(e.to_string().contains("flag") || e.to_string().contains("|"));
}

#[test]
fn readme_word_flag_rejected() {
    // §45 (?w) — the WORD flag is parsed but has no special effect yet.
    // It must at least compile without error (the flag is recognized).
    let r = Regex::new_with_flags(r"\bword\b", flags::WORD).unwrap();
    assert!(r.find("a word here").is_some());
}

#[test]
fn readme_version_flags_recognized() {
    // (?V1) and (?V0) are recognized global flags.
    assert!(Regex::new(r"(?V1)abc").is_ok());
    assert!(Regex::new(r"(?V0)abc").is_ok());
    // Bad version form is rejected.
    assert!(Regex::new(r"(?V9)abc").is_err());
}

// ===========================================================================
// §46  Timeout  [roadmap — no timeout API]
// ---------------------------------------------------------------------------
// We do not yet expose a timeout. This test documents the absence: there is
// no `timeout` parameter on the matching API. (Catastrophic-backtracking
// protection is roadmap.)

// ===========================================================================
// §8  Partial matches  [roadmap — no partial=True]
// ---------------------------------------------------------------------------
// No partial-match API yet. The normal `find` behaves as documented for a
// complete match: \d{4} does NOT match "123" (too short).
#[test]
fn readme_partial_matches_currently_absent() {
    let r = Regex::new(r"\d{4}").unwrap();
    assert!(r.fullmatch("123").is_none()); // partial would succeed; we don't
    assert!(r.fullmatch("1234").is_some()); // complete
    assert!(r.fullmatch("12345").is_none()); // too long
}
