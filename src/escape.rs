//! Pattern / literal escaping (mrab-regex's `regex.escape`).
//!
//! The escaping rules are chosen to reproduce the four examples in the
//! mrab-regex README:
//!
//! | `special_only` | `literal_spaces` | `escape("foo bar!?")` |
//! |----------------|------------------|-----------------------|
//! | false          | false            | `foo\ bar\!\?`        |
//! | true           | false            | `foo\ bar!\?`         |
//! | true           | true             | `foo bar!\?`          |

/// Escape `s` so it matches literally as a regex pattern (aggressive mode:
/// regex metacharacters plus spaces and common punctuation are escaped).
///
/// ```
/// assert_eq!(pregex::escape("a.b*c"), r"a\.b\*c");
/// ```
pub fn escape(s: &str) -> String {
    escape_impl(s, false, false)
}

/// Like [`escape`] but only escapes regex "special" characters, leaving
/// non-special punctuation alone (mrab's `special_only=True`).
///
/// ```
/// assert_eq!(pregex::escape_special_only("a.b!"), r"a\.b!");
/// ```
pub fn escape_special_only(s: &str) -> String {
    escape_impl(s, true, false)
}

/// Like [`escape`] but leaves spaces unescaped (mrab's `literal_spaces=True`).
/// This implies `special_only` semantics.
pub fn escape_literal_spaces(s: &str) -> String {
    escape_impl(s, true, true)
}

fn escape_impl(s: &str, special_only: bool, literal_spaces: bool) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        let is_special = matches!(
            c,
            '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '\\'
                | '|'
        );
        let is_space = c == ' ';
        let is_punct = matches!(
            c,
            '!' | '"' | '#' | '%' | '&' | '\'' | ',' | ':' | ';' | '<' | '=' | '>' | '@'
                | '`' | '~' | '/'
        );
        // * regex metacharacters are always escaped;
        // * space is escaped unless `literal_spaces` is true;
        // * other punctuation is escaped unless `special_only` is true.
        let escape_this = is_special
            || (is_space && !literal_spaces)
            || (is_punct && !special_only);
        if escape_this {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
