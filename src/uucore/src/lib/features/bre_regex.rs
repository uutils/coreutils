// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranspileError {
    UnmatchedOpeningBrace,
    InvalidBracketContent,
    TrailingBackslash,
    TooBigRangeQuantifierIndex,
}

fn is_end_of_expression<I>(pattern_chars: &I) -> bool
where
    I: Iterator<Item = char> + Clone,
{
    let mut pattern_chars_clone = pattern_chars.clone();
    match pattern_chars_clone.next() {
        Some('\\') => matches!(pattern_chars_clone.next(), Some(')' | '|')),
        None => true,
        _ => false,
    }
}

fn verify_range_quantifier<I>(pattern_chars: &I) -> Result<(), TranspileError>
where
    I: Iterator<Item = char> + Clone,
{
    let mut pattern_chars_clone = pattern_chars.clone().peekable();
    if pattern_chars_clone.peek().is_none() {
        return Err(TranspileError::UnmatchedOpeningBrace);
    }

    let mut quantifier = String::new();
    let mut prev = '\0';
    let mut curr_is_escaped = false;
    while let Some(curr) = pattern_chars_clone.next() {
        curr_is_escaped = prev == '\\' && !curr_is_escaped;
        if curr_is_escaped && curr == '}' {
            break;
        }
        if pattern_chars_clone.peek().is_none() {
            return Err(TranspileError::UnmatchedOpeningBrace);
        }
        if prev != '\0' {
            quantifier.push(prev);
        }
        prev = curr;
    }

    let valid_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let parse_bound = |s: &str| {
        if valid_digits(s) {
            s.parse::<i16>()
                .map_err(|_| TranspileError::TooBigRangeQuantifierIndex)
        } else {
            Err(TranspileError::InvalidBracketContent)
        }
    };

    match quantifier.split_once(',') {
        Some(("", "")) => Ok(()),
        Some((lower, upper)) if lower.is_empty() => {
            parse_bound(upper)?;
            Ok(())
        }
        Some((lower, upper)) if upper.is_empty() => {
            parse_bound(lower)?;
            Ok(())
        }
        Some((lower, upper)) => {
            let lower = parse_bound(lower)?;
            let upper = parse_bound(upper)?;
            if lower > upper {
                Err(TranspileError::InvalidBracketContent)
            } else {
                Ok(())
            }
        }
        None => {
            parse_bound(&quantifier)?;
            Ok(())
        }
    }
}

/// Convert a POSIX basic regular expression into equivalent extended syntax.
pub fn transpile(pattern_str: &str) -> Result<String, TranspileError> {
    let mut re_string = String::with_capacity(pattern_str.len() + 8);
    let mut pattern_chars = pattern_str.chars().peekable();
    let mut prev = '\0';
    let mut prev_is_escaped = false;
    let mut in_bracket = false;
    let mut bracket_start_idx = 0;
    let mut is_start_of_expression = true;
    let mut after_anchor_caret = false;

    while let Some(curr) = pattern_chars.next() {
        let curr_is_escaped = prev == '\\' && !prev_is_escaped;
        let mut next_is_start = false;
        let mut next_after_anchor = false;

        if in_bracket {
            let is_first = re_string.len() == bracket_start_idx + 1
                || (re_string.ends_with("[^") && re_string.len() == bracket_start_idx + 2);
            if curr == ']' && !curr_is_escaped && !is_first {
                in_bracket = false;
            }
            re_string.push(curr);
        } else {
            match curr {
                '[' if !curr_is_escaped => {
                    in_bracket = true;
                    bracket_start_idx = re_string.len();
                    re_string.push('[');
                }
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
                                if is_start_of_expression || after_anchor_caret {
                                    re_string.push_str(r"\{");
                                } else {
                                    verify_range_quantifier(&pattern_chars)?;
                                    re_string.push('{');
                                    if pattern_chars.peek() == Some(&',') {
                                        re_string.push('0');
                                    }
                                }
                            }
                            _ => re_string.push(curr),
                        }
                    } else {
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
                        return Err(TranspileError::TrailingBackslash);
                    }
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
    use super::{TranspileError, transpile};

    #[test]
    fn transpiles_basic_syntax() {
        assert_eq!(transpile(r"^a\{2\}$"), Ok(String::from("^a{2}$")));
        assert_eq!(transpile(r"^\(a\)\1$"), Ok(String::from("^(a)\\1$")));
        assert_eq!(transpile(r"^a(b)+$"), Ok(String::from("^a\\(b\\)\\+$")));
    }

    #[test]
    fn rejects_invalid_range_quantifiers_and_escapes() {
        assert_eq!(
            transpile(r"a\{3,2\}"),
            Err(TranspileError::InvalidBracketContent)
        );
        assert_eq!(
            transpile(r"a\{3"),
            Err(TranspileError::UnmatchedOpeningBrace)
        );
        assert_eq!(transpile("a\\"), Err(TranspileError::TrailingBackslash));
    }
}
