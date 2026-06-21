//! Recursive-descent parser turning a pattern string into a resolved
//! [`Node`](crate::ast::Node) tree.
//!
//! All flag-dependent behaviour is resolved here: case-insensitivity, ASCII
//! vs Unicode, multiline, dotall, verbose. The resulting tree is what the
//! matcher runs.

use std::collections::HashMap;

use crate::ast::{CharClass, ClassItem, Node, Predef, Property};
use crate::charset::CharSet;
use crate::error::{Error, ErrorKind, Result};
use crate::flags::{self, Flags};
use crate::unicode;

/// The outcome of parsing: the AST plus group metadata.
pub(crate) struct Parsed {
    pub node: Node,
    pub n_groups: usize,
    pub names: HashMap<String, usize>,
    pub flags: Flags,
}

/// Maximum repetition count, to reject pathological patterns.
const MAX_REPEAT: usize = 65535;

struct Parser {
    chars: Vec<char>,
    /// `byte_off[i]` = byte offset of `chars[i]`.
    byte_off: Vec<usize>,
    /// Current char index.
    pos: usize,
    /// Currently-active scoped flags.
    flags: Flags,
    group_count: usize,
    names: HashMap<String, usize>,
}

impl Parser {
    fn new(pattern: &str) -> Self {
        let mut chars = Vec::new();
        let mut byte_off = Vec::new();
        for (b, c) in pattern.char_indices() {
            chars.push(c);
            byte_off.push(b);
        }
        byte_off.push(pattern.len());
        Parser {
            chars,
            byte_off,
            pos: 0,
            flags: Flags::NONE,
            group_count: 0,
            names: HashMap::new(),
        }
    }

    fn byte_pos(&self) -> usize {
        self.byte_off[self.pos.min(self.byte_off.len() - 1)]
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn err(&self, kind: ErrorKind) -> Error {
        Error::at(kind, self.byte_pos())
    }

    fn err_msg(&self, kind: ErrorKind, pos: usize) -> Error {
        Error::at(kind, pos)
    }

    // -- flag helpers ------------------------------------------------------

    fn ascii(&self) -> bool {
        self.flags.contains(Flags::ASCII)
    }
    fn ign(&self) -> bool {
        self.flags.contains(Flags::IGNORECASE)
    }
    fn multiline(&self) -> bool {
        self.flags.contains(Flags::MULTILINE)
    }
    fn dotall(&self) -> bool {
        self.flags.contains(Flags::DOTALL)
    }

    // -- entry point -------------------------------------------------------

    fn parse(&mut self, mut flags: Flags) -> Result<Node> {
        flags = flags::resolve_defaults(flags);
        // Global flags are informational here; scoped flags drive parsing.
        self.flags = flags.scoped();
        let node = self.parse_alternation()?;
        if self.pos != self.chars.len() {
            // Stray ')' or similar.
            return Err(self.err(ErrorKind::Syntax(format!(
                "unexpected {:?}",
                self.peek().unwrap_or(')')
            ))));
        }
        Ok(node)
    }

    // alternation := sequence ('|' sequence)*
    fn parse_alternation(&mut self) -> Result<Node> {
        let mut alts = vec![self.parse_sequence()?];
        while self.peek() == Some('|') {
            self.bump();
            alts.push(self.parse_sequence()?);
        }
        Ok(if alts.len() == 1 {
            alts.pop().unwrap()
        } else {
            Node::Branch { alts }
        })
    }

    // sequence := (quantified)*
    fn parse_sequence(&mut self) -> Result<Node> {
        let mut items: Vec<Node> = Vec::new();
        loop {
            match self.peek() {
                None => break,
                Some('|') | Some(')') => break,
                Some(c) if self.flags.contains(Flags::VERBOSE) && is_pattern_ws(c) => {
                    self.bump();
                    continue;
                }
                Some('#') if self.flags.contains(Flags::VERBOSE) => {
                    // Line comment: skip to end of line.
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                    continue;
                }
                Some(_) => {
                    let atom = self.parse_quantified()?;
                    items.push(atom);
                }
            }
        }
        Ok(Node::seq(items))
    }

    // quantified := atom quantifier?
    fn parse_quantified(&mut self) -> Result<Node> {
        let atom = self.parse_atom()?;
        if matches!(atom, Node::Empty) {
            // Nothing to quantify (can happen after a bare inline flag group).
            return Ok(atom);
        }
        // Apply quantifiers. A single atom may carry one quantifier; chains
        // like `a**` are invalid.
        if let Some(q) = self.parse_quant()? {
            let node = apply_quant(atom, q)?;
            // Reject `a++`, `a*+?` style chains on an already-quantified node.
            if self.parse_quant()?.is_some() {
                return Err(self.err(ErrorKind::BadRepeat(
                    "multiple quantifiers".into(),
                )));
            }
            Ok(node)
        } else {
            Ok(atom)
        }
    }

    /// Parse an optional quantifier suffix.
    fn parse_quant(&mut self) -> Result<Option<Quant>> {
        match self.peek() {
            Some('*') => {
                self.bump();
                Ok(Some(self.quant_suffix(0, None)))
            }
            Some('+') => {
                self.bump();
                Ok(Some(self.quant_suffix(1, None)))
            }
            Some('?') => {
                self.bump();
                Ok(Some(self.quant_suffix(0, Some(1))))
            }
            Some('{') => {
                // Try to parse `{...}`; if it isn't a well-formed quantifier,
                // treat `{` as a literal.
                let save = self.pos;
                if let Some(q) = self.parse_brace_quant()? {
                    Ok(Some(q))
                } else {
                    self.pos = save;
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// Read the optional `?`/`+` after a quantifier (lazy / possessive).
    fn quant_suffix(&mut self, min: usize, max: Option<usize>) -> Quant {
        match self.peek() {
            Some('?') => {
                self.bump();
                Quant { min, max, greedy: false, possessive: false }
            }
            Some('+') => {
                self.bump();
                Quant { min, max, greedy: true, possessive: true }
            }
            _ => Quant { min, max, greedy: true, possessive: false },
        }
    }

    /// Parse `{m}`, `{m,}`, `{m,n}`, `{,n}`. Returns `Ok(None)` if the text
    /// isn't a valid quantifier (so `{` can be treated as a literal).
    fn parse_brace_quant(&mut self) -> Result<Option<Quant>> {
        // self.peek() == Some('{')
        let start = self.pos;
        self.bump(); // consume '{'
        // Detect fuzzy-matching specifiers like `{e<=1}`, `{i,s}`, `{d<=3}`
        // (mrab-regex approximate matching). We don't implement fuzzy matching
        // yet; reject clearly rather than silently treating as a literal.
        if self.peek().map_or(false, |c| matches!(c, 'e' | 'i' | 'd' | 's')) {
            // Peek ahead for a fuzzy-spec signature: an operator letter
            // followed by `<`, `<=`, `,`, `}`, or another letter.
            let nxt = self.peek_at(1);
            if nxt.map_or(false, |c| matches!(c, '<' | ',' | '}' | '+' | 'i' | 'd' | 's' | 'e')) {
                let pos = self.byte_off[start.min(self.byte_off.len() - 1)];
                return Err(Error::at(
                    ErrorKind::Syntax(
                        "fuzzy matching ({...}) is not yet supported".into(),
                    ),
                    pos,
                ));
            }
        }
        let mut first_str = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                first_str.push(c);
                self.bump();
            } else {
                break;
            }
        }
        let has_comma = self.eat(',');
        let mut second_str = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                second_str.push(c);
                self.bump();
            } else {
                break;
            }
        }
        if self.peek() != Some('}') {
            // Not a quantifier; bail.
            self.pos = start;
            return Ok(None);
        }
        self.bump(); // consume '}'

        let parse_usize = |s: &str| -> Result<usize> {
            s.parse::<usize>().map_err(|_| {
                self.err(ErrorKind::BadRepeat("bad repeat count".into()))
            })
        };

        let (min, max) = if !has_comma {
            // `{m}` exact count. Both first/second should be empty/one value.
            if second_str.is_empty() {
                if first_str.is_empty() {
                    self.pos = start;
                    return Ok(None);
                }
                let m = parse_usize(&first_str)?;
                (m, Some(m))
            } else {
                // Shouldn't happen (no comma), but be safe.
                let m = parse_usize(&second_str)?;
                (m, Some(m))
            }
        } else {
            // `{m,}` or `{m,n}` or `{,n}`.
            let min = if first_str.is_empty() { 0 } else { parse_usize(&first_str)? };
            let max = if second_str.is_empty() {
                None
            } else {
                let m = parse_usize(&second_str)?;
                if m < min {
                    return Err(self.err(ErrorKind::BadRepeat(
                        "min greater than max in {m,n}".into(),
                    )));
                }
                Some(m)
            };
            (min, max)
        };

        if min > MAX_REPEAT || max.map_or(false, |m| m > MAX_REPEAT) {
            return Err(self.err(ErrorKind::TooLarge(format!(
                "repeat count exceeds {MAX_REPEAT}"
            ))));
        }
        Ok(Some(self.quant_suffix(min, max)))
    }

    // atom := group | class | escape | anchor | dot | literal
    fn parse_atom(&mut self) -> Result<Node> {
        let c = match self.peek() {
            Some(c) => c,
            None => return Ok(Node::Empty),
        };
        match c {
            '(' => self.parse_group(),
            '[' => self.parse_class(),
            '\\' => {
                self.bump();
                self.parse_escape(false)
            }
            '.' => {
                self.bump();
                Ok(Node::Any { dotall: self.dotall() })
            }
            '^' => {
                self.bump();
                Ok(Node::StartLine { multiline: self.multiline() })
            }
            '$' => {
                self.bump();
                Ok(Node::EndLine { multiline: self.multiline() })
            }
            '*' | '+' | '?' => Err(self.err(ErrorKind::BadRepeat(format!(
                "nothing to repeat before {c:?}"
            )))),
            ')' => Err(self.err(ErrorKind::Syntax("unbalanced parenthesis".into()))),
            // Ordinary literal.
            _ => {
                self.bump();
                Ok(self.lit_char(c))
            }
        }
    }

    /// Build a literal node, coalescing into LitStr when useful. For
    /// simplicity here we return a single-char `Lit`; the sequence builder
    /// handles runs.
    fn lit_char(&self, c: char) -> Node {
        Node::Lit { ch: c, ign: self.ign() }
    }

    // -- groups ------------------------------------------------------------

    fn parse_group(&mut self) -> Result<Node> {
        let open_pos = self.byte_pos();
        self.bump(); // '('
        // Backtracking-control verbs: (*PRUNE), (*SKIP), (*FAIL)/(*F),
        // (*COMMIT), (*ACCEPT), (*MARK:name). Not yet implemented.
        if self.peek() == Some('*') {
            return Err(Error::at(
                ErrorKind::Syntax(
                    "backtracking verbs (*PRUNE *SKIP *FAIL ...) are not yet supported".into(),
                ),
                open_pos,
            ));
        }
        if self.peek() == Some('?') {
            self.bump();
            self.parse_extension(open_pos)
        } else {
            // Plain capturing group.
            let saved = self.flags;
            self.group_count += 1;
            let idx = self.group_count;
            let body = self.parse_alternation()?;
            self.flags = saved;
            self.expect_close(open_pos)?;
            Ok(Node::Group { index: idx, node: Box::new(body) })
        }
    }

    fn parse_extension(&mut self, open_pos: usize) -> Result<Node> {
        match self.peek() {
            Some(':') => {
                self.bump();
                let saved = self.flags;
                let body = self.parse_alternation()?;
                self.flags = saved;
                self.expect_close(open_pos)?;
                Ok(Node::NonCap(Box::new(body)))
            }
            Some('>') => {
                self.bump();
                let saved = self.flags;
                let body = self.parse_alternation()?;
                self.flags = saved;
                self.expect_close(open_pos)?;
                Ok(Node::Atomic(Box::new(body)))
            }
            Some('=') | Some('!') => {
                let positive = self.bump().unwrap() == '=';
                let saved = self.flags;
                let body = self.parse_alternation()?;
                self.flags = saved;
                self.expect_close(open_pos)?;
                Ok(Node::Look { behind: false, positive, node: Box::new(body) })
            }
            Some('<') => {
                // Could be lookbehind (?<= / (?<! or named group (?<name>
                match self.peek_at(1) {
                    Some('=') | Some('!') => {
                        let positive = self.peek_at(1).unwrap() == '=';
                        self.bump(); // '<'
                        self.bump(); // '=' or '!'
                        let saved = self.flags;
                        let body = self.parse_alternation()?;
                        self.flags = saved;
                        self.expect_close(open_pos)?;
                        Ok(Node::Look { behind: true, positive, node: Box::new(body) })
                    }
                    _ => {
                        // (?<name>...)
                        self.bump(); // '<'
                        self.parse_named_group_body(open_pos, '>')
                    }
                }
            }
            Some('P') => {
                self.bump();
                match self.peek() {
                    Some('<') => {
                        self.bump();
                        self.parse_named_group_body(open_pos, '>')
                    }
                    Some('=') => {
                        // (?P=name)  backreference
                        self.bump();
                        let name = self.read_group_name(')', open_pos)?;
                        let idx = self.lookup_name(&name, open_pos)?;
                        Ok(Node::BackRef { group: idx, ign: self.ign() })
                    }
                    Some('>') | Some('&') => Err(self.err(ErrorKind::Syntax(
                        "recursive subpattern calls (?P>...) are not yet supported".into(),
                    ))),
                    _ => Err(self.err(ErrorKind::Syntax("bad (?P...) extension".into()))),
                }
            }
            Some('(') => {
                Err(self.err(ErrorKind::Syntax(
                    "conditional (?(...)) and (?(DEFINE)...) are not yet supported".into(),
                )))
            }
            Some('#') => {
                // (?#...) comment: skip to ')'.
                self.bump();
                while let Some(c) = self.bump() {
                    if c == ')' {
                        return Ok(Node::Empty);
                    }
                }
                Err(self.err_msg(
                    ErrorKind::Syntax("unterminated (?#...) comment".into()),
                    open_pos,
                ))
            }
            // Flags: (?flags) or (?flags:...) or (?flags-flags:...)
            _ => self.parse_flag_group(open_pos),
        }
    }

    fn parse_named_group_body(&mut self, open_pos: usize, terminator: char) -> Result<Node> {
        let name = self.read_group_name(terminator, open_pos)?;
        // mrab-regex allows the SAME name to be reused by multiple groups
        // (Hg issue 87): all groups with the same name share one group number,
        // and `.captures(name)` returns every capture in order. If the name is
        // new, allocate a fresh group index; if it already exists, reuse it.
        let idx = if let Some(&existing) = self.names.get(&name) {
            existing
        } else {
            self.group_count += 1;
            let idx = self.group_count;
            self.names.insert(name, idx);
            idx
        };
        let saved = self.flags;
        let body = self.parse_alternation()?;
        self.flags = saved;
        self.expect_close(open_pos)?;
        Ok(Node::Group { index: idx, node: Box::new(body) })
    }

    fn read_group_name(&mut self, terminator: char, open_pos: usize) -> Result<String> {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c == terminator {
                break;
            }
            // Allow any non-special char in names (Python is permissive for
            // ASCII identifiers; we accept word chars and a few extras).
            if c.is_alphanumeric() || c == '_' {
                name.push(c);
                self.bump();
            } else {
                return Err(self.err_msg(
                    ErrorKind::Syntax(format!("bad character {c:?} in group name")),
                    open_pos,
                ));
            }
        }
        if name.is_empty() || self.peek() != Some(terminator) {
            return Err(self.err_msg(
                ErrorKind::Syntax("missing group name".into()),
                open_pos,
            ));
        }
        self.bump(); // consume terminator
        Ok(name)
    }

    fn lookup_name(&self, name: &str, pos: usize) -> Result<usize> {
        self.names.get(name).copied().ok_or_else(|| {
            Error::at(ErrorKind::BadGroupRef(format!("unknown group name {name:?}")), pos)
        })
    }

    fn parse_flag_group(&mut self, open_pos: usize) -> Result<Node> {
        let (on, off) = parse_flag_chars(self)?;
        match self.peek() {
            Some(':') => {
                self.bump();
                let saved = self.flags;
                self.flags = (saved | on) & !off;
                let body = self.parse_alternation()?;
                self.flags = saved;
                self.expect_close(open_pos)?;
                Ok(Node::NonCap(Box::new(body)))
            }
            Some(')') => {
                self.bump();
                // Bare (?flags) applies to the rest of the current group.
                if !off.is_empty() {
                    // Turning off global flags is invalid.
                    if (off & Flags::VERSION0.union(Flags::VERSION1)).intersects(Flags::VERSION0) {
                        // tolerated silently
                    }
                }
                self.flags = (self.flags | on) & !off;
                Ok(Node::Empty)
            }
            _ => Err(self.err_msg(
                ErrorKind::BadFlag("expected ':' or ')' after flags".into()),
                open_pos,
            )),
        }
    }

    fn expect_close(&mut self, open_pos: usize) -> Result<()> {
        if self.eat(')') {
            Ok(())
        } else {
            Err(self.err_msg(
                ErrorKind::Syntax("missing closing ')'".into()),
                open_pos,
            ))
        }
    }

    // -- character classes -------------------------------------------------

    fn parse_class(&mut self) -> Result<Node> {
        let open_pos = self.byte_pos();
        self.bump(); // '['
        let mut cc = CharClass::new();
        if self.eat('^') {
            cc.negated = true;
        }
        // A ']' immediately after '[' or '[^' is a literal ']'.
        if self.peek() == Some(']') {
            cc.items.push(ClassItem::Set(CharSet::from_char(']')));
            self.bump();
        }
        while let Some(c) = self.peek() {
            if c == ']' {
                break;
            }
            self.parse_class_member(&mut cc)?;
        }
        if !self.eat(']') {
            return Err(self.err_msg(
                ErrorKind::BadCharClass("unterminated character class".into()),
                open_pos,
            ));
        }
        // Case-insensitivity: expand set members.
        if self.ign() {
            cc.add_case_variants();
        }
        Ok(Node::Class { cc })
    }

    fn parse_class_member(&mut self, cc: &mut CharClass) -> Result<()> {
        let start_pos = self.byte_pos();
        // Nested set operations `&& -- || ~~` and nested `[...]` sets are
        // mrab-regex version-1 features we don't yet implement. Reject them
        // clearly so they can't be silently misparsed as literals.
        if self.detect_set_op() {
            return Err(self.err(ErrorKind::BadCharClass(
                "nested set operations (&& -- || ~~) are not yet supported".into(),
            )));
        }
        let first = match self.peek() {
            Some(c) => c,
            None => {
                return Err(self.err(ErrorKind::BadCharClass(
                    "unterminated character class".into(),
                )))
            }
        };

        // POSIX classes [[:alpha:]] (subset).
        if first == '[' && self.peek_at(1) == Some(':') {
            if let Some(item) = self.try_parse_posix(start_pos)? {
                cc.items.push(item);
                return Ok(());
            }
        }

        // Escapes inside the class.
        let lo = if first == '\\' {
            self.bump();
            match self.parse_escape(true)? {
                Node::Lit { ch, .. } => ch,
                Node::Predef { kind, negated, ascii } => {
                    cc.items.push(ClassItem::Predef { kind, negated, ascii });
                    return Ok(());
                }
                Node::Prop(p) => {
                    cc.items.push(ClassItem::Prop { pred: p.pred, negated: p.negated });
                    return Ok(());
                }
                Node::Class { cc: inner } => {
                    // \Q...\E inside a class produces a Class node; fold it in.
                    cc.items.extend(inner.items);
                    cc.negated = cc.negated ^ inner.negated; // uncommon; keep simple
                    return Ok(());
                }
                other => {
                    return Err(self.err_msg(
                        ErrorKind::BadCharClass(format!("invalid class member {other:?}")),
                        start_pos,
                    ))
                }
            }
        } else {
            self.bump();
            first
        };

        // Range?
        if self.peek() == Some('-') && self.peek_at(1) != Some(']') {
            self.bump(); // '-'
            let hi_first = self.peek();
            let hi = if hi_first == Some('\\') {
                self.bump();
                match self.parse_escape(true)? {
                    Node::Lit { ch, .. } => ch,
                    _ => {
                        return Err(self.err_msg(
                            ErrorKind::BadCharClass("bad range endpoint".into()),
                            start_pos,
                        ))
                    }
                }
            } else if let Some(h) = self.bump() {
                h
            } else {
                return Err(self.err(ErrorKind::BadCharClass(
                    "unterminated character class".into(),
                )));
            };
            if (hi as u32) < (lo as u32) {
                return Err(self.err_msg(
                    ErrorKind::BadCharClass(format!("bad range {lo:?}-{hi:?}")),
                    start_pos,
                ));
            }
            cc.items.push(ClassItem::Set(CharSet::from_range(lo, hi)));
        } else {
            cc.items.push(ClassItem::Set(CharSet::from_char(lo)));
        }
        Ok(())
    }

    /// Look ahead for a set-operation operator (`&& -- || ~~`) starting at the
    /// current position. These are two-char sequences.
    fn detect_set_op(&self) -> bool {
        let a = self.peek();
        let b = self.peek_at(1);
        matches!((a, b),
            (Some('&'), Some('&'))
            | (Some('-'), Some('-'))
            | (Some('|'), Some('|'))
            | (Some('~'), Some('~'))
        )
    }

    /// Attempt to parse a POSIX `[[:name:]]` class. Returns `Ok(Some(item))`
    /// on success, `Ok(None)` if it isn't one.
    fn try_parse_posix(&mut self, start_pos: usize) -> Result<Option<ClassItem>> {
        let save = self.pos;
        self.bump(); // '['
        self.bump(); // ':'
        let neg = self.eat('^');
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c == ':' {
                break;
            }
            name.push(c);
            self.bump();
        }
        if !(self.eat(':') && self.eat(']')) {
            self.pos = save;
            return Ok(None);
        }
        let key = name.to_ascii_lowercase();
        let pred = match key.as_str() {
            "alpha" => |c: char| c.is_alphabetic(),
            "alnum" => |c: char| c.is_alphanumeric(),
            "digit" => |c: char| c.is_digit(10),
            "lower" => |c: char| c.is_lowercase(),
            "upper" => |c: char| c.is_uppercase(),
            "space" => |c: char| c.is_whitespace(),
            "blank" => |c: char| matches!(c, ' ' | '\t'),
            "xdigit" => |c: char| c.is_ascii_hexdigit(),
            "punct" => |c: char| c.is_ascii_punctuation(),
            "cntrl" => |c: char| c.is_control(),
            "print" => |c: char| !c.is_control(),
            "graph" => |c: char| c.is_alphanumeric() || c.is_ascii_punctuation(),
            "ascii" => |c: char| c.is_ascii(),
            "word" => |c: char| c.is_alphanumeric() || c == '_',
            _ => {
                return Err(Error::at(
                    ErrorKind::BadCharClass(format!("unknown POSIX class [:{name}:]")),
                    start_pos,
                ))
            }
        };
        Ok(Some(ClassItem::Prop { pred, negated: neg }))
    }

    // -- escapes -----------------------------------------------------------

    fn parse_escape(&mut self, in_set: bool) -> Result<Node> {
        let start_pos = self.byte_pos();
        let c = match self.bump() {
            Some(c) => c,
            None => {
                return Err(self.err_msg(
                    ErrorKind::BadEscape("trailing backslash".into()),
                    start_pos,
                ))
            }
        };
        match c {
            // Predefined classes.
            'd' => Ok(Node::Predef { kind: Predef::Digit, negated: false, ascii: self.ascii() }),
            'D' => Ok(Node::Predef { kind: Predef::Digit, negated: true, ascii: self.ascii() }),
            'w' => Ok(Node::Predef { kind: Predef::Word, negated: false, ascii: self.ascii() }),
            'W' => Ok(Node::Predef { kind: Predef::Word, negated: true, ascii: self.ascii() }),
            's' => Ok(Node::Predef { kind: Predef::Space, negated: false, ascii: self.ascii() }),
            'S' => Ok(Node::Predef { kind: Predef::Space, negated: true, ascii: self.ascii() }),

            // Anchors (only outside classes; inside a class \b is backspace).
            'b' if !in_set => Ok(Node::WordBoundary { negated: false, ascii: self.ascii() }),
            'B' if !in_set => Ok(Node::WordBoundary { negated: true, ascii: self.ascii() }),
            // \m = start of word, \M = end of word (mrab-regex additions).
            'm' if !in_set => Ok(Node::WordEdge { end: false, ascii: self.ascii() }),
            'M' if !in_set => Ok(Node::WordEdge { end: true, ascii: self.ascii() }),
            // \X = single grapheme cluster (approximated as one char).
            'X' if !in_set => Ok(Node::Grapheme),
            // mrab-regex extensions that we do NOT yet support. Erroring
            // clearly is much safer than silently treating these as literals.
            'K' if !in_set => Err(self.err_msg(
                ErrorKind::Syntax("\\K (keep) is not yet supported".into()),
                start_pos,
            )),
            'G' if !in_set => Err(self.err_msg(
                ErrorKind::Syntax("\\G (search anchor) is not yet supported".into()),
                start_pos,
            )),
            'L' => {
                // \L<name> named-list reference (Hg issue 11) — roadmap.
                Err(self.err_msg(
                    ErrorKind::Syntax("\\L<name> named lists are not yet supported".into()),
                    start_pos,
                ))
            }
            'A' if !in_set => Ok(Node::StartText),
            'Z' | 'z' if !in_set => Ok(Node::EndText),

            'b' if in_set => Ok(self.lit_char('\x08')),
            // Backreference \1..\9 vs octal escape \ooo (outside classes).
            //
            // Rule: if the digit is followed by more octal digits (forming
            // \oo or \ooo), treat as an octal escape; otherwise it's a
            // backreference.
            d @ '1'..='9' if !in_set => {
                let next_is_octal = self.peek().map_or(false, |c| ('0'..='7').contains(&c));
                if next_is_octal {
                    // Octal escape: we already consumed `d`; gather up to 2
                    // more octal digits.
                    let mut v: u32 = d as u32 - '0' as u32;
                    let mut count = 1;
                    while count < 3 {
                        match self.peek() {
                            Some(od @ '0'..='7') => {
                                v = v * 8 + (od as u32 - '0' as u32);
                                self.bump();
                                count += 1;
                            }
                            _ => break,
                        }
                    }
                    let ch = char::from_u32(v).ok_or_else(|| {
                        self.err_msg(
                            ErrorKind::BadEscape("invalid octal escape".into()),
                            start_pos,
                        )
                    })?;
                    Ok(self.lit_char_or_class(ch, in_set))
                } else {
                    let idx = d.to_digit(10).unwrap() as usize;
                    if idx > self.group_count {
                        return Err(self.err_msg(
                            ErrorKind::BadGroupRef(format!("backreference to unknown group {idx}")),
                            start_pos,
                        ));
                    }
                    Ok(Node::BackRef { group: idx, ign: self.ign() })
                }
            }
            // \g<number> or \g<name> or \g'name' — backreference.
            'g' if !in_set => {
                let (name, is_num) = self.parse_g_ref(start_pos)?;
                let idx = if is_num {
                    let n: usize = name.parse().map_err(|_| {
                        self.err_msg(ErrorKind::BadGroupRef("bad \\g<number>".into()), start_pos)
                    })?;
                    if n == 0 || n > self.group_count {
                        return Err(self.err_msg(
                            ErrorKind::BadGroupRef(format!("unknown group {n}")),
                            start_pos,
                        ));
                    }
                    n
                } else {
                    self.lookup_name(&name, start_pos)?
                };
                Ok(Node::BackRef { group: idx, ign: self.ign() })
            }
            // \k<name> backreference (PCRE style).
            'k' if !in_set => {
                let (name, is_num) = self.parse_g_ref(start_pos)?;
                if is_num {
                    return Err(self.err_msg(
                        ErrorKind::BadGroupRef("\\k<> expects a name".into()),
                        start_pos,
                    ));
                }
                let idx = self.lookup_name(&name, start_pos)?;
                Ok(Node::BackRef { group: idx, ign: self.ign() })
            }

            // Unicode properties.
            'p' | 'P' => {
                let positive = c == 'p';
                let name = self.read_property_name(start_pos)?;
                let inner_neg = name.starts_with('^');
                let name = if inner_neg { name[1..].to_string() } else { name };
                // \p{X} matches X, \P{X} matches not-X, and a leading `^`
                // inside the braces flips the sense again.
                let negated = !positive ^ inner_neg;
                let pred = unicode::property(&name).ok_or_else(|| {
                    self.err_msg(ErrorKind::BadProperty(format!("unknown property {name:?}")), start_pos)
                })?;
                Ok(Node::Prop(Property { name, negated, pred }))
            }

            // Hex escapes.
            'x' => {
                let ch = self.parse_hex_escape(start_pos)?;
                Ok(self.lit_char_or_class(ch, in_set))
            }
            'u' => {
                let ch = self.read_fixed_hex(4, start_pos)?;
                Ok(self.lit_char_or_class(ch, in_set))
            }
            'U' => {
                let ch = self.read_fixed_hex(8, start_pos)?;
                Ok(self.lit_char_or_class(ch, in_set))
            }

            // Octal / null.
            '0' => {
                let mut v: u32 = 0;
                let mut count = 0;
                while count < 3 {
                    match self.peek() {
                        Some(d @ '0'..='7') => {
                            v = v * 8 + (d as u32 - '0' as u32);
                            self.bump();
                            count += 1;
                        }
                        _ => break,
                    }
                }
                let ch = char::from_u32(v).ok_or_else(|| {
                    self.err_msg(ErrorKind::BadEscape("invalid octal escape".into()), start_pos)
                })?;
                Ok(self.lit_char_or_class(ch, in_set))
            }

            // Named single char \N{name} — roadmap.
            'N' if !in_set => Err(self.err_msg(
                ErrorKind::BadEscape("\\N{name} is not yet supported".into()),
                start_pos,
            )),

            // Control-letter escapes.
            'n' => Ok(self.lit_char_or_class('\n', in_set)),
            'r' => Ok(self.lit_char_or_class('\r', in_set)),
            't' => Ok(self.lit_char_or_class('\t', in_set)),
            'f' => Ok(self.lit_char_or_class('\x0c', in_set)),
            'v' => Ok(self.lit_char_or_class('\x0b', in_set)),
            'a' => Ok(self.lit_char_or_class('\x07', in_set)),
            'e' => Ok(self.lit_char_or_class('\x1b', in_set)),

            // \Q...\E literal quoting.
            'Q' if !in_set => {
                let mut chars = Vec::new();
                loop {
                    match self.bump() {
                        Some('\\') if self.peek() == Some('E') => {
                            self.bump();
                            break;
                        }
                        Some(ch) => chars.push(ch),
                        None => break, // unterminated \Q runs to end
                    }
                }
                if chars.is_empty() {
                    Ok(Node::Empty)
                } else {
                    Ok(Node::LitStr { chars, ign: self.ign() })
                }
            }

            // Any other escaped char is a literal of that char.
            other => Ok(self.lit_char_or_class(other, in_set)),
        }
    }

    /// Inside a class we still want a single `Lit` char so range logic works.
    fn lit_char_or_class(&self, ch: char, in_set: bool) -> Node {
        // In a class context case-folding is handled by expanding the set at
        // the class level; a plain Lit is fine because the class member logic
        // only reads `ch`.
        let _ = in_set;
        Node::Lit { ch, ign: self.ign() }
    }

    fn parse_g_ref(&mut self, start_pos: usize) -> Result<(String, bool)> {
        let open = self.bump().ok_or_else(|| {
            self.err_msg(ErrorKind::BadGroupRef("expected '<' or '\\'' after \\g".into()), start_pos)
        })?;
        let close = match open {
            '<' => '>',
            '\'' => '\'',
            _ => {
                return Err(self.err_msg(
                    ErrorKind::BadGroupRef("expected '<' or '\\'' after \\g".into()),
                    start_pos,
                ))
            }
        };
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == close {
                break;
            }
            s.push(c);
            self.bump();
        }
        if !self.eat(close) || s.is_empty() {
            return Err(self.err_msg(
                ErrorKind::BadGroupRef("empty or unterminated \\g<>".into()),
                start_pos,
            ));
        }
        let is_num = s.chars().all(|c| c.is_ascii_digit());
        Ok((s, is_num))
    }

    fn read_property_name(&mut self, start_pos: usize) -> Result<String> {
        let close;
        match self.peek() {
            Some('{') => {
                self.bump();
                close = '}';
            }
            Some(':') => {
                self.bump();
                close = ':';
            }
            _ => {
                return Err(self.err_msg(
                    ErrorKind::BadProperty("expected '{' after \\p".into()),
                    start_pos,
                ))
            }
        }
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == close {
                break;
            }
            s.push(c);
            self.bump();
        }
        if !self.eat(close) {
            return Err(self.err_msg(
                ErrorKind::BadProperty("unterminated \\p{...}".into()),
                start_pos,
            ));
        }
        Ok(s)
    }

    /// `\xHH` or `\x{H+}`.
    fn parse_hex_escape(&mut self, start_pos: usize) -> Result<char> {
        if self.eat('{') {
            let mut s = String::new();
            while let Some(c) = self.peek() {
                if c == '}' {
                    break;
                }
                s.push(c);
                self.bump();
            }
            if !self.eat('}') || s.is_empty() {
                return Err(self.err_msg(
                    ErrorKind::BadEscape("bad \\x{...} escape".into()),
                    start_pos,
                ));
            }
            let v = u32::from_str_radix(&s, 16).map_err(|_| {
                self.err_msg(ErrorKind::BadEscape("bad hex digits".into()), start_pos)
            })?;
            char::from_u32(v).ok_or_else(|| {
                self.err_msg(ErrorKind::BadEscape("invalid codepoint".into()), start_pos)
            })
        } else {
            self.read_fixed_hex(2, start_pos)
        }
    }

    fn read_fixed_hex(&mut self, n: usize, start_pos: usize) -> Result<char> {
        let mut s = String::new();
        for _ in 0..n {
            match self.peek() {
                Some(c) if c.is_ascii_hexdigit() => {
                    s.push(c);
                    self.bump();
                }
                _ => break,
            }
        }
        if s.is_empty() {
            return Err(self.err_msg(
                ErrorKind::BadEscape("expected hex digits".into()),
                start_pos,
            ));
        }
        let v = u32::from_str_radix(&s, 16).map_err(|_| {
            self.err_msg(ErrorKind::BadEscape("bad hex digits".into()), start_pos)
        })?;
        char::from_u32(v).ok_or_else(|| {
            self.err_msg(ErrorKind::BadEscape("invalid codepoint".into()), start_pos)
        })
    }
}

/// A parsed quantifier.
struct Quant {
    min: usize,
    max: Option<usize>,
    greedy: bool,
    possessive: bool,
}

fn apply_quant(atom: Node, q: Quant) -> Result<Node> {
    Ok(atom.quantified(q.min, q.max, q.greedy, q.possessive))
}

fn is_pattern_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c')
}

/// Parse a run of flag characters (and an optional `-flags`) for `(?flags)` /
/// `(?flags-flags:...)`.
fn parse_flag_chars(p: &mut Parser) -> Result<(Flags, Flags)> {
    let mut on = Flags::NONE;
    let mut off = Flags::NONE;
    let mut side = 0u8; // 0 = on, 1 = off
    while let Some(c) = p.peek() {
        if c == ':' || c == ')' {
            break;
        }
        if c == '-' {
            side = 1;
            p.bump();
            continue;
        }
        let f = match c {
            'a' => Flags::ASCII,
            'u' => Flags::UNICODE,
            'i' => Flags::IGNORECASE,
            'L' => Flags::LOCALE,
            'm' => Flags::MULTILINE,
            's' => Flags::DOTALL,
            'x' => Flags::VERBOSE,
            'f' => Flags::FULLCASE,
            'w' => Flags::WORD,
            // Global flags in the on-side only.
            'V' => {
                // V0 / V1
                p.bump();
                match p.peek() {
                    Some('0') => {
                        p.bump();
                        on = handle_global(on, Flags::VERSION0, p)?;
                        continue;
                    }
                    Some('1') => {
                        p.bump();
                        on = handle_global(on, Flags::VERSION1, p)?;
                        continue;
                    }
                    _ => {
                        return Err(p.err(ErrorKind::BadFlag("expected V0 or V1".into())));
                    }
                }
            }
            _ => {
                return Err(p.err(ErrorKind::BadFlag(format!("unknown flag {c:?}"))));
            }
        };
        p.bump();
        if side == 0 {
            on |= f;
        } else {
            off |= f;
        }
    }
    Ok((on, off))
}

fn handle_global(on: Flags, g: Flags, _p: &Parser) -> Result<Flags> {
    Ok(on | g)
}

/// Parse a pattern string into a resolved AST plus metadata.
pub(crate) fn parse(pattern: &str, flags: Flags) -> Result<Parsed> {
    let mut p = Parser::new(pattern);
    let node = p.parse(flags)?;
    Ok(Parsed {
        node,
        n_groups: p.group_count,
        names: p.names,
        flags: flags::resolve_defaults(flags),
    })
}
