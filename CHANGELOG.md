# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **WASM bindings** (`crates/eregex-wasm`): a `wasm-bindgen` / `wasm-pack`
  package (published as `eregex-wasm` on npm) that exposes the same API as the
  native Node bindings, built with `--target nodejs`. Absent values serialize
  to JS `null` via `serde-wasm-bindgen` (not `undefined`), so it is a drop-in
  for the napi-rs package; a hand-written `index.js` entry materializes the
  flag constants that `wasm-bindgen` cannot export as `const`. CI builds and
  tests it, and the release workflow publishes it to npm.
- **Partial matching** via [`Regex::find_partial`]: an end-anchored search that
  reports whether the haystack is a *full* match, a *partial* match (a prefix
  of some full match, cut short by end-of-input), or *no match* (a hard
  mismatch before end-of-input). New public types: `PartialMatch`,
  `MatchStatus`, `GroupMatch`.
- Crate-level documentation rendered from `README.md`, plus rustdoc lints
  (`broken_intra_doc_links`, `bare_urls`) so the generated docs stay clean.
- `# Examples` / `# Errors` doc sections with compile-checked doctests across
  the main `Regex` API surface.
- Example `examples/gap_match.rs`: a user-space recipe for gap-tolerant
  ("fuzzy") matching built on `find_at` + `find_partial`.
- `CHANGELOG.md`, `documentation` / `rust-version` metadata in `Cargo.toml`,
  and `[package.metadata.docs.rs]`.

### Changed
- `State`, `Snapshot`, and `PartialCandidate` now also track open capturing
  groups, so partial matches can report the group that was entered but not
  completed. Backtracking choice points save/restore this state alongside
  captures.

## [0.1.0] — baseline

Initial public surface of the engine:

- Literals, `.`, anchors (`^ $ \A \z \b \B \m \M`), predefined classes
  (`\d \D \w \W \s \S`), user character classes with ranges/negation/escapes,
  POSIX classes, and a curated subset of Unicode properties (`\p{...}`).
- Alternation, quantifiers (`* + ? {m} {m,} {m,n}`) with greedy/lazy/possessive
  flavours, atomic groups `(?>...)`.
- Capturing / non-capturing / named groups, backreferences
  (`\1 \g<...> (?P=...)`), lookahead / lookbehind (variable length), inline
  scoped flags (`(?i) (?i-m:...)`), inline comments and free-spacing (`(?x)`).
- Repeated captures (`captures`, `captures_iter`), and the matching API:
  `is_match`, `find`, `find_at`, `find_iter`, `captures`, `captures_iter`,
  `match_at_start`, `fullmatch`, `replace`, `replace_all`, `split`, `escape`.

[Unreleased]: https://github.com/a5i/eregex/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/a5i/eregex/releases/tag/v0.1.0
