# eregex-wasm

WebAssembly bindings for [`eregex`](https://github.com/a5i/eregex) — an
advanced regular expression engine for Rust inspired by mrab-regex (the Python
`regex` module).

This package exposes eregex's full API to JavaScript / TypeScript via
[`wasm-bindgen`](https://rustwasm.github.io/wasm-bindgen/) + [`wasm-pack`](https://rustwasm.github.io/docs/wasm-pack/).
All matching logic runs in compiled Rust shipped as a single `.wasm` artifact;
the JavaScript layer is a thin adapter.

It mirrors the native Node bindings ([`eregex`](https://www.npmjs.com/package/eregex),
napi-rs) method-for-method: the same `Regex` / `Match` / `PartialMatch`
classes, the same flag constants, the same `null`-on-absent semantics. Code
written against one works unchanged against the other.

| package     | engine     | best for                                  |
| ----------- | ---------- | ----------------------------------------- |
| `eregex`     | napi-rs    | fastest native execution on Node          |
| `eregex-wasm`| wasm-bindgen | a single portable binary; also bundler/browser-buildable |

## Build

The package is built from the Rust core with `wasm-pack`:

```bash
cd crates/eregex-wasm
rustup target add wasm32-unknown-unknown   # one-time
cargo install wasm-pack --locked           # one-time (or use a CI action)
npm run build
# → pkg/eregex_wasm.js + pkg/eregex_wasm_bg.wasm + pkg/index.js (assembled)
```

`npm run build` runs `wasm-pack build --target nodejs --release` and then
`scripts/assemble-pkg.cjs`, which drops the hand-written `index.js` entry into
`pkg/` and sets it as `main`. The assembled `pkg/` is what gets published to
npm.

The `pkg/` directory is generated; it is not checked in.

## Quick start

```js
const { Regex, IGNORECASE, parseFlags } = require('eregex-wasm');

const re = new Regex(String.raw`(\w+)\s+(\w+)`);
const m = re.find('hello world');
console.log(m.matched);      // 'hello world'
console.log(m.group(1));     // 'hello'
console.log(m.group(2));     // 'world'

// Flags: pass a bitset of the exported constants, or parse a string.
new Regex('hello', IGNORECASE).isMatch('HELLO');         // true
new Regex('hello', parseFlags('i')).isMatch('HELLO');    // true

// Repeated captures (signature mrab-regex feature).
new Regex(String.raw`(\w)+`).find('abc').captures(1);    // ['a', 'b', 'c']

// Replace with named groups.
new Regex(String.raw`(?P<a>\d)(?P<b>\d)`).replaceAll('12 34', '${b}${a}'); // '21 43'
```

## `Regex`

```ts
class Regex {
  constructor(pattern: string, flags?: number)
  get pattern(): string
  get flags(): number         // resolved flags (defaults UNICODE + VERSION1 are added)
  get captureCount(): number  // capturing groups (group 0 excluded)
  groupNames(): string[]
  groupIndex(name: string): number | null

  isMatch(haystack: string): boolean
  find(haystack: string): Match | null
  findAt(haystack: string, start: number): Match | null
  matchAtStart(haystack: string): Match | null   // like re.match
  fullMatch(haystack: string): Match | null       // like re.fullmatch
  findAll(haystack: string): Match[]
  findPartial(haystack: string): PartialMatch | null

  replace(haystack: string, repl: string): string
  replaceAll(haystack: string, repl: string): string
  split(haystack: string): string[]
  dump(): string                                  // parsed AST (debug aid)
}
```

`flags` is a bitwise OR of the exported constants: `IGNORECASE`, `MULTILINE`,
`DOTALL`, `UNICODE`, `ASCII`, `VERBOSE`, `FULLCASE`, `WORD`, `LOCALE`,
`VERSION0`, `VERSION1`. `parseFlags("ims")` parses a flag string for
`RegExp`-familiar ergonomics.

## `Match`

```ts
class Match {
  get matched(): string       // whole match (group 0)
  get input(): string         // original haystack
  get start(): number         // byte offset
  get end(): number
  get span(): { start: number; end: number }
  get captureCount(): number
  get groups(): (string | null)[]             // current text, group 0 first
  get namedGroups(): Record<string, string>
  get allCaptures(): (string | null)[][]      // repeated-capture history
  get capturesDict(): Record<string, (string | null)[]>

  group(index: number): string | null
  namedGroup(name: string): string | null
  captures(index: number): (string | null)[]
  capturesByName(name: string): (string | null)[]
  spanOf(index: number): { start: number; end: number } | null
}
```

All offsets are **byte offsets** (UTF-8), matching Python's `re` and the Rust
core. `null` is returned for groups that did not participate.

## `PartialMatch`

`findPartial` is end-anchored: the match must consume the input to its end.

```ts
class PartialMatch {
  get status(): 'full' | 'partial'
  get isFull(): boolean
  get isPartial(): boolean
  get matched(): string
  get start(): number
  get end(): number
  get captureCount(): number

  group(index: number): string | null
  namedGroup(name: string): string | null
  groupState(index: number): 'matched' | 'partial' | 'none'
}
```

- `null` from `findPartial` → the input **cannot** be a prefix of any match.
- `status === 'partial'` → the input is a valid prefix of some full match
  (more input could complete it).

```js
const re = new Regex(String.raw`token=([a-z]+)([0-9]+)`);
const p = re.findPartial('x token=abc');
p.isPartial;            // true
p.group(1);             // 'abc'
p.groupState(1);        // 'matched'
p.groupState(2);        // 'partial'   (entered but not completed)

re.findPartial('x token=abc!'); // null — '!' rules out any continuation
```

## Module-level helpers

```ts
escape(s: string): string
escapeSpecialOnly(s: string): string
escapeLiteralSpaces(s: string): string
isMatch(pattern: string, haystack: string): boolean  // compiles pattern once
parseFlags(flagStr: string): number
flags(): { IGNORECASE: number; MULTILINE: number; /* ... */ }
```

## Notes on the wasm target

- Built for `wasm32-unknown-unknown` and packaged with `--target nodejs`, so
  it loads as a plain CommonJS module in Node 10+ with no bundler required.
- `wasm-bindgen` cannot export `const` values, so the flag bits are produced
  by `flags()` and spread onto the module by `index.js`. Callers see ordinary
  numeric properties (`eregex-wasm.IGNORECASE`, ...).
- Absent values come back as JS `null` (not `undefined`): nullable returns are
  routed through `serde-wasm-bindgen`, which maps `Option::None → null`, so
  `=== null` and `deepStrictEqual(..., null)` behave exactly like the native
  package.
- To target browsers or bundlers instead of Node, rebuild with
  `wasm-pack build --target web` (or `bundler`) and drop the `index.js`
  CommonJS wrapper.

## Testing

```bash
npm test          # runs test/smoke.js against the assembled pkg/
npm run test:all  # build, then test
```

## Layout

This is one facet of eregex's binding story. The same Rust core (`eregex`)
also ships native Node bindings (`eregex`, napi-rs) and Python bindings
(`eregex`, pyo3). See the project root for the core crate and its feature
matrix.

## License

Apache-2.0, matching the upstream `mrab-regex` project.
