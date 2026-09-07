// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps a [`CsplitError`] onto the operand it came from, so that
//! [`uucore::diagnostics`] can render it with a caret.

use std::ffi::OsString;

use uucore::diagnostics::{Snapshot, ValueOptions};
use uucore::translate;

use crate::csplit_error::CsplitError;

/// The operands a caret may point at, as they were typed.
///
/// The errors name a pattern or a format but not where it was written, so the
/// operands are carried alongside them to the point where the report is drawn.
pub struct Operands<'a> {
    /// The pattern operands, in the order they appear on the line.
    pub patterns: &'a [&'a str],
    /// The value of `-b`/`--suffix-format`, when it was given.
    pub suffix_format: Option<&'a str>,
    /// The value of `-n`/`--digits`, when it was given.
    pub digits: Option<&'a str>,
}

/// The options whose value can be a separate argument, which an operand must
/// not be counted as.
const VALUE_OPTIONS: ValueOptions = ValueOptions {
    shorts: &['b', 'f', 'n'],
    longs: &["suffix-format", "prefix", "digits"],
};

/// Render `err` against `args` — the whole argument list, program name
/// included — where `operands` are the operands as typed.
///
/// Returns `false` when the error cannot be tied to one of them, in which case
/// the caller should fall back to the plain one-line message.
pub fn render(args: &[OsString], operands: &Operands, err: &CsplitError) -> bool {
    let snapshot = Snapshot::with_program(args);
    let index_of_pattern = |pattern: &str| index_of_pattern(args, &snapshot, operands, pattern);
    let message = err.to_string();

    match err {
        CsplitError::InvalidPattern(pattern, problem) => {
            let Some(index) = index_of_pattern(pattern) else {
                return false;
            };
            // Without a problem there is no offset to trust, so the operand is
            // underlined whole.
            let Some(problem) = problem else {
                return snapshot.render(
                    index,
                    &message,
                    None,
                    Some(&translate!("csplit-diag-help-pattern")),
                );
            };
            snapshot.render_inside_at(
                index,
                pattern,
                problem.span.clone(),
                &message,
                problem.label.as_deref(),
                Some(&translate!("csplit-diag-help-pattern")),
            )
        }
        // Neither error says which operand it is about, so the pattern is
        // found by the line number it carries.
        CsplitError::LineNumberIsZero => render_line_number(args, &snapshot, operands, 0, &message),
        CsplitError::LineNumberSmallerThanPrevious(current, _) => {
            render_line_number(args, &snapshot, operands, *current, &message)
        }
        CsplitError::InvalidNumber(_) => {
            let Some(digits) = operands.digits else {
                return false;
            };
            snapshot.render_option_value(
                digits,
                Some('n'),
                Some("digits"),
                0..digits.len(),
                &message,
                None,
                Some(&translate!("csplit-diag-help-digits")),
            )
        }
        // Both messages describe the format as a whole rather than a character
        // of it, so the whole value is underlined.
        CsplitError::SuffixFormatIncorrect | CsplitError::SuffixFormatTooManyPercents => {
            let Some(format) = operands.suffix_format else {
                return false;
            };
            snapshot.render_option_value(
                format,
                Some('b'),
                Some("suffix-format"),
                0..format.len(),
                &message,
                None,
                Some(&translate!("csplit-diag-help-suffix-format")),
            )
        }
        // Raised while reading the input rather than while parsing the line,
        // and named by the pattern as csplit rewrites it rather than as it was
        // typed, so there is nothing to point a caret at.
        _ => false,
    }
}

/// Render `message` against the pattern operand holding the line number
/// `line_number`.
fn render_line_number(
    args: &[OsString],
    snapshot: &Snapshot,
    operands: &Operands,
    line_number: usize,
    message: &str,
) -> bool {
    let Some(pattern) = operands
        .patterns
        .iter()
        .find(|p| p.parse::<usize>() == Ok(line_number))
    else {
        return false;
    };
    let Some(index) = index_of_pattern(args, snapshot, operands, pattern) else {
        return false;
    };
    snapshot.render(
        index,
        message,
        None,
        Some(&translate!("csplit-diag-help-line-number")),
    )
}

/// Where in `args` the pattern operand `pattern` sits.
///
/// The patterns are the operands following FILE. The argument arrived at is
/// checked against the pattern as typed, so a walk thrown off by a spelling
/// [`VALUE_OPTIONS`] does not know draws nothing rather than the wrong thing.
fn index_of_pattern(
    args: &[OsString],
    snapshot: &Snapshot,
    operands: &Operands,
    pattern: &str,
) -> Option<usize> {
    let nth = operands.patterns.iter().position(|p| *p == pattern)?;
    // FILE is the first operand; the patterns follow it.
    let index = snapshot.index_of_operand(nth + 1, &VALUE_OPTIONS)?;
    (args[index].as_encoded_bytes() == pattern.as_bytes()).then_some(index)
}

#[cfg(test)]
mod tests {
    use super::{Operands, VALUE_OPTIONS, index_of_pattern};
    use std::ffi::OsString;
    use uucore::diagnostics::Snapshot;

    /// The index the caret would be drawn at for `pattern`, given a command
    /// line and the patterns clap picked out of it.
    fn index(args: &[&str], patterns: &[&str], pattern: &str) -> Option<usize> {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        let operands = Operands {
            patterns,
            suffix_format: None,
            digits: None,
        };
        index_of_pattern(&args, &Snapshot::with_program(&args), &operands, pattern)
    }

    #[test]
    fn a_separate_option_value_is_not_a_pattern() {
        // Without the option table, the `3` would be taken for the first
        // pattern and the caret drawn on it.
        assert_eq!(
            index(&["csplit", "-n", "3", "file", "5"], &["5"], "5"),
            Some(4)
        );
        assert_eq!(index(&["csplit", "-n3", "file", "5"], &["5"], "5"), Some(3));
        assert_eq!(
            index(&["csplit", "--digits", "3", "file", "5"], &["5"], "5"),
            Some(4)
        );
        assert_eq!(
            index(&["csplit", "--digits=3", "file", "5"], &["5"], "5"),
            Some(3)
        );
        // An abbreviated long name still takes its value.
        assert_eq!(
            index(&["csplit", "--dig", "3", "file", "5"], &["5"], "5"),
            Some(4)
        );
    }

    #[test]
    fn a_flag_takes_nothing_and_a_cluster_ends_in_its_value() {
        assert_eq!(index(&["csplit", "-k", "file", "5"], &["5"], "5"), Some(3));
        assert_eq!(
            index(&["csplit", "-kn", "3", "file", "5"], &["5"], "5"),
            Some(4)
        );
        assert_eq!(
            index(&["csplit", "-kn3", "file", "5"], &["5"], "5"),
            Some(3)
        );
    }

    #[test]
    fn the_patterns_are_counted_in_order_after_the_file() {
        let args = ["csplit", "notes.txt", "3", "1"];
        assert_eq!(index(&args, &["3", "1"], "3"), Some(2));
        assert_eq!(index(&args, &["3", "1"], "1"), Some(3));
    }

    #[test]
    fn a_walk_landing_elsewhere_draws_nothing() {
        // The patterns clap reports do not line up with the line: rather than
        // blame whatever sits there, nothing is drawn.
        assert_eq!(index(&["csplit", "notes.txt"], &["5"], "5"), None);
    }

    #[test]
    fn the_option_table_names_every_value_taking_option() {
        for option in VALUE_OPTIONS.longs {
            assert!(VALUE_OPTIONS.takes_next(&format!("--{option}")));
        }
        for short in VALUE_OPTIONS.shorts {
            assert!(VALUE_OPTIONS.takes_next(&format!("-{short}")));
        }
        assert!(!VALUE_OPTIONS.takes_next("-k"));
    }
}
