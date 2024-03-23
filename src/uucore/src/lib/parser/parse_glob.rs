// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Parsing a glob Pattern from a string.
//!
//! Use the [`from_str`] function to parse a [`Pattern`] from a string.

// cSpell:words fnmatch

use glob::{Pattern, PatternError};

fn fix_negation(glob: &str) -> String {
    let mut chars = glob.chars().collect::<Vec<_>>();

    let mut i = 0;
    // Add 3 to prevent out of bounds in loop
    while i + 3 < chars.len() {
        if chars[i] == '[' && chars[i + 1] == '^' {
            match chars[i + 3..].iter().position(|x| *x == ']') {
                None => {
                    // if closing square bracket not found, stop looking for it
                    // again
                    break;
                }
                Some(j) => {
                    chars[i + 1] = '!';
                    i += j + 4;
                    continue;
                }
            }
        }

        i += 1;
    }

    chars.into_iter().collect::<String>()
}

/// Parse a glob Pattern from a string.
///
/// This function amends the input string to replace any caret or circumflex
/// character (^) used to negate a set of characters with an exclamation mark
/// (!), which adapts rust's glob matching to function the way the GNU utils'
/// fnmatch does.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use uucore::parse_glob::from_str;
/// assert!(!from_str("[^abc]").unwrap().matches("a"));
/// assert!(from_str("[^abc]").unwrap().matches("x"));
/// ```
pub fn from_str(glob: &str) -> Result<Pattern, PatternError> {
    Pattern::new(&fix_negation(glob))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!(from_str("[^abc]").unwrap(), Pattern::new("[!abc]").unwrap());
    }

    #[test]
    fn test_fix_negation() {
        // Happy/Simple case
        assert_eq!(fix_negation("[^abc]"), "[!abc]");

        // Should fix negations in a long regex
        assert_eq!(fix_negation("foo[abc]  bar[^def]"), "foo[abc]  bar[!def]");

        // Should fix multiple negations in a regex
        assert_eq!(fix_negation("foo[^abc]bar[^def]"), "foo[!abc]bar[!def]");

        // Should fix negation of the single character ]
        assert_eq!(fix_negation("[^]]"), "[!]]");

        // Should fix negation of the single character ^
        assert_eq!(fix_negation("[^^]"), "[!^]");

        // Should fix negation of the space character
        assert_eq!(fix_negation("[^ ]"), "[! ]");

        // Complicated patterns
        assert_eq!(fix_negation("[^][]"), "[!][]");
        assert_eq!(fix_negation("[^[]]"), "[![]]");

        // More complex patterns that should be replaced
        assert_eq!(fix_negation("[[]] [^a]"), "[[]] [!a]");
        assert_eq!(fix_negation("[[] [^a]"), "[[] [!a]");
        assert_eq!(fix_negation("[]] [^a]"), "[]] [!a]");

        // test that we don't look for closing square brackets unnecessarily
        // Verifies issue #5584
        let chars = "^[".repeat(174571);
        assert_eq!(fix_negation(chars.as_str()), chars);
    }

    #[test]
    fn test_fix_negation_should_not_amend() {
        assert_eq!(fix_negation("abc"), "abc");

        // Regex specifically matches either [ or ^
        assert_eq!(fix_negation("[[^]"), "[[^]");

        // Regex that specifically matches either space or ^
        assert_eq!(fix_negation("[ ^]"), "[ ^]");

        // Regex that specifically matches either [, space or ^
        assert_eq!(fix_negation("[[ ^]"), "[[ ^]");
        assert_eq!(fix_negation("[ [^]"), "[ [^]");

        // Invalid globs (according to rust's glob implementation) will remain unamended
        assert_eq!(fix_negation("[^]"), "[^]");
        assert_eq!(fix_negation("[^"), "[^");
        assert_eq!(fix_negation("[][^]"), "[][^]");

        // Issue #4479
        assert_eq!(fix_negation("ààà[^"), "ààà[^");
    }
}
