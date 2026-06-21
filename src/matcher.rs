//! The backtracking matching engine.
//!
//! The engine is a recursive, continuation-passing backtracker over the
//! [`Node`](crate::ast::Node) tree. Each node is asked to match at the current
//! cursor position; on success it invokes a continuation `k` representing "the
//! rest of the pattern". A choice point (alternation, repetition) saves state,
//! tries one option, and restores state before trying the next.
//!
//! This mirrors the structure of mrab-regex's C engine, which is itself a
//! backtracking VM. Performance optimizations (bytecode compilation, memoized
//! prefix search, bitset classes) are roadmap items; correctness and feature
//! parity come first.

use std::rc::Rc;

use crate::ast::{Node, Predef};
use crate::state::State;
use crate::unicode;

/// A continuation: "match the rest of the pattern given the current state".
/// Cheaply cloneable via `Rc`.
pub type Cont<'p> = Rc<dyn Fn(&mut State) -> bool + 'p>;

/// The "everything succeeded" continuation.
fn k_true<'p>() -> Cont<'p> {
    Rc::new(|_| true)
}

/// Try to match `node` at the current cursor, then run `k`.
pub fn m_node<'p>(node: &'p Node, st: &mut State, k: &Cont<'p>) -> bool {
    match node {
        Node::Empty => k(st),
        Node::Lit { ch, ign } => m_lit(*ch, *ign, st, k),
        Node::LitStr { chars, ign } => m_litstr(chars, *ign, st, k),
        Node::Any { dotall } => m_any(*dotall, st, k),
        Node::Class { cc } => m_class(cc, st, k),
        Node::Predef { kind, negated, ascii } => m_predef(*kind, *negated, *ascii, st, k),
        Node::Prop(p) => m_prop(p.pred, p.negated, st, k),
        Node::StartLine { multiline } => m_startline(*multiline, st, k),
        Node::EndLine { multiline } => m_endline(*multiline, st, k),
        Node::StartText => {
            if st.pos == 0 {
                k(st)
            } else {
                false
            }
        }
        Node::EndText => {
            if st.pos == st.len() {
                k(st)
            } else {
                false
            }
        }
        Node::WordBoundary { negated, ascii } => m_word_boundary(*negated, *ascii, st, k),
        Node::WordEdge { end, ascii } => m_word_edge(*end, *ascii, st, k),
        Node::Grapheme => m_grapheme(st, k),
        Node::Group { index, node } => m_group(*index, node, st, k),
        Node::NonCap(node) => m_node(node, st, k),
        Node::Atomic(node) => m_atomic(node, st, k),
        Node::Branch { alts } => m_branch(alts, st, k),
        Node::Sequence { items } => m_seq(items, 0, st, k),
        Node::Repeat { node, min, max, greedy } => m_repeat(node, *min, *max, *greedy, st, k),
        Node::BackRef { group, ign } => m_backref(*group, *ign, st, k),
        Node::Look { behind, positive, node } => m_look(*behind, *positive, node, st, k),
    }
}

/// Match a sequence of nodes starting at `idx`.
fn m_seq<'p>(items: &'p [Node], idx: usize, st: &mut State, k: &Cont<'p>) -> bool {
    if idx >= items.len() {
        return k(st);
    }
    let k = k.clone();
    let next: Cont<'p> = Rc::new(move |st: &mut State| m_seq(items, idx + 1, st, &k));
    m_node(&items[idx], st, &next)
}

fn m_lit<'p>(ch: char, ign: bool, st: &mut State, k: &Cont<'p>) -> bool {
    let matches = match st.cur() {
        Some(c) if !ign => c == ch,
        Some(c) if ign => unicode::case_eq(c, ch),
        _ => false,
    };
    if !matches {
        if st.partial_mode && st.cur().is_none() {
            st.record_partial_block();
        }
        return false;
    }
    st.pos += 1;
    if k(st) {
        true
    } else {
        st.pos -= 1;
        false
    }
}

fn m_litstr<'p>(chars: &[char], ign: bool, st: &mut State, k: &Cont<'p>) -> bool {
    let start = st.pos;
    for &c in chars {
        let ok = match st.cur() {
            Some(cur) if !ign => cur == c,
            Some(cur) if ign => unicode::case_eq(cur, c),
            _ => false,
        };
        if !ok {
            if st.partial_mode && st.cur().is_none() {
                st.record_partial_block();
            }
            st.pos = start;
            return false;
        }
        st.pos += 1;
    }
    if k(st) {
        true
    } else {
        st.pos = start;
        false
    }
}

fn m_any<'p>(dotall: bool, st: &mut State, k: &Cont<'p>) -> bool {
    let ok = match st.cur() {
        Some(c) => dotall || c != '\n',
        None => false,
    };
    if !ok {
        if st.partial_mode && st.cur().is_none() {
            st.record_partial_block();
        }
        return false;
    }
    st.pos += 1;
    if k(st) {
        true
    } else {
        st.pos -= 1;
        false
    }
}

fn m_class<'p>(cc: &crate::ast::CharClass, st: &mut State, k: &Cont<'p>) -> bool {
    let Some(c) = st.cur() else {
        if st.partial_mode {
            st.record_partial_block();
        }
        return false;
    };
    if !cc.matches(c) {
        return false;
    }
    st.pos += 1;
    if k(st) {
        true
    } else {
        st.pos -= 1;
        false
    }
}

fn m_predef<'p>(kind: Predef, negated: bool, ascii: bool, st: &mut State, k: &Cont<'p>) -> bool {
    let Some(c) = st.cur() else {
        if st.partial_mode {
            st.record_partial_block();
        }
        return false;
    };
    let pred = match kind {
        Predef::Digit => unicode::is_digit(c, ascii),
        Predef::Word => unicode::is_word(c, ascii),
        Predef::Space => unicode::is_space(c, ascii),
    };
    if pred == negated {
        return false;
    }
    st.pos += 1;
    if k(st) {
        true
    } else {
        st.pos -= 1;
        false
    }
}

fn m_prop<'p>(pred: unicode::PropFn, negated: bool, st: &mut State, k: &Cont<'p>) -> bool {
    let Some(c) = st.cur() else {
        if st.partial_mode {
            st.record_partial_block();
        }
        return false;
    };
    if pred(c) == negated {
        return false;
    }
    st.pos += 1;
    if k(st) {
        true
    } else {
        st.pos -= 1;
        false
    }
}

fn m_startline<'p>(multiline: bool, st: &mut State, k: &Cont<'p>) -> bool {
    let at_start_of_text = st.pos == 0;
    let after_newline = multiline && matches!(st.prev(), Some('\n'));
    if at_start_of_text || after_newline {
        k(st)
    } else {
        false
    }
}

fn m_endline<'p>(multiline: bool, st: &mut State, k: &Cont<'p>) -> bool {
    let at_end = st.pos == st.len();
    let before_trailing_newline =
        matches!(st.cur(), Some('\n')) && st.pos + 1 == st.len();
    let before_any_newline = multiline && matches!(st.cur(), Some('\n'));
    if at_end || before_trailing_newline || before_any_newline {
        k(st)
    } else {
        false
    }
}

fn m_word_boundary<'p>(negated: bool, ascii: bool, st: &mut State, k: &Cont<'p>) -> bool {
    let before = st.prev().map(|c| unicode::is_word(c, ascii)).unwrap_or(false);
    let after = st.cur().map(|c| unicode::is_word(c, ascii)).unwrap_or(false);
    let boundary = before != after;
    if boundary != negated {
        k(st)
    } else {
        false
    }
}

/// `\m` requires a word char ahead and a non-word char (or start) behind;
/// `\M` requires the reverse.
fn m_word_edge<'p>(end: bool, ascii: bool, st: &mut State, k: &Cont<'p>) -> bool {
    let before = st.prev().map(|c| unicode::is_word(c, ascii)).unwrap_or(false);
    let after = st.cur().map(|c| unicode::is_word(c, ascii)).unwrap_or(false);
    let ok = if end {
        before && !after
    } else {
        !before && after
    };
    if ok {
        k(st)
    } else {
        false
    }
}

/// `\X` — approximate grapheme match: consume exactly one char. (Full
/// UAX #29 grapheme clustering — combining marks, ZWJ sequences, flag emoji —
/// is a roadmap item.)
fn m_grapheme<'p>(st: &mut State, k: &Cont<'p>) -> bool {
    if st.cur().is_none() {
        if st.partial_mode {
            st.record_partial_block();
        }
        return false;
    }
    st.pos += 1;
    if k(st) {
        true
    } else {
        st.pos -= 1;
        false
    }
}

fn m_group<'p>(index: usize, node: &'p Node, st: &mut State, k: &Cont<'p>) -> bool {
    let start = st.pos;
    st.open_groups.push((index, start));
    let idx = index;
    let kc = k.clone();
    let close: Cont<'p> = Rc::new(move |st: &mut State| {
        // On a successful body completion, drop our entry (it is on top —
        // closes fire innermost-first) before closing, so any later
        // partial-block recorded downstream does not falsely include us.
        if st.open_groups.last().map(|&(i, _)| i == idx).unwrap_or(false) {
            st.open_groups.pop();
        }
        st.close_group(idx, start);
        kc(st)
    });
    let r = m_node(node, st, &close);
    if !r {
        // The whole group attempt failed. Choice points inside the body will
        // have restored `open_groups` via snapshots taken after our push, so
        // our entry is typically still on top here — drop it if so.
        if st.open_groups.last().map(|&(i, _)| i == index).unwrap_or(false) {
            st.open_groups.pop();
        }
    }
    r
}

fn m_atomic<'p>(node: &'p Node, st: &mut State, k: &Cont<'p>) -> bool {
    let snap = st.snapshot();
    // Match the inner pattern with a trivially-succeeding continuation,
    // freezing the *first* way it matches.
    if !m_node(node, st, &k_true()) {
        st.restore(snap);
        return false;
    }
    // Inner matched; now try the real continuation. If it fails, do NOT
    // backtrack into the inner pattern.
    if k(st) {
        true
    } else {
        st.restore(snap);
        false
    }
}

fn m_branch<'p>(alts: &'p [Node], st: &mut State, k: &Cont<'p>) -> bool {
    let snap = st.snapshot();
    for alt in alts {
        st.restore(snap.clone());
        if m_node(alt, st, k) {
            return true;
        }
    }
    // All alternatives failed; reset defensively before unwinding.
    st.restore(snap);
    false
}

fn m_repeat<'p>(
    node: &'p Node,
    min: usize,
    max: Option<usize>,
    greedy: bool,
    st: &mut State,
    k: &Cont<'p>,
) -> bool {
    m_rep(node, min, max, greedy, 0, st, k)
}

/// Recursion core of [`m_repeat`].
///
/// `count` is how many iterations have already completed. Zero-width bodies
/// are guarded against infinite loops: the continuation passed to the body
/// checks whether the cursor advanced; if not, it runs the outer continuation
/// `k` directly rather than recursing into another iteration.
#[allow(clippy::too_many_arguments)]
fn m_rep<'p>(
    node: &'p Node,
    min: usize,
    max: Option<usize>,
    greedy: bool,
    count: usize,
    st: &mut State,
    k: &Cont<'p>,
) -> bool {
    let iter_start = st.pos;
    let can_more = match max {
        Some(m) => count < m,
        None => true,
    };

    // Build the continuation that runs after one body iteration.
    let kc = k.clone();
    let after_body: Cont<'p> = Rc::new(move |st: &mut State| {
        if st.pos == iter_start {
            // Body matched empty: repeating further cannot change anything,
            // so jump straight to the rest of the pattern.
            kc(st)
        } else {
            m_rep(node, min, max, greedy, count + 1, st, &kc)
        }
    });

    if greedy {
        // Prefer matching one more iteration.
        if can_more {
            let snap = st.snapshot();
            if m_node(node, st, &after_body) {
                return true;
            }
            st.restore(snap);
        }
        if count >= min {
            return k(st);
        }
        false
    } else {
        // Lazy: prefer to stop (if minimum satisfied).
        if count >= min {
            let snap = st.snapshot();
            if k(st) {
                return true;
            }
            st.restore(snap);
        }
        if can_more {
            let snap = st.snapshot();
            if m_node(node, st, &after_body) {
                return true;
            }
            st.restore(snap);
        }
        false
    }
}

fn m_backref<'p>(group: usize, ign: bool, st: &mut State, k: &Cont<'p>) -> bool {
    let Some(Some((s, e))) = st.caps.get(group).copied() else {
        // Unset group: an empty backreference, always matches empty.
        return k(st);
    };
    let start = st.pos;
    for i in s..e {
        let want = st.chars[i];
        let ok = match st.cur() {
            Some(cur) if !ign => cur == want,
            Some(cur) if ign => unicode::case_eq(cur, want),
            _ => false,
        };
        if !ok {
            if st.partial_mode && st.cur().is_none() {
                st.record_partial_block();
            }
            st.pos = start;
            return false;
        }
        st.pos += 1;
    }
    if k(st) {
        true
    } else {
        st.pos = start;
        false
    }
}

fn m_look<'p>(behind: bool, positive: bool, node: &'p Node, st: &mut State, k: &Cont<'p>) -> bool {
    let snap = st.snapshot();
    let matched = if behind {
        lookbehind_matches(node, st)
    } else {
        let saved = st.pos;
        let ok = m_node(node, st, &k_true());
        st.pos = saved; // never consume on lookahead
        ok
    };
    if matched == positive {
        if k(st) {
            true
        } else {
            // The rest of the pattern failed: roll back any captures the
            // assertion produced.
            st.restore(snap);
            false
        }
    } else {
        st.restore(snap);
        false
    }
}

/// Does `node` match ending *exactly* at the current cursor position
/// (variable-length lookbehind)?
fn lookbehind_matches<'p>(node: &'p Node, st: &mut State) -> bool {
    let target = st.pos;
    let snap = st.snapshot();
    // Try every possible start position from farthest back to nearest.
    let mut start = target;
    loop {
        st.restore(snap.clone());
        st.pos = start;
        let end_check: Cont<'p> = Rc::new(move |st: &mut State| st.pos == target);
        if m_node(node, st, &end_check) {
            st.restore(snap);
            return true;
        }
        if start == 0 {
            break;
        }
        start -= 1;
    }
    st.restore(snap);
    false
}

/// Drive a top-level match of `node` at the current cursor with a succeeding
/// continuation. On success `st.pos` and `st.caps` are set.
pub fn try_match(node: &Node, st: &mut State) -> bool {
    m_node(node, st, &k_true())
}

/// Like [`try_match`], but the match must end exactly at `end_pos` (a char
/// index). Used to implement `fullmatch` and end-constrained searches: lazy
/// quantifiers are forced to keep extending until the cursor reaches the
/// required position.
pub fn try_match_to(node: &Node, st: &mut State, end_pos: usize) -> bool {
    let end_check: Cont<'_> = Rc::new(move |s: &mut State| s.pos == end_pos);
    m_node(node, st, &end_check)
}
