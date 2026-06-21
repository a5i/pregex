# eregex

An advanced regular expression engine for Rust, inspired by
[mrab-regex](https://github.com/mrabarnett/mrab-regex) (the Python `regex`
module).

`eregex` aims to bring the richer feature set of `mrab-regex` to Rust:

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
use eregex::{Regex, flags};

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

// Partial matching: is the input a prefix of some full match?
let re = Regex::new(r"token=([a-z]+)([0-9]+)").unwrap();
// "token=abc" is incomplete — more input could turn it into a full match.
let p = re.find_partial("xxx token=abc").unwrap();
assert!(p.is_partial());
assert_eq!(p.matched, "token=abc");
// Group 1 fully matched, group 2 is still empty/partial.
assert_eq!(p.group(1), Some("abc"));
assert_eq!(p.group(2), Some(""));
// A wrong character rules out any continuation -> no match at all.
assert!(re.find_partial("xxx token=abc!").is_none());
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
* Partial (end-anchored) matching via `find_partial`
* Inline scoped flags `(?i) (?i:...) (?i-m:...)`
* Inline comments `(?#...)` and free-spacing (`VERBOSE`)
* Named & unicode properties `\p{...}` (a curated subset)
* Repeated captures (`captures`, `captures_iter`)
* `is_match`, `find`, `find_at`, `find_iter`, `find_partial`, `captures`, `captures_iter`
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
* Concurrent/GIL-free operation, timeouts
* `\L<name>` named lists

## Core concepts

* [`Regex`](https://docs.rs/eregex/latest/eregex/struct.Regex.html) — a
  compiled pattern. Compile once with [`Regex::new`] (or
  [`Regex::new_with_flags`]), then search many inputs.
* [`Match`](https://docs.rs/eregex/latest/eregex/struct.Match.html) — a
  successful full match, with group lookup by index or name and full repeated-
  capture history.
* [`PartialMatch`](https://docs.rs/eregex/latest/eregex/struct.PartialMatch.html) —
  the result of [`Regex::find_partial`], carrying a [`MatchStatus`] of `Full`
  or `Partial` and per-group [`GroupMatch`] state.
* [`Flags`](https://docs.rs/eregex/latest/eregex/flags/struct.Flags.html) and
  the [`flags`](https://docs.rs/eregex/latest/eregex/flags/index.html) module —
  compile-time flags (`IGNORECASE`, `MULTILINE`, `DOTALL`, …) and their inline
  `(?im)` syntax.

## Error handling

All fallible operations return [`Result<T, Error>`](https://docs.rs/eregex/latest/eregex/error/type.Result.html).
[`Error`](https://docs.rs/eregex/latest/eregex/error/struct.Error.html) carries
an [`ErrorKind`](https://docs.rs/eregex/latest/eregex/error/enum.ErrorKind.html)
(syntax error, bad escape, bad quantifier, unknown group, …) plus the byte
offset in the pattern where the problem was detected, when known.

```rust
use eregex::Regex;

let err = Regex::new(r"(").unwrap_err();
println!("{}", err); // e.g. "eregex error at position 1: unclosed group"
```

## Examples

The [`examples/`](./examples) directory contains runnable programs:

* [`demo.rs`](./examples/demo.rs) — a tour of the core API.
* [`gap_match.rs`](./examples/gap_match.rs) — gap-tolerant ("fuzzy") matching
  built on `find_at` + `find_partial`, for inputs where the target is split by
  noise (a workaround while in-pattern fuzzy matching is on the roadmap).

Run them with `cargo run --example demo` / `cargo run --example gap_match`.

## Development

A shared pre-commit hook runs `cargo fmt --all --check` and
`cargo test --workspace` before each commit. Enable it once per clone:

```sh
git config core.hooksPath .githooks
```

Bypass it for a single commit with `git commit --no-verify`.

## Compatibility

* **MSRV:** 1.85 (uses the 2024 edition).
* **License:** Apache-2.0.
* `#![forbid(unsafe_code)]` is enforced crate-wide.

## License

Apache-2.0, matching the upstream `mrab-regex` project.

[`Regex::new`]: https://docs.rs/eregex/latest/eregex/struct.Regex.html#method.new
[`Regex::new_with_flags`]: https://docs.rs/eregex/latest/eregex/struct.Regex.html#method.new_with_flags
[`Regex::find_partial`]: https://docs.rs/eregex/latest/eregex/struct.Regex.html#method.find_partial
[`MatchStatus`]: https://docs.rs/eregex/latest/eregex/enum.MatchStatus.html
[`GroupMatch`]: https://docs.rs/eregex/latest/eregex/enum.GroupMatch.html
