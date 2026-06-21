//! Mutable matching state: the haystack (as chars), the cursor, and the
//! capture state with snapshot/restore for backtracking.

/// A capture: `(start, end)` in *char* indices, or `None` if not participating.
pub type Cap = Option<(usize, usize)>;

/// A snapshot of progress at the moment a consuming leaf was blocked by
/// end-of-input during a partial-mode match.
///
/// `caps` holds the *completed* capture spans at the block point, and `open`
/// holds the stack of groups whose bodies were entered but not yet closed —
/// the top of `open` is the innermost *partial* group.
#[derive(Clone, Debug)]
pub struct PartialCandidate {
    /// Char position where input ran out (always the haystack length).
    pub end: usize,
    /// Number of completed captures at the block point (tie-break metric).
    pub completed: usize,
    /// Completed capture spans at the block point; index 0 is the whole match.
    pub caps: Vec<Cap>,
    /// Open-group stack at the block point; top is the partial group.
    pub open: Vec<(usize, usize)>,
}

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
    /// Whether partial-match semantics are active: consuming leaves that fail
    /// solely because of end-of-input record a [`PartialCandidate`].
    pub partial_mode: bool,
    /// Best partial candidate seen during the current attempt (max end, then
    /// most completed captures). Reset per search attempt by the caller.
    pub partial_best: Option<PartialCandidate>,
    /// Currently-open capturing groups on the active path: `(index, start)`.
    /// Top of stack = innermost open group. Restored with the rest of the
    /// snapshot at backtracking choice points.
    pub open_groups: Vec<(usize, usize)>,
}

/// A cheap snapshot of the mutable parts of [`State`], used for backtracking.
#[derive(Clone)]
pub struct Snapshot {
    pub pos: usize,
    pub caps: Vec<Cap>,
    pub log_lens: Vec<usize>,
    pub open_groups: Vec<(usize, usize)>,
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
            partial_mode: false,
            partial_best: None,
            open_groups: Vec::new(),
        }
    }

    /// Snapshot the mutable state.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            pos: self.pos,
            caps: self.caps.clone(),
            log_lens: self.log.iter().map(|v| v.len()).collect(),
            open_groups: self.open_groups.clone(),
        }
    }

    /// Restore mutable state from a snapshot.
    pub fn restore(&mut self, s: Snapshot) {
        self.pos = s.pos;
        self.caps = s.caps;
        for (v, len) in self.log.iter_mut().zip(s.log_lens.iter()) {
            v.truncate(*len);
        }
        self.open_groups = s.open_groups;
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
        self.open_groups.clear();
        self.partial_best = None;
    }

    /// Record a partial candidate at the current cursor. Called by consuming
    /// leaves in partial mode when they are blocked by end-of-input. Keeps the
    /// candidate with the greatest `(end, completed)` so that, at equal
    /// depth-in-input, the most-progressed (deepest) attempt wins.
    pub fn record_partial_block(&mut self) {
        let completed = self.caps.iter().filter(|c| c.is_some()).count();
        let cand = PartialCandidate {
            end: self.pos,
            completed,
            caps: self.caps.clone(),
            open: self.open_groups.clone(),
        };
        let better = match &self.partial_best {
            None => true,
            Some(b) => (cand.end, cand.completed) > (b.end, b.completed),
        };
        if better {
            self.partial_best = Some(cand);
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
