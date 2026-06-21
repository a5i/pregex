//! The public module-level API surface of `eregex::*`.

use eregex::{Regex, flags};

// --- module-level convenience functions -----------------------------------

#[test]
fn module_is_match() {
    assert!(eregex::is_match(r"\d+", "abc 123"));
    assert!(!eregex::is_match(r"\d+", "abc"));
    // bad pattern -> false, not panic
    assert!(!eregex::is_match(r"(abc", "abc"));
}

#[test]
fn module_find() {
    let m = eregex::find(r"\w+", "  hello").unwrap();
    assert_eq!(m.as_str(), "hello");
    assert!(eregex::find(r"\d", "no digits").is_none());
}

#[test]
fn module_find_all() {
    let ms = eregex::find_all(r"\d", "a1b2c3").unwrap();
    assert_eq!(ms.len(), 3);
    assert_eq!(ms[0].as_str(), "1");
    let bad = eregex::find_all(r"(unclosed", "x");
    assert!(bad.is_err());
}

#[test]
fn module_new_and_new_with_flags() {
    let r = eregex::new(r"abc").unwrap();
    assert!(r.is_match("abc"));
    let r = eregex::new_with_flags(r"abc", flags::IGNORECASE).unwrap();
    assert!(r.is_match("ABC"));
}

#[test]
fn module_replace_and_replace_all() {
    assert_eq!(eregex::replace(r"\d", "a1b", "X").unwrap(), "aXb");
    assert_eq!(eregex::replace_all(r"\d", "a1b2", "X").unwrap(), "aXbX");
}

#[test]
fn module_split() {
    assert_eq!(eregex::split(r"\s", "a b c").unwrap(), vec!["a", "b", "c"]);
}

// --- Regex accessors ------------------------------------------------------

#[test]
fn regex_accessors() {
    let r = Regex::new(r"(?P<name>\w+)\s+(?P<n>\d+)").unwrap();
    assert_eq!(r.as_str(), r"(?P<name>\w+)\s+(?P<n>\d+)");
    assert_eq!(r.capture_count(), 2);
    assert_eq!(r.group_index("name"), Some(1));
    assert_eq!(r.group_index("n"), Some(2));
    assert_eq!(r.group_index("missing"), None);
    let names = r.group_names();
    assert_eq!(names.get("name"), Some(&1));
    assert_eq!(names.get("n"), Some(&2));
}

#[test]
fn regex_flags_accessor() {
    let r = Regex::new_with_flags(r"a", flags::IGNORECASE | flags::MULTILINE).unwrap();
    let f = r.flags();
    assert!(f.contains(flags::IGNORECASE | flags::MULTILINE));
}

#[test]
fn regex_find_at_offset() {
    let r = Regex::new(r"\d+").unwrap();
    let m = r.find_at("a12 b34 c56", 7).unwrap(); // byte 7 == "c56"
    assert_eq!(m.as_str(), "56");
}

#[test]
fn regex_captures_alias_and_iter() {
    let r = Regex::new(r"(\w)(\d)").unwrap();
    let m = r.captures("a1").unwrap();
    assert_eq!(m.group(1), Some("a"));
    let ms: Vec<_> = r
        .captures_iter("a1 b2 c3")
        .map(|m| m.as_str().to_string())
        .collect();
    assert_eq!(ms, vec!["a1", "b2", "c3"]);
}

#[test]
fn regex_split_iter() {
    let r = Regex::new(r",").unwrap();
    let v: Vec<_> = r.split_iter("a,b,,c").collect();
    assert_eq!(v, vec!["a", "b", "", "c"]);
}

#[test]
fn regex_dump_runs() {
    let r = Regex::new(r"(a|b)+").unwrap();
    let s = r.dump();
    assert!(s.contains("Branch") || s.contains("Repeat"));
}

// --- Match accessors ------------------------------------------------------

#[test]
fn match_span_of_and_start_end_of() {
    let r = Regex::new(r"(\w+)@(\w+)").unwrap();
    let m = r.find("mail user@host end").unwrap();
    assert_eq!(m.span(), (5, 14));
    assert_eq!(m.span_of(0), (5, 14));
    assert_eq!(m.start_of(1), 5);
    assert_eq!(m.end_of(1), 9);
    assert_eq!(m.start_of(2), 10);
    assert_eq!(m.end_of(2), 14);
    // Non-participating group returns end-of-string (Python semantics).
    let r = Regex::new(r"(a)|(b)").unwrap();
    let m = r.find("b").unwrap();
    assert_eq!(m.start_of(1), 1); // group 1 didn't match
}

#[test]
fn match_len_and_is_empty() {
    let r = Regex::new(r"(a)(b)(c)").unwrap();
    let m = r.find("abc").unwrap();
    assert_eq!(m.len(), 4); // group 0 + 3 captures
    assert!(!m.is_empty());
}

#[test]
fn match_index_operator() {
    let r = Regex::new(r"(\w)(\d)").unwrap();
    let m = r.find("x5").unwrap();
    assert_eq!(&m[0], "x5");
    assert_eq!(&m[1], "x");
    assert_eq!(&m[2], "5");
}

#[test]
fn match_named_groups_map() {
    let r = Regex::new(r"(?P<a>\d+)-(?P<b>\d+)").unwrap();
    let m = r.find("12-34").unwrap();
    let g = m.named_groups();
    assert_eq!(g.get("a").copied(), Some("12"));
    assert_eq!(g.get("b").copied(), Some("34"));
}

#[test]
fn match_debug_format() {
    let r = Regex::new(r"\d+").unwrap();
    let m = r.find("x42y").unwrap();
    let s = format!("{:?}", m);
    assert!(s.contains("42"));
}
