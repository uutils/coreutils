// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parsing a glob Pattern from a string.
//!
//! Use the [`from_str`] function to parse a [`Pattern`] from a string.

// spell-checker:words fnmatch xdigit cntrl punct corefile

use glob::{Pattern, PatternError};

const UNCLOSED_BRACKET: &str = "unclosed character class";
const UNKNOWN_CLASS: &str = "unknown character class";

/// Returns the members of a POSIX character class such as `digit` in
/// `[[:digit:]]`, as inclusive ranges of the C locale.
fn class_ranges(name: &str) -> Option<&'static [(char, char)]> {
    Some(match name {
        "alnum" => &[('0', '9'), ('A', 'Z'), ('a', 'z')],
        "alpha" => &[('A', 'Z'), ('a', 'z')],
        "blank" => &[(' ', ' '), ('\t', '\t')],
        "cntrl" => &[('\0', '\x1f'), ('\x7f', '\x7f')],
        "digit" => &[('0', '9')],
        "graph" => &[('!', '~')],
        "lower" => &[('a', 'z')],
        "print" => &[(' ', '~')],
        "punct" => &[('!', '/'), (':', '@'), ('[', '`'), ('{', '~')],
        "space" => &[('\t', '\r'), (' ', ' ')],
        "upper" => &[('A', 'Z')],
        "xdigit" => &[('0', '9'), ('A', 'F'), ('a', 'f')],
        _ => return None,
    })
}

/// A parsed bracket expression such as `[!a-z_]`.
struct Bracket {
    negated: bool,
    /// The members, as inclusive ranges.
    ranges: Vec<(char, char)>,
    /// The index just past the closing `]`.
    end: usize,
}

/// Parses the bracket expression whose opening `[` is at `chars[start - 1]`.
fn parse_bracket(chars: &[char], start: usize) -> Result<Bracket, PatternError> {
    let unclosed = || PatternError {
        pos: start - 1,
        msg: UNCLOSED_BRACKET,
    };
    let mut i = start;
    let negated = matches!(chars.get(i), Some('!' | '^'));
    if negated {
        i += 1;
    }
    let first = i;
    let mut ranges = Vec::new();

    loop {
        let Some(&c) = chars.get(i) else {
            return Err(unclosed());
        };
        // A `]` right after the opening `[` (or `[!`) is a member, not the end.
        if c == ']' && i > first {
            return Ok(Bracket {
                negated,
                ranges,
                end: i + 1,
            });
        }

        if c == '[' && chars.get(i + 1) == Some(&':') {
            let rest = &chars[i + 2..];
            if let Some(end) = rest.windows(2).position(|w| w == [':', ']']) {
                let name: String = rest[..end].iter().collect();
                let members = class_ranges(&name).ok_or(PatternError {
                    pos: i,
                    msg: UNKNOWN_CLASS,
                })?;
                ranges.extend_from_slice(members);
                i += end + 4;
                continue;
            }
        }

        let (lo, next) = match c {
            '\\' => (*chars.get(i + 1).ok_or_else(unclosed)?, i + 2),
            _ => (c, i + 1),
        };
        i = next;

        // `a-z` is a range, but a `-` right before the closing `]` is a member.
        if chars.get(i) == Some(&'-') && chars.get(i + 1).is_some_and(|&c| c != ']') {
            let (hi, next) = match chars[i + 1] {
                '\\' => (*chars.get(i + 2).ok_or_else(unclosed)?, i + 3),
                c => (c, i + 2),
            };
            ranges.push((lo, hi));
            i = next;
        } else {
            ranges.push((lo, lo));
        }
    }
}

/// Removes `c` from `ranges`, returning whether any range contained it.
fn take_char(ranges: &mut Vec<(char, char)>, c: char) -> bool {
    let mut found = false;
    let mut out = Vec::with_capacity(ranges.len());
    for &(lo, hi) in ranges.iter() {
        if lo <= c && c <= hi {
            found = true;
            // `]` and `-` are ASCII, so the chars next to them are valid.
            let before = char::from_u32(c as u32 - 1).unwrap();
            let after = char::from_u32(c as u32 + 1).unwrap();
            if lo < c {
                out.push((lo, before));
            }
            if c < hi {
                out.push((after, hi));
            }
        } else {
            out.push((lo, hi));
        }
    }
    *ranges = out;
    found
}

/// Writes a bracket expression in the syntax of [`glob::Pattern`], which
/// ends the set at the first `]` after its first member, reads `-` between
/// two members as a range, and reads `!` as the first member as negation.
fn push_bracket(out: &mut String, negated: bool, mut ranges: Vec<(char, char)>) {
    if let [(lo, hi)] = ranges[..]
        && !negated
        && lo == hi
    {
        out.push_str(&Pattern::escape(&lo.to_string()));
        return;
    }

    let close = take_char(&mut ranges, ']');
    let dash = take_char(&mut ranges, '-');
    // Without a leading `]`, a leading `!` would negate the set.
    if !negated && !close && ranges.first().is_some_and(|&(lo, _)| lo == '!') {
        if let Some(pos) = ranges.iter().position(|&(lo, _)| lo != '!') {
            ranges.swap(0, pos);
        } else {
            take_char(&mut ranges, '!');
            if ranges.is_empty() {
                out.push_str(if dash { "[-!]" } else { "!" });
                return;
            }
            ranges.push(('!', '!'));
        }
    }

    out.push('[');
    if negated {
        out.push('!');
    }
    if close {
        out.push(']');
    }
    for (lo, hi) in ranges {
        out.push(lo);
        if lo != hi {
            out.push('-');
            out.push(hi);
        }
    }
    if dash {
        out.push('-');
    }
    out.push(']');
}

/// Translates a pattern in `fnmatch` syntax, as used by GNU, into the
/// syntax of [`glob::Pattern`].
fn fnmatch_to_glob(glob: &str) -> Result<String, PatternError> {
    let chars = glob.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(glob.len());
    let mut i = 0;

    while let Some(&c) = chars.get(i) {
        match c {
            // `glob::Pattern` rejects `**` unless it is a whole path
            // component; for `fnmatch` it means the same as `*`.
            '*' => {
                if !out.ends_with('*') || out.ends_with("[*]") {
                    out.push('*');
                }
                i += 1;
            }
            '\\' => {
                match chars.get(i + 1) {
                    Some(&escaped) => out.push_str(&Pattern::escape(&escaped.to_string())),
                    None => out.push('\\'),
                }
                i += 2;
            }
            '[' => {
                let bracket = parse_bracket(&chars, i + 1)?;
                push_bracket(&mut out, bracket.negated, bracket.ranges);
                i = bracket.end;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }

    Ok(out)
}

/// Parse a glob Pattern from a string.
///
/// The string uses the syntax of the GNU utils' `fnmatch`: `[^...]` as well
/// as `[!...]` negates a set, sets can contain POSIX character classes such
/// as `[:digit:]` (with their C locale members), and a backslash makes the
/// next character literal.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use uucore::parser::parse_glob::from_str;
/// assert!(!from_str("[^abc]").unwrap().matches("a"));
/// assert!(from_str("[^abc]").unwrap().matches("x"));
/// assert!(from_str("[[:digit:]]*").unwrap().matches("1.log"));
/// assert!(from_str("*\\.txt").unwrap().matches("report.txt"));
/// ```
pub fn from_str(glob: &str) -> Result<Pattern, PatternError> {
    Pattern::new(&fnmatch_to_glob(glob)?)
}

#[cfg(test)]
mod tests {

    use super::*;

    fn translate(glob: &str) -> String {
        fnmatch_to_glob(glob).unwrap()
    }

    fn matches(glob: &str, name: &str) -> bool {
        from_str(glob).unwrap().matches(name)
    }

    #[test]
    fn test_from_str() {
        assert_eq!(from_str("[^abc]").unwrap(), Pattern::new("[!abc]").unwrap());
    }

    #[test]
    fn test_negation() {
        // Happy/Simple case
        assert_eq!(translate("[^abc]"), "[!abc]");

        // Should fix negations in a long regex
        assert_eq!(translate("foo[abc]  bar[^def]"), "foo[abc]  bar[!def]");

        // Should fix multiple negations in a regex
        assert_eq!(translate("foo[^abc]bar[^def]"), "foo[!abc]bar[!def]");

        // Should fix negation of the single character ]
        assert_eq!(translate("[^]]"), "[!]]");

        // Should fix negation of the single character ^
        assert_eq!(translate("[^^]"), "[!^]");

        // Should fix negation of the space character
        assert_eq!(translate("[^ ]"), "[! ]");

        // Complicated patterns
        assert_eq!(translate("[^][]"), "[!][]");
        assert_eq!(translate("[^[]]"), "[![]]");

        // More complex patterns that should be replaced
        assert_eq!(translate("[[]] [^a]"), "[[]] [!a]");
        assert_eq!(translate("[[] [^a]"), "[[] [!a]");
        assert_eq!(translate("[]] [^a]"), "[]] [!a]");

        // test that we don't look for closing square brackets unnecessarily
        // Verifies issue #5584
        let chars = "^[".repeat(174_571);
        assert!(fnmatch_to_glob(chars.as_str()).is_err());
    }

    #[test]
    fn test_should_not_amend() {
        assert_eq!(translate("abc"), "abc");

        // Regex specifically matches either [ or ^
        assert_eq!(translate("[[^]"), "[[^]");

        // Regex that specifically matches either space or ^
        assert_eq!(translate("[ ^]"), "[ ^]");

        // Regex that specifically matches either [, space or ^
        assert_eq!(translate("[[ ^]"), "[[ ^]");
        assert_eq!(translate("[ [^]"), "[ [^]");

        assert_eq!(translate("[][^]"), "[][^]");
    }

    #[test]
    fn test_unclosed_bracket_is_an_error() {
        assert!(from_str("[^]").is_err());
        assert!(from_str("[^").is_err());
        assert!(from_str("READ[ME").is_err());
        // Issue #4479
        assert!(from_str("ààà[^").is_err());
    }

    #[test]
    fn test_character_classes() {
        assert_eq!(translate("[[:digit:]]*"), "[0-9]*");
        assert_eq!(translate("*[![:alnum:]._-]*"), "*[!0-9A-Za-z._-]*");

        assert!(matches("[[:digit:]]*", "1.log"));
        assert!(!matches("[[:digit:]]*", "d]x"));
        assert!(matches("*[[:space:]]*", "x ledger.db"));
        assert!(!matches("*[[:space:]]*", "report.txt"));
        assert!(matches("*[![:alnum:]._-]*", "x ledger.db"));
        assert!(!matches("*[![:alnum:]._-]*", "report.txt"));
        assert!(matches("*[![:alnum:]._-]*", "c._-]z"));
        assert!(matches("[[:upper:][:punct:]]", "]"));
        assert!(matches("[[:upper:][:punct:]]", "-"));
        assert!(matches("[[:upper:][:punct:]]", "!"));
        assert!(!matches("[[:upper:][:punct:]]", "a"));
        assert!(matches("[[:xdigit:]]", "F"));
        assert!(!matches("[[:xdigit:]]", "g"));

        assert!(from_str("[[:nope:]]").is_err());
    }

    #[test]
    fn test_backslash_escapes() {
        assert!(matches("*\\.txt", "report.txt"));
        assert!(matches("lost\\+found", "lost+found"));
        assert!(matches("a\\[1\\].txt", "a[1].txt"));
        assert!(!matches("a\\[1\\].txt", "a\\1.txt"));
        assert!(matches("\\*", "*"));
        assert!(!matches("\\*", "x"));
        assert!(matches("[\\]]", "]"));
        assert!(matches("[a\\-z]", "-"));
        assert!(!matches("[a\\-z]", "m"));
    }

    #[test]
    fn test_special_members() {
        assert!(from_str("[!]").is_err());
        assert!(matches("[\\!]", "!"));
        assert!(!matches("[\\!]", "x"));
        assert!(matches("[\\!a]", "!"));
        assert!(matches("[\\!a]", "a"));
        assert!(!matches("[\\!a]", "b"));
        assert!(matches("[\\!-]", "-"));
        assert!(matches("[\\!-]", "!"));
        assert!(!matches("[\\!-]", "x"));
        // A range that spans `-` and `]`.
        assert!(matches("[+-^]", "-"));
        assert!(matches("[+-^]", "]"));
        assert!(matches("[+-^]", "A"));
        assert!(!matches("[+-^]", "a"));
    }

    #[test]
    fn test_repeated_stars() {
        assert_eq!(translate("core**"), "core*");
        assert!(matches("core**", "core"));
        assert!(matches("core**", "corefile"));
        assert!(matches("\\***", "*x"));
    }
}
