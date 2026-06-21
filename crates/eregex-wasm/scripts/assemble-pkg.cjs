'use strict';

// Post-build assembly: turn wasm-pack's `pkg/` into a publishable package
// whose entry (`index.js`) re-exports the wasm glue with the flag constants
// materialized as properties (wasm-bindgen can't export `const` values).
//
// Run automatically by `npm run build` after `wasm-pack build`.

const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const pkgDir = path.join(root, 'pkg');
const pkgJsonPath = path.join(pkgDir, 'package.json');

if (!fs.existsSync(pkgJsonPath)) {
  console.error('assemble-pkg: pkg/package.json not found — run wasm-pack first');
  process.exit(1);
}

// 1. Ship the hand-written entry + types next to the generated glue.
fs.copyFileSync(path.join(root, 'index.js'), path.join(pkgDir, 'index.js'));
fs.copyFileSync(path.join(root, 'index.d.ts'), path.join(pkgDir, 'index.d.ts'));

// 2. Point the published package at the wrapper and make sure it ships.
const pkg = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));
pkg.main = 'index.js';
pkg.types = 'index.d.ts';
pkg.files = Array.from(
  new Set([...(pkg.files || []), 'index.js', 'index.d.ts']),
).sort();
fs.writeFileSync(pkgJsonPath, JSON.stringify(pkg, null, 2) + '\n');

console.log('assemble-pkg: set main/types -> index.js, refreshed files list');
