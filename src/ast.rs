//! The parsed abstract syntax tree of a pattern.
//!
//! `Node`s are fully "resolved" at parse time: all flag-dependent behaviour
//! (case-insensitivity, ASCII vs Unicode, multiline, dotall, …) is baked into
//! the node, so the matcher never has to consult flags while matching.
/// The kind of predefined character class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Predef {
    /// `\d`
    Digit,
    /// `\w`
    Word,
    /// `\s`
    Space,
}

/// A property predicate `\p{...}`.
#[derive(Clone, Debug)]
pub struct Property {
    /// The original (display) name, for diagnostics.
    pub name: String,
    /// Whether the property is negated (`\P{...}` or `\p{^...}`).
    pub negated: bool,
    /// The predicate to run.
    pub pred: crate::unicode::PropFn,
}

/// One member of a user character class `[...]`.
#[derive(Clone, Debug)]
pub enum ClassItem {
    /// A range/set of codepoints.
    Set(crate::charset::CharSet),
    /// A predefined class `\d \w \s` (possibly negated) inside the class.
    Predef {
        /// Which predefined class.
        kind: Predef,
        /// Whether negated.
        negated: bool,
        /// ASCII vs Unicode.
        ascii: bool,
    },
    /// A `\p{...}` property test inside the class.
    Prop {
        /// The predicate.
        pred: crate::unicode::PropFn,
        /// Whether negated.
        negated: bool,
    },
}

impl ClassItem {
    /// Does this member match `c`?
    pub fn matches(&self, c: char) -> bool {
 match self {
            ClassItem::Set(set) => set.contains(c),
            ClassItem::Predef { kind, negated, ascii } => {
                let p = match kind {
                    Predef::Digit => crate::unicode::is_digit(c, *ascii),
                    Predef::Word => crate::unicode::is_word(c, *ascii),
                    Predef::Space => crate::unicode::is_space(c, *ascii),
                };
                p != *negated
            }
            ClassItem::Prop { pred, negated } => pred(c) != *negated,
        }
    }
}

/// A composite character class `[...]`, possibly negated.
#[derive(Clone, Debug, Default)]
pub struct CharClass {
    /// The members; a character matches if *any* member matches.
    pub items: Vec<ClassItem>,
    /// Whether the whole class is negated (`[^...]`).
    pub negated: bool,
}

impl CharClass {
    /// Create an empty (positive) class.
    pub fn new() -> Self {
        CharClass { items: Vec::new(), negated: false }
    }

    /// Does this class match `c`?
    pub fn matches(&self, c: char) -> bool {
        let any = self.items.iter().any(|m| m.matches(c));
        any != self.negated
    }

    /// Expand all `Set` members to include case-equivalents (for
    /// case-insensitive matching). Predef/Prop members are unaffected.
    pub fn add_case_variants(&mut self) {
        for item in &mut self.items {
            if let ClassItem::Set(set) = item {
                set.add_case_variants();
            }
        }
    }
}

/// A node in the parsed pattern tree.
#[derive(Clone, Debug)]
pub enum Node {
    /// Matches the empty string.
    Empty,
    /// A single literal character. `ign` requests case-insensitive matching.
    Lit {
        /// The character to match.
        ch: char,
        /// Whether to match case-insensitively.
        ign: bool,
    },
    /// A run of consecutive literal characters, stored for clarity.
    LitStr {
        /// The characters.
        chars: Vec<char>,
        /// Whether to match case-insensitively.
        ign: bool,
    },
    /// `.` — any character.
    Any {
        /// If true, `.` matches newlines too (`DOTALL`).
        dotall: bool,
    },
    /// A user-defined character class `[...]`, possibly containing a mix of
    /// ranges, predefined classes and properties.
    Class {
        /// The composite class.
        cc: CharClass,
    },
    /// A predefined class `\d \w \s` or their complements.
    Predef {
        /// Which predefined class.
        kind: Predef,
        /// Whether negated (`\D \W \S`).
        negated: bool,
        /// ASCII vs Unicode semantics.
        ascii: bool,
    },
    /// A `\p{...}` property test.
    Prop(Property),
    /// `^`.
    StartLine {
        /// Whether MULTILINE is in effect.
        multiline: bool,
    },
    /// `$`.
    EndLine {
        /// Whether MULTILINE is in effect.
        multiline: bool,
    },
    /// `\A` — start of text.
    StartText,
    /// `\Z` / `\z` — end of text.
    EndText,
    /// `\b` (`negated = false`) or `\B` (`negated = true`).
    WordBoundary {
        /// `\B` if true.
        negated: bool,
        /// ASCII vs Unicode `\w` for boundary computation.
        ascii: bool,
    },
    /// `\m` (start of word) or `\M` (end of word). `end=false` is `\m`.
    WordEdge {
        /// `true` for end-of-word (`\M`), `false` for start-of-word (`\m`).
        end: bool,
        /// ASCII vs Unicode `\w` for boundary computation.
        ascii: bool,
    },
    /// `\X` — a single Unicode grapheme cluster. (Currently: a single char;
    /// full UAX #29 clustering is a roadmap item.)
    Grapheme,
    /// A capturing group.
    Group {
        /// Group index (1-based; group 0 is the whole match).
        index: usize,
        /// Inner pattern.
        node: Box<Node>,
    },
    /// A non-capturing group `(?:...)`.
    NonCap(Box<Node>),
    /// An atomic group `(?>...)`.
    Atomic(Box<Node>),
    /// Alternation `a|b|c`.
    Branch {
        /// The alternatives.
        alts: Vec<Node>,
    },
    /// A sequence (concatenation) of nodes.
    Sequence {
        /// The items.
        items: Vec<Node>,
    },
    /// A quantified repetition.
    Repeat {
        /// The repeated node.
        node: Box<Node>,
        /// Minimum repetitions.
        min: usize,
        /// Maximum repetitions (`None` = unbounded).
        max: Option<usize>,
        /// Greedy (`true`) or lazy (`false`). Possessive quantifiers are
        /// represented as `Atomic(Repeat { greedy: true, .. })`.
        greedy: bool,
    },
    /// A backreference to a capturing group.
    BackRef {
        /// The group index.
        group: usize,
        /// Whether to compare case-insensitively.
        ign: bool,
    },
    /// A lookahead / lookbehind assertion.
    Look {
        /// `true` for lookbehind, `false` for lookahead.
        behind: bool,
        /// `true` for positive (`(?=...)` / `(?<=...)`).
        positive: bool,
        /// The assertion body.
        node: Box<Node>,
    },
}

impl Node {
    /// Convenience constructor for a `Sequence` of the given items, collapsing
    /// nested sequences and dropping `Empty` items.
    pub fn seq(mut items: Vec<Node>) -> Node {
        let mut flat: Vec<Node> = Vec::with_capacity(items.len());
        for it in items.drain(..) {
            match it {
                Node::Empty => {}
                Node::Sequence { items: sub } => flat.extend(sub),
                other => flat.push(other),
            }
        }
        match flat.len() {
            0 => Node::Empty,
            1 => flat.pop().unwrap(),
            _ => Node::Sequence { items: flat },
        }
    }

    /// Apply a quantifier to `self`, returning the new node.
    pub fn quantified(self, min: usize, max: Option<usize>, greedy: bool, possessive: bool) -> Node {
        let repeat = Node::Repeat {
            node: Box::new(self),
            min,
            max,
            greedy,
        };
        if possessive {
            Node::Atomic(Box::new(repeat))
        } else {
            repeat
        }
    }

    /// Pretty-print the tree (debug aid; requires the `trace` feature for the
    /// public surface but is always available internally).
    pub fn dump(&self, f: &mut impl std::fmt::Write, indent: usize) -> std::fmt::Result {
        let pad = "  ".repeat(indent);
        match self {
            Node::Empty => writeln!(f, "{pad}Empty"),
            Node::Lit { ch, ign } => writeln!(f, "{pad}Lit {ch:?} ign={ign}"),
            Node::LitStr { chars, ign } => writeln!(f, "{pad}LitStr {:?} ign={ign}", chars),
            Node::Any { dotall } => writeln!(f, "{pad}Any dotall={dotall}"),
            Node::Class { cc } => {
                writeln!(f, "{pad}Class negated={} ({} items)", cc.negated, cc.items.len())
            }
            Node::Predef { kind, negated, ascii } => {
                writeln!(f, "{pad}Predef {kind:?} neg={negated} ascii={ascii}")
            }
            Node::Prop(p) => writeln!(f, "{pad}Prop {} neg={}", p.name, p.negated),
            Node::StartLine { multiline } => writeln!(f, "{pad}StartLine ml={multiline}"),
            Node::EndLine { multiline } => writeln!(f, "{pad}EndLine ml={multiline}"),
            Node::StartText => writeln!(f, "{pad}StartText"),
            Node::EndText => writeln!(f, "{pad}EndText"),
            Node::WordBoundary { negated, ascii } => {
                writeln!(f, "{pad}WordBoundary neg={negated} ascii={ascii}")
            }
            Node::WordEdge { end, ascii } => {
                writeln!(f, "{pad}WordEdge end={end} ascii={ascii}")
            }
            Node::Grapheme => writeln!(f, "{pad}Grapheme"),
            Node::Group { index, node } => {
                writeln!(f, "{pad}Group {index}")?;
                node.dump(f, indent + 1)
            }
            Node::NonCap(n) => {
                writeln!(f, "{pad}NonCap")?;
                n.dump(f, indent + 1)
            }
            Node::Atomic(n) => {
                writeln!(f, "{pad}Atomic")?;
                n.dump(f, indent + 1)
            }
            Node::Branch { alts } => {
                writeln!(f, "{pad}Branch")?;
                for a in alts {
                    a.dump(f, indent + 1)?;
                }
                Ok(())
            }
            Node::Sequence { items } => {
                writeln!(f, "{pad}Sequence")?;
                for it in items {
                    it.dump(f, indent + 1)?;
                }
                Ok(())
            }
            Node::Repeat { node, min, max, greedy } => {
                writeln!(f, "{pad}Repeat min={min} max={max:?} greedy={greedy}")?;
                node.dump(f, indent + 1)
            }
            Node::BackRef { group, ign } => writeln!(f, "{pad}BackRef {group} ign={ign}"),
            Node::Look { behind, positive, node } => {
                writeln!(f, "{pad}Look behind={behind} positive={positive}")?;
                node.dump(f, indent + 1)
            }
        }
    }
}
