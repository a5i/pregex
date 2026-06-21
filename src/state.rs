//! Mutable matching state: the haystack (as chars), the cursor, and the
//! capture state with snapshot/restore for backtracking.

/// A capture: `(start, end)` in *char* indices, or `None` if not participating.
pub type Cap = Option<(usize, usize)>;

/// The matching state threaded through the backtracking engine.
pub struct State {
    /// The haystack as UTF-32 chars.
    pub chars: Vec<char>,
    /// `char_to_byte[i]` is the byte offset of `chars[i]` in the original
    /// `&str`. Has length `chars.len() + 1` (the final entry is the string
    /// length).
    pub char_to_byte: Vec<usize>,
    /// Current cursor position (char index).
    pub pos: usize,
    /// The position at which the current top-level search attempt started
    /// (used by `\G` semantics, currently informational).
    pub search_start: usize,
    /// Number of capturing groups (excluding group 0).
    pub n_groups: usize,
    /// Current capture per group; index 0 is the whole match.
    pub caps: Vec<Cap>,
    /// The full history of captures per group (for `.captures()`). Each inner
    /// vector holds the *completed* captures in order.
    pub log: Vec<Vec<(usize, usize)>>,
}

/// A cheap snapshot of the mutable parts of [`State`], used for backtracking.
#[derive(Clone)]
pub struct Snapshot {
    pub pos: usize,
    pub caps: Vec<Cap>,
    pub log_lens: Vec<usize>,
}

impl State {
    /// Build a state over the given haystack chars and a precomputed
    /// char→byte map.
    pub fn new(chars: Vec<char>, char_to_byte: Vec<usize>, n_groups: usize) -> Self {
        State {
            chars,
            char_to_byte,
            pos: 0,
            search_start: 0,
            n_groups,
            caps: vec![None; n_groups + 1],
            log: (0..=n_groups).map(|_| Vec::new()).collect(),
        }
    }

    /// Snapshot the mutable state.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            pos: self.pos,
            caps: self.caps.clone(),
            log_lens: self.log.iter().map(|v| v.len()).collect(),
        }
    }

    /// Restore mutable state from a snapshot.
    pub fn restore(&mut self, s: Snapshot) {
        self.pos = s.pos;
        self.caps = s.caps;
        for (v, len) in self.log.iter_mut().zip(s.log_lens.iter()) {
            v.truncate(*len);
        }
    }

    /// Reset capture state for a fresh search attempt at `start`.
    pub fn reset_for_search(&mut self, start: usize) {
        self.pos = start;
        self.search_start = start;
        for c in self.caps.iter_mut() {
            *c = None;
        }
        for v in self.log.iter_mut() {
            v.clear();
        }
    }

    /// Record that group `idx` (1-based) closed at the current position.
    #[inline]
    pub fn close_group(&mut self, idx: usize, start: usize) {
        let end = self.pos;
        self.caps[idx] = Some((start, end));
        self.log[idx].push((start, end));
    }

    /// The character at `self.pos`, if any.
    #[inline]
    pub fn cur(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// The character immediately before `self.pos`, if any.
    #[inline]
    pub fn prev(&self) -> Option<char> {
        if self.pos == 0 {
            None
        } else {
            self.chars.get(self.pos - 1).copied()
        }
    }

    /// Length of the haystack in chars.
    #[inline]
    pub fn len(&self) -> usize {
        self.chars.len()
    }
}
