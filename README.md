# pregex

An advanced regular expression engine for Rust, inspired by
[mrab-regex](https://github.com/mrabarnett/mrab-regex) (the Python `regex`
module).

`pregex` aims to bring the richer feature set of `mrab-regex` to Rust:

* Named groups, duplicate group names, repeated captures
* Greedy / lazy / possessive quantifiers
* Atomic groups `(?>...)`
* Variable-length lookbehind
* Nested character-class set operations `[a-z&&[^aeiou]]` _(planned)_
* Inline, scoped flags `(?flags-flags:...)`
* Backreferences `\1`, `\g<name>`, `(?P=name)`
* Unicode properties `\p{L}`, `\P{^N}` _(subset)_
* Fuzzy / approximate matching `(?:foo){e<=1}` _(planned)_
* Recursive patterns `(?R)`, `(?(DEFINE)...)` _(planned)_
* Partial matches, POSIX matching, reverse search _(planned)_

This crate currently implements a strong foundation. See **Feature status**
below for what is ready today and what is on the roadmap.

## Quick start

```rust
use pregex::{Regex, flags};

let re = Regex::new(r"(\w+)\s+(\w+)").unwrap();
let m = re.find("hello world").unwrap();
assert_eq!(m.group(1), Some("hello"));
assert_eq!(m.group(2), Some("world"));

let re = Regex::new_with_flags(r"(?i)hello", flags::IGNORECASE).unwrap();
assert!(re.is_match("HELLO, World"));

// Repeated captures (signature mrab-regex feature)
let re = Regex::new(r"(\w)+").unwrap();
let m = re.find("abc").unwrap();
assert_eq!(m.captures(1), vec![Some("a"), Some("b"), Some("c")]);
```

## Feature status

### Implemented

* Literals, `.`, anchors `^ $ \A \z \b \B`
* Predefined classes `\d \D \w \W \s \S` (ASCII + Unicode via `std`)
* Character classes `[...]` with ranges, negation, escapes
* Alternation `a|b|c`
* Quantifiers `* + ? {m} {m,} {m,n}` with greedy `?`-lazy and `+`-possessive
* Capturing / non-capturing / named groups `(...) (?:...) (?P<n>...) (?<n>...)`
* Atomic groups `(?>...)`
* Backreferences `\1 \g<n> \g<name> (?P=name)`
* Lookahead / lookbehind `(?=...) (?!...) (?<=...) (?<!...)` (variable length)
* Inline scoped flags `(?i) (?i:...) (?i-m:...)`
* Inline comments `(?#...)` and free-spacing (`VERBOSE`)
* Named & unicode properties `\p{...}` (a curated subset)
* Repeated captures (`captures`, `captures_iter`)
* `is_match`, `find`, `find_at`, `find_iter`, `captures`, `captures_iter`
* `replace`, `replace_all` with `$1` / `${name}` / `$$` templates
* `split`, `split_iter`
* `escape`

### Roadmap (signature mrab-regex features)

* Fuzzy / approximate matching `{e<=2}`
* Recursive patterns & subexpression calls `(?R) (?1) (?&name) (?(DEFINE)...)`
* Branch reset `(?|...|...)`
* Nested set operations `[a&&b] [a--b] [a||b] [a~~b]`
* Full Unicode case-folding (ß ↔ ss); currently simple casefolding
* `\K`, `(*PRUNE)`, `(*SKIP)`, `(*FAIL)`, `\G` semantics
* POSIX (`leftmost-longest`) and reverse (`(?r)`) matching modes
* Partial matching, concurrent/GIL-free, timeouts
* `\L<name>` named lists

## License

Apache-2.0, matching the upstream `mrab-regex` project.
