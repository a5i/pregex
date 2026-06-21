//! The [`Match`] type and the match iterators.

use std::collections::HashMap;

/// A successful match, carrying the full capture state.
///
/// Group indexing follows the usual convention: index `0` is the whole match,
/// indices `1..` are capturing groups in order of opening parenthesis, and
/// named groups may also be looked up by name.
pub struct Match<'h> {
    pub(crate) haystack: &'h str,
    pub(crate) char_to_byte: Vec<usize>,
    /// Char-index spans per group; `[0]` is the whole match.
    pub(crate) caps: Vec<Option<(usize, usize)>>,
    /// Full capture history per group (for repeated captures).
    pub(crate) log: Vec<Vec<(usize, usize)>>,
    pub(crate) names: HashMap<String, usize>,
}

impl<'h> Match<'h> {
    fn byte_span(&self, g: usize) -> Option<(usize, usize)> {
        let (s, e) = self.caps.get(g).copied().flatten()?;
        Some((self.char_to_byte[s], self.char_to_byte[e]))
    }

    /// The whole match, equivalent to [`group(0)`](Self::group).
    pub fn as_str(&self) -> &'h str {
        self.group(0).unwrap_or("")
    }

    /// Return the text of group `g`, or `None` if it didn't participate.
    pub fn group(&self, g: usize) -> Option<&'h str> {
        let (s, e) = self.byte_span(g)?;
        Some(&self.haystack[s..e])
    }

    /// Return the text of a named group.
    pub fn name(&self, name: &str) -> Option<&'h str> {
        let g = *self.names.get(name)?;
        self.group(g)
    }

    /// The byte offset where the whole match (or group `g`) starts.
    pub fn start(&self) -> usize {
        self.start_of(0)
    }

    /// The byte offset where the whole match (or group `g`) ends.
    pub fn end(&self) -> usize {
        self.end_of(0)
    }

    /// The `(start, end)` byte span of the whole match.
    pub fn span(&self) -> (usize, usize) {
        (self.start(), self.end())
    }

    /// Start byte offset of group `g` (the end of the string if the group
    /// didn't participate, matching Python semantics).
    pub fn start_of(&self, g: usize) -> usize {
        match self.byte_span(g) {
            Some((s, _)) => s,
            None => self.haystack.len(),
        }
    }

    /// End byte offset of group `g`.
    pub fn end_of(&self, g: usize) -> usize {
        match self.byte_span(g) {
            Some((_, e)) => e,
            None => self.haystack.len(),
        }
    }

    /// Span of group `g`.
    pub fn span_of(&self, g: usize) -> (usize, usize) {
        (self.start_of(g), self.end_of(g))
    }

    /// Number of capturing groups plus one (for group 0).
    pub fn len(&self) -> usize {
        self.caps.len()
    }

    /// Always `false`; groups are never empty container-wise.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Iterator over all groups' current text.
    pub fn groups(&self) -> Vec<Option<&'h str>> {
        (0..self.caps.len()).map(|i| self.group(i)).collect()
    }

    /// All captures of group `g` (repeated-capture support, a signature
    /// mrab-regex feature). The last entry equals [`group(g)`](Self::group).
    pub fn captures(&self, g: usize) -> Vec<Option<&'h str>> {
        self.log
            .get(g)
            .map(|v| {
                v.iter()
                    .map(|(s, e)| Some(&self.haystack[self.char_to_byte[*s]..self.char_to_byte[*e]]))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// All captures of a named group.
    pub fn captures_name(&self, name: &str) -> Vec<Option<&'h str>> {
        match self.names.get(name) {
            Some(&g) => self.captures(g),
            None => Vec::new(),
        }
    }

    /// All start byte offsets of group `g`'s repeated captures.
    ///
    /// Mirrors mrab-regex's `Match.starts(group)`.
    pub fn starts(&self, g: usize) -> Vec<usize> {
        self.log
            .get(g)
            .map(|v| v.iter().map(|(s, _)| self.char_to_byte[*s]).collect())
            .unwrap_or_default()
    }

    /// All end byte offsets of group `g`'s repeated captures.
    pub fn ends(&self, g: usize) -> Vec<usize> {
        self.log
            .get(g)
            .map(|v| v.iter().map(|(_, e)| self.char_to_byte[*e]).collect())
            .unwrap_or_default()
    }

    /// All byte spans of group `g`'s repeated captures.
    pub fn spans(&self, g: usize) -> Vec<(usize, usize)> {
        self.log
            .get(g)
            .map(|v| {
                v.iter()
                    .map(|(s, e)| (self.char_to_byte[*s], self.char_to_byte[*e]))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A map from group name to the group's **current** text (a.k.a.
    /// `groupdict` in Python / mrab-regex).
    pub fn named_groups(&self) -> HashMap<String, &'h str> {
        let mut out = HashMap::new();
        for (name, &g) in &self.names {
            if let Some(s) = self.group(g) {
                out.insert(name.clone(), s);
            }
        }
        out
    }

    /// A map from group name to **all** of that group's captures (mrab-regex's
    /// `capturesdict`).
    pub fn captures_dict(&self) -> HashMap<String, Vec<&'h str>> {
        let mut out = HashMap::new();
        for (name, &g) in &self.names {
            let v: Vec<&'h str> = self.captures(g).into_iter().flatten().collect();
            out.insert(name.clone(), v);
        }
        out
    }

    /// All captures of **all** groups (group 0 first), as a list per group.
    /// Mirrors mrab-regex's `allcaptures`.
    pub fn all_captures(&self) -> Vec<Vec<&'h str>> {
        (0..self.caps.len())
            .map(|g| self.captures(g).into_iter().flatten().collect())
            .collect()
    }

    /// All byte spans of all captures of all groups. Mirrors mrab-regex's
    /// `allspans`.
    pub fn all_spans(&self) -> Vec<Vec<(usize, usize)>> {
        (0..self.caps.len()).map(|g| self.spans(g)).collect()
    }

    /// The whole match text (alias of [`as_str`](Self::as_str)).
    pub fn group0(&self) -> &'h str {
        self.as_str()
    }

    /// A tuple-like view of **all** groups' current text — the Rust analogue
    /// of mrab-regex's `m[:]` (which returns a tuple in Python). Index 0 is
    /// the whole match.
    pub fn all_groups(&self) -> Vec<Option<&'h str>> {
        (0..self.caps.len()).map(|i| self.group(i)).collect()
    }
}

impl<'h> std::ops::Index<usize> for Match<'h> {
    type Output = str;
    fn index(&self, i: usize) -> &str {
        self.group(i).unwrap_or("")
    }
}

impl<'h> std::ops::Index<&str> for Match<'h> {
    type Output = str;
    fn index(&self, name: &str) -> &str {
        self.name(name).unwrap_or("")
    }
}

impl<'h> std::fmt::Debug for Match<'h> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (s, e) = self.span();
        write!(
            f,
            "Match {:?} span={}..{}",
            &self.haystack[s..e],
            s,
            e
        )
    }
}

// --- Iterators -------------------------------------------------------------

/// Iterator over non-overlapping matches of a [`Regex`](crate::Regex).
pub struct FindIter<'r, 'h> {
    pub(crate) re: &'r crate::Regex,
    pub(crate) haystack: &'h str,
    pub(crate) st: crate::state::State,
    pub(crate) pos: usize,
    pub(crate) last_end: Option<usize>,
}

impl<'r, 'h> Iterator for FindIter<'r, 'h> {
    type Item = Match<'h>;
    fn next(&mut self) -> Option<Match<'h>> {
        if let Some((start, end)) = self.re.find_from(&mut self.st, self.pos) {
            let m = Match {
                haystack: self.haystack,
                char_to_byte: self.st.char_to_byte.clone(),
                caps: self.st.caps.clone(),
                log: self.st.log.clone(),
                names: self.re.names_clone(),
            };
            // Advance, guarding against zero-width match loops.
            self.pos = if end == start { end + 1 } else { end };
            self.last_end = Some(end);
            Some(m)
        } else {
            None
        }
    }
}

/// Iterator that yields [`Match`] objects with full capture state (an alias of
/// [`FindIter`] in this implementation, since matches always carry captures).
pub type CaptureMatches<'r, 'h> = FindIter<'r, 'h>;
