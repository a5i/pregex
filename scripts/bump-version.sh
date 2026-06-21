#!/usr/bin/env bash
# Bump the project version everywhere and keep all six manifests in sync.
#
# The single source of truth for the four Rust crates is
# `[workspace.package].version` in the root Cargo.toml; every member crate
# uses `version.workspace = true`. This script updates that one line plus the
# npm (package.json) and PyPI (pyproject.toml) versions, then refreshes the two
# lockfiles (Cargo.lock, package-lock.json) and verifies everything agrees.
#
# Usage:
#   ./scripts/bump-version.sh 0.2.0
#
# Afterwards, commit and tag:
#   git commit -am "Release v0.2.0"
#   git tag v0.2.0 && git push origin v0.2.0
# The release workflow's `check-version` job verifies all versions match the tag.

set -euo pipefail

die() { printf 'bump-version: %s\n' "$*" >&2; exit 1; }

# --- args + validation ----------------------------------------------------

NEW="${1:-}"
[ -n "$NEW" ] || die "usage: $0 <version>   (e.g. 0.2.0, 1.0.0-rc.1)"

printf '%s' "$NEW" | grep -Eq \
  '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$' \
  || die "'$NEW' is not a valid semver (expecting e.g. 0.2.0)"

# Run from the repo root regardless of where the script is invoked from.
cd "$(dirname "$0")/.."

[ -f Cargo.toml ] && [ -d crates/eregex-node ] && [ -d crates/eregex-python ] && [ -d crates/eregex-wasm ] \
  || die "not running from a project root (expected Cargo.toml + crates/eregex-*)"

# --- helper: portable in-place sed ----------------------------------------
# GNU and BSD sed disagree on `-i`; the `.bak` suffix form works on both.
inplace() {  # inplace <pattern> <file>
  sed -i.bak "$1" "$2" && rm -f "$2.bak"
}

OLD=$(awk '
  /^\[workspace\.package\]/ { in_ws=1; next }
  /^\[/                      { in_ws=0 }
  in_ws && /^version *=/     { gsub(/version *= *"|"$/, ""); print; exit }
' Cargo.toml)
[ -n "$OLD" ] || die "could not find [workspace.package].version in Cargo.toml"
printf 'bumping %s -> %s\n\n' "$OLD" "$NEW"

# --- 1. Cargo (workspace source of truth) ---------------------------------
# Only the root Cargo.toml carries a literal version now (under
# [workspace.package]); all three binding crates inherit it.

inplace "s|^version = \".*\"|version = \"$NEW\"|" Cargo.toml

# --- 2. npm (package.json) ------------------------------------------------

NODE=crates/eregex-node/package.json
inplace "s|\"version\": \".*\"|\"version\": \"$NEW\"|" "$NODE"

# The wasm dev package.json carries a cosmetic version too (the published
# wasm package's version is stamped from Cargo by wasm-pack). Keep it in sync
# so every manifest agrees.
WASM=crates/eregex-wasm/package.json
inplace "s|\"version\": \".*\"|\"version\": \"$NEW\"|" "$WASM"

# --- 3. PyPI (pyproject.toml) ---------------------------------------------

PY=crates/eregex-python/pyproject.toml
inplace "s|^version = \".*\"|version = \"$NEW\"|" "$PY"

# --- 4. refresh lockfiles -------------------------------------------------

# Cargo.lock: re-resolve the workspace so member versions update.
cargo update --offline 2>/dev/null || cargo update

# package-lock.json: let npm rewrite it from package.json (no node_modules /
# build-script churn). `--prefer-offline` uses the cache when possible.
( cd crates/eregex-node && npm install --package-lock-only --ignore-scripts --prefer-offline )

# --- 5. verify everything agrees ------------------------------------------

# Cargo: every crate inherits [workspace.package].version, so `cargo metadata`
# (which dereferences inheritance) must yield exactly one version.
CARGO_VERSIONS=$(cargo metadata --format-version 1 --no-deps \
  | grep -o '"version":"[^"]*"' | sed 's/"version":"//; s/"//' | sort -u)
[ "$(printf '%s\n' "$CARGO_VERSIONS" | wc -l)" -eq 1 ] \
  || die "cargo crates disagree on version:\n$CARGO_VERSIONS"

NODE_V=$(grep -m1 '"version"' "$NODE" | sed 's/.*: "\(.*\)".*/\1/' | tr -d ',')
WASM_V=$(grep -m1 '"version"' "$WASM" | sed 's/.*: "\(.*\)".*/\1/' | tr -d ',')
PY_V=$(grep -m1 '^version =' "$PY" | sed 's/version = "\(.*\)"/\1/')
LOCK_V=$(grep -m1 '"version"' crates/eregex-node/package-lock.json \
  | sed 's/.*: "\(.*\)".*/\1/' | tr -d ',')

echo "cargo (all crates) = $CARGO_VERSIONS"
echo "npm node           = $NODE_V"
echo "npm wasm           = $WASM_V"
echo "npm package-lock   = $LOCK_V"
echo "pypi pyproject     = $PY_V"

[ "$CARGO_VERSIONS" = "$NEW" ] || die "cargo version is '$CARGO_VERSIONS', expected '$NEW'"
[ "$NODE_V"          = "$NEW" ] || die "node package.json is '$NODE_V', expected '$NEW'"
[ "$WASM_V"          = "$NEW" ] || die "wasm package.json is '$WASM_V', expected '$NEW'"
[ "$LOCK_V"          = "$NEW" ] || die "package-lock.json is '$LOCK_V', expected '$NEW'"
[ "$PY_V"            = "$NEW" ] || die "pyproject.toml is '$PY_V', expected '$NEW'"

printf '\nOK: all manifests at %s\n' "$NEW"
printf 'next: git commit -am "Release v%s" && git tag v%s\n' "$NEW" "$NEW"
