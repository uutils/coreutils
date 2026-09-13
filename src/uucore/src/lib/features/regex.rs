// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Unified regular expression infrastructure for uutils.
//!
//! Provides a clean abstraction layer around [`fancy_regex`] with support for
//! POSIX Basic Regular Expressions (BRE) transpilation, GNU extensions,
//! and POSIX leftmost-longest matching semantics.

use crate::translate;
pub use fancy_regex::{Regex, RegexBuilder};

/// Errors encountered when compiling or transpiling regular expressions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegexError {
    #[error("{}", translate!("regex-error-unmatched-opening-parenthesis"))]
    UnmatchedOpeningParenthesis,

    #[error("{}", translate!("regex-error-unmatched-closing-parenthesis"))]
    UnmatchedClosingParenthesis,

    #[error("{}", translate!("regex-error-trailing-backslash"))]
    TrailingBackslash,

    #[error("{}", translate!("regex-error-unmatched-opening-brace"))]
    UnmatchedOpeningBrace,

    #[error("{}", translate!("regex-error-invalid-bracket-content"))]
    InvalidBracketContent,

    #[error("{}", translate!("regex-error-too-big-range-quantifier-index"))]
    TooBigRangeQuantifierIndex,

    #[error("{}", translate!("regex-error-invalid-character-class-name"))]
    InvalidCharacterClassName,

    #[error("{}", translate!("regex-error-compilation-failed", "error" => _0))]
    CompilationFailed(String),
}

impl crate::error::UError for RegexError {
    fn code(&self) -> i32 {
        2
    }
}

impl From<fancy_regex::Error> for RegexError {
    fn from(err: fancy_regex::Error) -> Self {
        Self::CompilationFailed(err.to_string())
    }
}

/// Map POSIX character class name (e.g. `"alpha"`, `"digit"`) to its Unicode property equivalent.
pub fn map_posix_class(name: &str) -> Option<&'static str> {
    match name {
        "alpha" => Some(r"\p{Alphabetic}"),
        "lower" => Some(r"\p{Lowercase}"),
        "upper" => Some(r"\p{Uppercase}"),
        "alnum" => Some(r"\p{Alphabetic}0-9"),
        "space" => Some(r"\p{White_Space}"),
        "blank" => Some(r"\t\p{Zs}"),
        "cntrl" => Some(r"\p{Control}"),
        "digit" => Some("0-9"),
        "xdigit" => Some("0-9A-Fa-f"),
        "punct" => Some(r"\p{Punctuation}"),
        "graph" => Some(r"\P{C}&&\P{Z}"),
        "print" => Some(r"\P{C}"),
        _ => None,
    }
}

/// Check if a regex character iterator is at the start of a valid range quantifier (`\{m,n\}`).
///
/// The iterator's start position is expected to be immediately after the opening brace.
fn verify_range_quantifier<I>(pattern_chars: &I) -> Result<(), RegexError>
where
    I: Iterator<Item = char> + Clone,
{
    let mut pattern_chars_clone = pattern_chars.clone().peekable();
    if pattern_chars_clone.peek().is_none() {
        return Err(RegexError::UnmatchedOpeningBrace);
    }

    // Parse the string between braces
    let mut quantifier = String::new();
    let mut prev = '\0';
    let mut curr_is_escaped = false;
    while let Some(curr) = pattern_chars_clone.next() {
        curr_is_escaped = prev == '\\' && !curr_is_escaped;
        if curr_is_escaped && curr == '}' {
            break;
        }
        if pattern_chars_clone.peek().is_none() {
            return Err(RegexError::UnmatchedOpeningBrace);
        }
        if prev != '\0' {
            quantifier.push(prev);
        }
        prev = curr;
    }

    // Check if parsed quantifier is valid
    let re = Regex::new(r"^([0-9]*,[0-9]*|[0-9]+)$").expect("valid regular expression");
    if let Ok(Some(captures)) = re.captures(&quantifier) {
        let matched = captures.get(0).map_or("", |m| m.as_str());
        match matched.split_once(',') {
            Some(("", "")) => Ok(()),
            Some((x, "") | ("", x)) if x.parse::<i16>().is_ok() => Ok(()),
            Some((_, "") | ("", _)) => Err(RegexError::TooBigRangeQuantifierIndex),
            Some((f, l)) => match (f.parse::<i16>(), l.parse::<i16>()) {
                (Ok(f), Ok(l)) if f > l => Err(RegexError::InvalidBracketContent),
                (Ok(_), Ok(_)) => Ok(()),
                _ => Err(RegexError::TooBigRangeQuantifierIndex),
            },
            None if matched.parse::<i16>().is_ok() => Ok(()),
            None => Err(RegexError::TooBigRangeQuantifierIndex),
        }
    } else {
        Err(RegexError::InvalidBracketContent)
    }
}

/// Check for errors in a supplied regular expression
///
/// GNU coreutils shows messages for invalid regular expressions
/// differently from standard regex engines.
/// This method attempts to do these checks manually in one pass
/// through the regular expression.
///
/// This method is not comprehensively checking all cases in which
/// a regular expression could be invalid; any cases not caught will
/// fall through to `fancy-regex` compilation. This method is intended to
/// just identify a few situations for which GNU coreutils has specific
/// error messages.
fn check_posix_regex_errors(pattern: &str) -> Result<(), RegexError> {
    let mut escaped_parens: u64 = 0;
    let mut prev = '\0';
    let mut curr_is_escaped = false;

    for curr in pattern.chars() {
        curr_is_escaped = prev == '\\' && !curr_is_escaped;
        match (curr_is_escaped, curr) {
            (true, '(') => escaped_parens += 1,
            (true, ')') => {
                escaped_parens = escaped_parens
                    .checked_sub(1)
                    .ok_or(RegexError::UnmatchedClosingParenthesis)?;
            }
            _ => {}
        }
        prev = curr;
    }

    match escaped_parens {
        0 => Ok(()),
        _ => Err(RegexError::UnmatchedOpeningParenthesis),
    }
}

/// Check if regex pattern character iterator is at the end of a regex expression or subexpression
fn is_end_of_expression<I>(pattern_chars: &I) -> bool
where
    I: Iterator<Item = char> + Clone,
{
    let mut pattern_chars_clone = pattern_chars.clone();
    match pattern_chars_clone.next() {
        Some('\\') => matches!(pattern_chars_clone.next(), Some(')' | '|')),
        None => true, // No characters left
        _ => false,
    }
}

/// Transpile a POSIX Basic Regular Expression (BRE) into Extended Regular Expression (ERE)
/// compatible with `fancy-regex`.
///
/// If `anchored` is `true`, the pattern is implicitly anchored at the start (`^`), as required by POSIX `expr`.
/// Otherwise, standard unanchored BRE transpilation is performed (e.g. for `grep` and `sed`).
pub fn bre_to_ere(pattern_str: &str, anchored: bool) -> Result<String, RegexError> {
    check_posix_regex_errors(pattern_str)?;

    let mut re_string = String::with_capacity(pattern_str.len() + 8);
    let mut pattern_chars = pattern_str.chars().peekable();
    let mut prev = '\0';
    let mut prev_is_escaped = false;
    let mut in_bracket = false;
    let mut is_start_of_expression = true;
    let mut after_anchor_caret = false;

    if anchored && pattern_chars.peek() != Some(&'^') {
        re_string.push('^');
    }

    while let Some(curr) = pattern_chars.next() {
        let curr_is_escaped = prev == '\\' && !prev_is_escaped;
        let mut next_is_start = false;
        let mut next_after_anchor = false;

        if in_bracket {
            if curr == '[' && pattern_chars.peek() == Some(&':') {
                pattern_chars.next();
                let mut name = String::new();
                let mut closed = false;
                while let Some(c) = pattern_chars.next() {
                    if c == ':' && pattern_chars.peek() == Some(&']') {
                        pattern_chars.next();
                        closed = true;
                        break;
                    }
                    name.push(c);
                }
                if closed {
                    if let Some(unicode_class) = map_posix_class(&name) {
                        re_string.push_str(unicode_class);
                        prev = ']';
                        prev_is_escaped = false;
                        continue;
                    }
                    return Err(RegexError::InvalidCharacterClassName);
                }
                re_string.push_str("[:");
                re_string.push_str(&name);
                prev = name.chars().last().unwrap_or(':');
                prev_is_escaped = false;
                continue;
            }
            if curr == ']' && re_string.ends_with(|c| c != '\\' && c != '[' && c != '^') {
                in_bracket = false;
            }
            re_string.push(curr);
            prev = curr;
            prev_is_escaped = false;
            continue;
        }

        match curr {
            '[' if !curr_is_escaped => {
                in_bracket = true;
                re_string.push('[');
            }
            // In BRE, '(', ')', '|', '+', '?', '{', '}' are literal by default,
            // and become operators only when escaped. ERE has the exact opposite convention.
            '(' | ')' | '|' | '+' | '?' | '{' | '}' => {
                if curr_is_escaped {
                    if re_string.ends_with('\\') {
                        re_string.pop();
                    }
                    match curr {
                        '(' | '|' => {
                            re_string.push(curr);
                            next_is_start = true;
                        }
                        '+' | '?' => {
                            if is_start_of_expression || after_anchor_caret {
                                re_string.push('\\');
                            }
                            re_string.push(curr);
                        }
                        '{' => {
                            // Handle '{' literally at the start of an expression
                            if is_start_of_expression || after_anchor_caret {
                                re_string.push_str(r"\{");
                            } else {
                                // Check if the following section is a valid range quantifier
                                verify_range_quantifier(&pattern_chars)?;
                                re_string.push('{');
                                // Set the lower bound of range quantifier to 0 if it is missing
                                if pattern_chars.peek() == Some(&',') {
                                    re_string.push('0');
                                }
                            }
                        }
                        _ => re_string.push(curr), // ')' and '}'
                    }
                } else {
                    // Unescaped metacharacter in BRE -> literal in ERE
                    re_string.push('\\');
                    re_string.push(curr);
                }
            }
            '*' => {
                if curr_is_escaped {
                    re_string.push('*');
                } else if is_start_of_expression || after_anchor_caret {
                    re_string.push_str(r"\*");
                } else {
                    re_string.push('*');
                }
            }
            // Character class negation "[^a]"
            // Explicitly escaped caret "\^"
            '^' => {
                if curr_is_escaped {
                    re_string.push('^');
                } else if is_start_of_expression {
                    re_string.push('^');
                    next_after_anchor = true;
                } else if prev == '[' && !prev_is_escaped {
                    re_string.push('^');
                } else {
                    re_string.push_str(r"\^");
                }
            }
            '$' if !curr_is_escaped && !is_end_of_expression(&pattern_chars) => {
                re_string.push_str(r"\$");
            }
            '`' if curr_is_escaped => {
                if re_string.ends_with('\\') {
                    re_string.pop();
                }
                re_string.push_str(r"\A");
            }
            '\'' if curr_is_escaped => {
                if re_string.ends_with('\\') {
                    re_string.pop();
                }
                re_string.push_str(r"\z");
            }
            '<' if curr_is_escaped => {
                if re_string.ends_with('\\') {
                    re_string.pop();
                }
                re_string.push_str(r"\b(?=\w)");
            }
            '>' if curr_is_escaped => {
                if re_string.ends_with('\\') {
                    re_string.pop();
                }
                re_string.push_str(r"\b(?<=\w)");
            }
            '\\' if !curr_is_escaped => {
                if pattern_chars.peek().is_none() {
                    return Err(RegexError::TrailingBackslash);
                }
                // Carry the expression-start / after-anchor state over the
                // backslash so the escaped character is still treated as the
                // first token of a (sub)expression.
                next_is_start = is_start_of_expression;
                next_after_anchor = after_anchor_caret;
                re_string.push('\\');
            }
            _ => {
                if curr_is_escaped
                    && !"123456789.*^$[]\\wWsSbB".contains(curr)
                    && re_string.ends_with('\\')
                {
                    re_string.pop();
                }
                re_string.push(curr);
            }
        }

        is_start_of_expression = next_is_start;
        after_anchor_caret = next_after_anchor;
        prev_is_escaped = curr_is_escaped;
        prev = curr;
    }
    Ok(re_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bre_groups() {
        assert_eq!(bre_to_ere(r"foo\(bar\)baz", false).unwrap(), "foo(bar)baz");
    }

    #[test]
    fn test_bre_intervals() {
        assert_eq!(bre_to_ere(r"ab\{1,3\}c", false).unwrap(), "ab{1,3}c");
        assert_eq!(bre_to_ere(r"ab\{,3\}c", false).unwrap(), "ab{0,3}c");
    }

    #[test]
    fn test_bre_alternation() {
        assert_eq!(bre_to_ere(r"foo\|bar", false).unwrap(), "foo|bar");
    }

    #[test]
    fn test_bre_literal_specials() {
        assert_eq!(
            bre_to_ere("a+b?c|d(e)f{g}", false).unwrap(),
            r"a\+b\?c\|d\(e\)f\{g\}"
        );
    }

    #[test]
    fn test_bre_anchors() {
        assert_eq!(bre_to_ere("^foo$", false).unwrap(), "^foo$");
        assert_eq!(bre_to_ere("a^b$c", false).unwrap(), r"a\^b\$c");
    }

    #[test]
    fn test_bre_word_boundaries() {
        assert_eq!(
            bre_to_ere(r"\<word\>", false).unwrap(),
            r"\b(?=\w)word\b(?<=\w)"
        );
    }

    #[test]
    fn test_bre_buffer_anchors() {
        assert_eq!(bre_to_ere(r"\`start", false).unwrap(), r"\Astart");
        assert_eq!(bre_to_ere(r"end\'", false).unwrap(), r"end\z");
    }

    #[test]
    fn test_bre_posix_classes() {
        assert_eq!(
            bre_to_ere(r"[[:alpha:]]", false).unwrap(),
            r"[\p{Alphabetic}]"
        );
        assert_eq!(bre_to_ere(r"[[:digit:]]", false).unwrap(), r"[0-9]");
        assert_eq!(
            bre_to_ere(r"[^[:lower:]]", false).unwrap(),
            r"[^\p{Lowercase}]"
        );
        assert_eq!(
            bre_to_ere(r"[[:alpha:][:digit:]]", false).unwrap(),
            r"[\p{Alphabetic}0-9]"
        );
        assert_eq!(bre_to_ere(r"[]a]", false).unwrap(), "[]a]");
        assert_eq!(bre_to_ere(r"[^]a]", false).unwrap(), "[^]a]");

        // Unicode match test: [[:alpha:]] matches 'é'
        let transpiled = bre_to_ere(r"[[:alpha:]]", true).unwrap();
        let re = Regex::new(&format!("(?s){transpiled}")).unwrap();
        assert!(re.is_match("é").unwrap());
    }

    #[test]
    fn test_bre_invalid_posix_class() {
        assert_eq!(
            bre_to_ere(r"[[:bogus:]]", false).unwrap_err(),
            RegexError::InvalidCharacterClassName
        );
        assert_eq!(
            bre_to_ere(r"[[:123:]]", false).unwrap_err(),
            RegexError::InvalidCharacterClassName
        );
    }

    #[test]
    fn test_bre_backrefs() {
        assert_eq!(bre_to_ere(r"\(foo\)\1", false).unwrap(), r"(foo)\1");
    }

    #[test]
    fn test_bre_leading_quantifier() {
        assert_eq!(bre_to_ere("*foo", false).unwrap(), r"\*foo");
        assert_eq!(bre_to_ere(r"\(*foo\)", false).unwrap(), r"(\*foo)");
        assert_eq!(bre_to_ere(r"a\|*b", false).unwrap(), r"a|\*b");
    }

    #[test]
    fn test_bre_anchored() {
        assert_eq!(bre_to_ere("abc", true).unwrap(), "^abc");
        assert_eq!(bre_to_ere("^abc", true).unwrap(), "^abc");
        assert_eq!(bre_to_ere(r"\(foo\)", true).unwrap(), "^(foo)");
    }

    #[test]
    fn test_posix_errors() {
        assert_eq!(
            bre_to_ere(r"\(foo", false).unwrap_err(),
            RegexError::UnmatchedOpeningParenthesis
        );
        assert_eq!(
            bre_to_ere(r"foo\)", false).unwrap_err(),
            RegexError::UnmatchedClosingParenthesis
        );
        assert_eq!(
            bre_to_ere(r"foo\", false).unwrap_err(),
            RegexError::TrailingBackslash
        );
        assert_eq!(
            bre_to_ere(r"foo\{1", false).unwrap_err(),
            RegexError::UnmatchedOpeningBrace
        );
        assert_eq!(
            bre_to_ere(r"foo\{5,2\}", false).unwrap_err(),
            RegexError::InvalidBracketContent
        );
    }

    #[test]
    fn test_leftmost_longest_semantics() {
        // POSIX leftmost-longest requires (a|ab) against "ab" to match "ab" (longest), not "a".
        let transpiled = bre_to_ere(r"\(a\|ab\)", false).unwrap();
        let re = RegexBuilder::new(&transpiled)
            .oniguruma_mode(true)
            .leftmost_longest(true)
            .seek(true)
            .build()
            .unwrap();

        let caps = re.captures("ab").unwrap().expect("should match");
        assert_eq!(caps.get(1).unwrap().as_str(), "ab");

        // "aaaaa|a*" against "aaaaaa" should match all 6 "a"s
        let transpiled2 = bre_to_ere(r"aaaaa\|a*", false).unwrap();
        let re2 = RegexBuilder::new(&transpiled2)
            .oniguruma_mode(true)
            .leftmost_longest(true)
            .seek(true)
            .build()
            .unwrap();

        let m = re2.find("aaaaaa").unwrap().expect("should match");
        assert_eq!(m.as_str(), "aaaaaa");
    }

    #[test]
    fn check_regex_valid() {
        assert!(check_posix_regex_errors(r"(a+b) \(a* b\)").is_ok());
    }

    #[test]
    fn check_regex_simple_repeating_pattern() {
        assert!(check_posix_regex_errors(r"\(a+b\)\{4\}").is_ok());
    }

    #[test]
    fn check_regex_missing_closing() {
        assert_eq!(
            check_posix_regex_errors(r"\(abc"),
            Err(RegexError::UnmatchedOpeningParenthesis)
        );
    }

    #[test]
    fn check_regex_missing_opening() {
        assert_eq!(
            check_posix_regex_errors(r"abc\)"),
            Err(RegexError::UnmatchedClosingParenthesis)
        );
    }

    #[test]
    fn test_is_valid_range_quantifier() {
        assert!(verify_range_quantifier(&"3\\}".chars()).is_ok());
        assert!(verify_range_quantifier(&"3,\\}".chars()).is_ok());
        assert!(verify_range_quantifier(&",6\\}".chars()).is_ok());
        assert!(verify_range_quantifier(&"3,6\\}".chars()).is_ok());
        assert!(verify_range_quantifier(&",\\}".chars()).is_ok());
        assert!(verify_range_quantifier(&"32767\\}anything".chars()).is_ok());
        assert_eq!(
            verify_range_quantifier(&"\\{3,6\\}".chars()),
            Err(RegexError::InvalidBracketContent)
        );
        assert_eq!(
            verify_range_quantifier(&"\\}".chars()),
            Err(RegexError::InvalidBracketContent)
        );
        assert_eq!(
            verify_range_quantifier(&"".chars()),
            Err(RegexError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&"3".chars()),
            Err(RegexError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&"3,".chars()),
            Err(RegexError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&",6".chars()),
            Err(RegexError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&"3,6".chars()),
            Err(RegexError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&",".chars()),
            Err(RegexError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&"32768\\}".chars()),
            Err(RegexError::TooBigRangeQuantifierIndex)
        );
    }
}
