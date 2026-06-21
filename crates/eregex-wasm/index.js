'use strict';

// Package entry for `eregex-wasm`.
//
// `wasm-bindgen` cannot export `pub const` values, so the flag bits are
// delivered as a plain object from the Rust function `flags()`. This wrapper
// spreads that object onto the module so callers see the same
// `eregex.IGNORECASE` / `eregex.MULTILINE` / ... numeric properties as in the
// native Node bindings — making this package a drop-in for `eregex`
// (napi-rs) wherever the wasm target is preferred.
//
// The generated wasm-bindgen glue lives in `./eregex_wasm.js` (produced by
// `wasm-pack build --target nodejs`); `scripts/assemble-pkg.cjs` copies this
// file into `pkg/` and sets it as `package.json#main` after each build.

const wasm = require('./eregex_wasm.js');

module.exports = Object.assign({}, wasm, wasm.flags());
