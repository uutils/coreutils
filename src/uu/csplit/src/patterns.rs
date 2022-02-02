// spell-checker:ignore (regex) SKIPTO UPTO ; (vars) ntimes

use crate::csplit_error::CsplitError;
use regex::Regex;

/// The definition of a pattern to match on a line.
#[derive(Debug)]
pub enum Pattern {
    /// Copy the file's content to a split up to, not including, the given line number. The number
    /// of times the pattern is executed is detailed in [`ExecutePattern`].
    UpToLine(usize, ExecutePattern),
    /// Copy the file's content to a split up to, not including, the line matching the regex. The
    /// integer is an offset relative to the matched line of what to include (if positive) or
    /// to exclude (if negative). The number of times the pattern is executed is detailed in
    /// [`ExecutePattern`].
    UpToMatch(Regex, i32, ExecutePattern),
    /// Skip the file's content up to, not including, the line matching the regex. The integer
    /// is an offset relative to the matched line of what to include (if positive) or to exclude
    /// (if negative). The number of times the pattern is executed is detailed in [`ExecutePattern`].
    SkipToMatch(Regex, i32, ExecutePattern),
}

impl ToString for Pattern {
    fn to_string(&self) -> String {
        match self {
            Pattern::UpToLine(n, _) => n.to_string(),
            Pattern::UpToMatch(regex, 0, _) => format!("/{}/", regex.as_str()),
            Pattern::UpToMatch(regex, offset, _) => format!("/{}/{:+}", regex.as_str(), offset),
            Pattern::SkipToMatch(regex, 0, _) => format!("%{}%", regex.as_str()),
            Pattern::SkipToMatch(regex, offset, _) => format!("%{}%{:+}", regex.as_str(), offset),
        }
    }
}

/// The number of times a pattern can be used.
#[derive(Debug)]
pub enum ExecutePattern {
    /// Execute the pattern as many times as possible
    Always,
    /// Execute the pattern a fixed number of times
    Times(usize),
}

impl ExecutePattern {
    pub fn iter(&self) -> ExecutePatternIter {
        match self {
            Self::Times(n) => ExecutePatternIter::new(Some(*n)),
            Self::Always => ExecutePatternIter::new(None),
        }
    }
}

pub struct ExecutePatternIter {
    max: Option<usize>,
    cur: usize,
}

impl ExecutePatternIter {
    fn new(max: Option<usize>) -> Self {
        Self { max, cur: 0 }
    }
}

impl Iterator for ExecutePatternIter {
    type Item = (Option<usize>, usize);

    fn next(&mut self) -> Option<(Option<usize>, usize)> {
        match self.max {
            // iterate until m is reached
            Some(m) => {
                if self.cur == m {
                    None
                } else {
                    self.cur += 1;
                    Some((self.max, self.cur))
                }
            }
            // no limit, just increment a counter
            None => {
                self.cur += 1;
                Some((None, self.cur))
            }
        }
    }
}

/// Parses the definitions of patterns given on the command line into a list of [`Pattern`]s.
///
/// # Errors
///
/// If a pattern is incorrect, a [`CsplitError::InvalidPattern`] error is returned, which may be
/// due to, e.g.,:
/// - an invalid regular expression;
/// - an invalid number for, e.g., the offset.
pub fn get_patterns(args: &[String]) -> Result<Vec<Pattern>, CsplitError> {
    let patterns = extract_patterns(args)?;
    validate_line_numbers(&patterns)?;
    Ok(patterns)
}

fn extract_patterns(args: &[String]) -> Result<Vec<Pattern>, CsplitError> {
    let mut patterns = Vec::with_capacity(args.len());
    let to_match_reg =
        Regex::new(r"^(/(?P<UPTO>.+)/|%(?P<SKIPTO>.+)%)(?P<OFFSET>[\+-]\d+)?$").unwrap();
    let execute_ntimes_reg = Regex::new(r"^\{(?P<TIMES>\d+)|\*\}$").unwrap();
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        // get the number of times a pattern is repeated, which is at least once plus whatever is
        // in the quantifier.
        let execute_ntimes = match iter.peek() {
            None => ExecutePattern::Times(1),
            Some(&next_item) => {
                match execute_ntimes_reg.captures(next_item) {
                    None => ExecutePattern::Times(1),
                    Some(r) => {
                        // skip the next item
                        iter.next();
                        if let Some(times) = r.name("TIMES") {
                            ExecutePattern::Times(times.as_str().parse::<usize>().unwrap() + 1)
                        } else {
                            ExecutePattern::Always
                        }
                    }
                }
            }
        };

        // get the pattern definition
        if let Some(captures) = to_match_reg.captures(arg) {
            let offset = match captures.name("OFFSET") {
                None => 0,
                Some(m) => m.as_str().parse().unwrap(),
            };
            if let Some(up_to_match) = captures.name("UPTO") {
                let pattern = Regex::new(up_to_match.as_str())
                    .map_err(|_| CsplitError::InvalidPattern(arg.to_string()))?;
                patterns.push(Pattern::UpToMatch(pattern, offset, execute_ntimes));
            } else if let Some(skip_to_match) = captures.name("SKIPTO") {
                let pattern = Regex::new(skip_to_match.as_str())
                    .map_err(|_| CsplitError::InvalidPattern(arg.to_string()))?;
                patterns.push(Pattern::SkipToMatch(pattern, offset, execute_ntimes));
            }
        } else if let Ok(line_number) = arg.parse::<usize>() {
            patterns.push(Pattern::UpToLine(line_number, execute_ntimes));
        } else {
            return Err(CsplitError::InvalidPattern(arg.to_string()));
        }
    }
    Ok(patterns)
}

/// Asserts the line numbers are in increasing order, starting at 1.
fn validate_line_numbers(patterns: &[Pattern]) -> Result<(), CsplitError> {
    patterns
        .iter()
        .filter_map(|pattern| match pattern {
            Pattern::UpToLine(line_number, _) => Some(line_number),
            _ => None,
        })
        .try_fold(0, |prev_ln, &current_ln| match (prev_ln, current_ln) {
            // a line number cannot be zero
            (_, 0) => Err(CsplitError::LineNumberIsZero),
            // two consecutive numbers should not be equal
            (n, m) if n == m => {
                show_warning!("line number '{}' is the same as preceding line number", n);
                Ok(n)
            }
            // a number cannot be greater than the one that follows
            (n, m) if n > m => Err(CsplitError::LineNumberSmallerThanPrevious(m, n)),
            (_, m) => Ok(m),
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_pattern() {
        let input = vec!["bad".to_string()];
        assert!(get_patterns(input.as_slice()).is_err());
    }

    #[test]
    fn up_to_line_pattern() {
        let input: Vec<String> = vec!["24", "42", "{*}", "50", "{4}"]
            .into_iter()
            .map(|v| v.to_string())
            .collect();
        let patterns = get_patterns(input.as_slice()).unwrap();
        assert_eq!(patterns.len(), 3);
        match patterns.get(0) {
            Some(Pattern::UpToLine(24, ExecutePattern::Times(1))) => (),
            _ => panic!("expected UpToLine pattern"),
        };
        match patterns.get(1) {
            Some(Pattern::UpToLine(42, ExecutePattern::Always)) => (),
            _ => panic!("expected UpToLine pattern"),
        };
        match patterns.get(2) {
            Some(Pattern::UpToLine(50, ExecutePattern::Times(5))) => (),
            _ => panic!("expected UpToLine pattern"),
        };
    }

    #[test]
    fn up_to_match_pattern() {
        let input: Vec<String> = vec![
            "/test1.*end$/",
            "/test2.*end$/",
            "{*}",
            "/test3.*end$/",
            "{4}",
            "/test4.*end$/+3",
            "/test5.*end$/-3",
        ]
        .into_iter()
        .map(|v| v.to_string())
        .collect();
        let patterns = get_patterns(input.as_slice()).unwrap();
        assert_eq!(patterns.len(), 5);
        match patterns.get(0) {
            Some(Pattern::UpToMatch(reg, 0, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test1.*end$");
            }
            _ => panic!("expected UpToMatch pattern"),
        };
        match patterns.get(1) {
            Some(Pattern::UpToMatch(reg, 0, ExecutePattern::Always)) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test2.*end$");
            }
            _ => panic!("expected UpToMatch pattern"),
        };
        match patterns.get(2) {
            Some(Pattern::UpToMatch(reg, 0, ExecutePattern::Times(5))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test3.*end$");
            }
            _ => panic!("expected UpToMatch pattern"),
        };
        match patterns.get(3) {
            Some(Pattern::UpToMatch(reg, 3, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test4.*end$");
            }
            _ => panic!("expected UpToMatch pattern"),
        };
        match patterns.get(4) {
            Some(Pattern::UpToMatch(reg, -3, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test5.*end$");
            }
            _ => panic!("expected UpToMatch pattern"),
        };
    }

    #[test]
    fn skip_to_match_pattern() {
        let input: Vec<String> = vec![
            "%test1.*end$%",
            "%test2.*end$%",
            "{*}",
            "%test3.*end$%",
            "{4}",
            "%test4.*end$%+3",
            "%test5.*end$%-3",
        ]
        .into_iter()
        .map(|v| v.to_string())
        .collect();
        let patterns = get_patterns(input.as_slice()).unwrap();
        assert_eq!(patterns.len(), 5);
        match patterns.get(0) {
            Some(Pattern::SkipToMatch(reg, 0, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test1.*end$");
            }
            _ => panic!("expected SkipToMatch pattern"),
        };
        match patterns.get(1) {
            Some(Pattern::SkipToMatch(reg, 0, ExecutePattern::Always)) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test2.*end$");
            }
            _ => panic!("expected SkipToMatch pattern"),
        };
        match patterns.get(2) {
            Some(Pattern::SkipToMatch(reg, 0, ExecutePattern::Times(5))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test3.*end$");
            }
            _ => panic!("expected SkipToMatch pattern"),
        };
        match patterns.get(3) {
            Some(Pattern::SkipToMatch(reg, 3, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test4.*end$");
            }
            _ => panic!("expected SkipToMatch pattern"),
        };
        match patterns.get(4) {
            Some(Pattern::SkipToMatch(reg, -3, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test5.*end$");
            }
            _ => panic!("expected SkipToMatch pattern"),
        };
    }

    #[test]
    fn line_number_zero() {
        let patterns = vec![Pattern::UpToLine(0, ExecutePattern::Times(1))];
        match validate_line_numbers(&patterns) {
            Err(CsplitError::LineNumberIsZero) => (),
            _ => panic!("expected LineNumberIsZero error"),
        }
    }

    #[test]
    fn line_number_smaller_than_previous() {
        let input: Vec<String> = vec!["10".to_string(), "5".to_string()];
        match get_patterns(input.as_slice()) {
            Err(CsplitError::LineNumberSmallerThanPrevious(5, 10)) => (),
            _ => panic!("expected LineNumberSmallerThanPrevious error"),
        }
    }

    #[test]
    fn line_number_smaller_than_previous_separate() {
        let input: Vec<String> = vec!["10".to_string(), "/20/".to_string(), "5".to_string()];
        match get_patterns(input.as_slice()) {
            Err(CsplitError::LineNumberSmallerThanPrevious(5, 10)) => (),
            _ => panic!("expected LineNumberSmallerThanPrevious error"),
        }
    }

    #[test]
    fn line_number_zero_separate() {
        let input: Vec<String> = vec!["10".to_string(), "/20/".to_string(), "0".to_string()];
        match get_patterns(input.as_slice()) {
            Err(CsplitError::LineNumberIsZero) => (),
            _ => panic!("expected LineNumberIsZero error"),
        }
    }
}
