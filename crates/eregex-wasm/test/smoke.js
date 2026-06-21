'use strict';

// Drop-in twin of `crates/eregex-node/test/smoke.js`: the wasm package exposes
// the same API (classes, methods, flag constants) as the native napi-rs
// package, so the same assertions must pass against the assembled `pkg/`.

const assert = require('assert');
const P = require('../pkg/index.js');

function eq(actual, expected, msg) {
  assert.deepStrictEqual(actual, expected, msg);
}

// --- construction & flags -------------------------------------------------

const re = new P.Regex(String.raw`(\w+)\s+(\w+)`);
eq(re.pattern, String.raw`(\w+)\s+(\w+)`, 'pattern getter');
eq(re.captureCount, 2, 'captureCount');

const reFlags = new P.Regex('hello', P.IGNORECASE);
// `flags` returns resolved flags (defaults UNICODE + VERSION1 are added), so
// check membership with bitwise AND rather than equality.
assert.ok((reFlags.flags & P.IGNORECASE) !== 0, 'IGNORECASE is set in flags');
assert.ok((new P.Regex('hello', P.parseFlags('i')).flags & P.IGNORECASE) !== 0, 'parseFlags("i") sets IGNORECASE');
eq(P.parseFlags('im'), P.IGNORECASE | P.MULTILINE, 'parseFlags("im") raw bits');

assert.throws(() => new P.Regex('('), /eregex|\(|pattern/i, 'bad pattern throws');

// --- find / group accessors -----------------------------------------------

const m = re.find('hello world');
assert.ok(m, 'find returns a match');
eq(m.matched, 'hello world', 'matched (group 0)');
eq(m.start, 0, 'start');
eq(m.end, 11, 'end');
eq(m.span, { start: 0, end: 11 }, 'span object');
eq(m.input, 'hello world', 'input');
eq(m.group(1), 'hello', 'group(1)');
eq(m.group(2), 'world', 'group(2)');
eq(m.group(99), null, 'group out of range -> null');
eq(m.groups, ['hello world', 'hello', 'world'], 'groups array');

eq(new P.Regex(String.raw`(?P<host>\w+)=(?P<port>\d+)`).find('srv=8080').namedGroups, {
  host: 'srv',
  port: '8080',
}, 'namedGroups');
eq(re.find('oneword'), null, 'no match -> null');

// --- repeated captures ----------------------------------------------------

const m3 = new P.Regex(String.raw`(\w)+`).find('abc');
eq(m3.captures(1), ['a', 'b', 'c'], 'repeated captures(1)');

const m4 = new P.Regex(String.raw`(?P<c>\w)+`).find('xy');
eq(m4.capturesByName('c'), ['x', 'y'], 'capturesByName');
eq(m4.capturesDict.c, ['x', 'y'], 'capturesDict getter');

// --- matchAtStart / fullMatch ---------------------------------------------

const digits = new P.Regex(String.raw`\d+`);
eq(digits.matchAtStart('123abc').matched, '123', 'matchAtStart');
eq(digits.matchAtStart('abc123'), null, 'matchAtStart no anchor');
eq(digits.fullMatch('123').matched, '123', 'fullMatch ok');
eq(new P.Regex(String.raw`\d{3}`).fullMatch('1234'), null, 'fullMatch too long');

// --- findAll --------------------------------------------------------------

const all = new P.Regex(String.raw`\d+`).findAll('a1 bb 22 c333');
eq(all.map((x) => x.matched), ['1', '22', '333'], 'findAll');

// --- replace / replaceAll / split -----------------------------------------

const replacer = new P.Regex(String.raw`(?P<a>\d)(?P<b>\d)`);
eq(replacer.replace('12 x', '${b}${a}'), '21 x', 'replace');
eq(replacer.replaceAll('12 34', '${b}${a}'), '21 43', 'replaceAll');
eq(new P.Regex(String.raw`\s+`).split('a  b c'), ['a', 'b', 'c'], 'split');

// --- partial matching -----------------------------------------------------

const pRe = new P.Regex(String.raw`token=([a-z]+)([0-9]+)`);
const partial = pRe.findPartial('x token=abc');
assert.ok(partial, 'findPartial result');
assert.ok(partial.isPartial, 'isPartial');
eq(partial.status, 'partial', 'status string');
eq(partial.matched, 'token=abc', 'partial matched');
eq(partial.group(1), 'abc', 'partial group 1 (matched)');
eq(partial.groupState(1), 'matched', 'group 1 state');
eq(partial.groupState(2), 'partial', 'group 2 state');

const full = pRe.findPartial('token=abc123');
assert.ok(full.isFull, 'findPartial full');
eq(full.status, 'full', 'full status');

eq(pRe.findPartial('x token=abc!'), null, 'hard mismatch -> null');

// --- module-level helpers -------------------------------------------------

eq(P.escape('a.b*c'), String.raw`a\.b\*c`, 'escape');
eq(P.isMatch(String.raw`\d+`, 'abc 123'), true, 'isMatch helper');
eq(P.isMatch(String.raw`\d+`, 'no digits'), false, 'isMatch helper false');

// --- introspection --------------------------------------------------------

const namedRe = new P.Regex(String.raw`(?P<year>\d{4})-(?P<month>\d{2})`);
eq(namedRe.groupNames().sort(), ['month', 'year'], 'groupNames');
eq(namedRe.groupIndex('month'), 2, 'groupIndex');
eq(namedRe.groupIndex('nope'), null, 'groupIndex missing');

console.log('OK — all eregex-wasm smoke tests passed.');
