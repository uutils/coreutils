// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps a [`CsplitError`] onto the operand it came from, so that
//! [`uucore::diagnostics`] can render it with a caret.

use std::ffi::OsString;

use uucore::diagnostics::Snapshot;
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

/// The short and long spellings of the options whose value takes a separate
/// argument, which a positional operand must not be counted as.
const VALUE_SHORTS: [u8; 3] = [b'b', b'f', b'n'];
const VALUE_LONGS: [&str; 3] = ["suffix-format", "prefix", "digits"];

/// Render `err` against `args` — the whole argument list, program name
/// included — where `operands` are the operands as typed.
///
/// Returns `false` when the error cannot be tied to one of them, in which case
/// the caller should fall back to the plain one-line message.
pub fn render(args: &[OsString], operands: &Operands, err: &CsplitError) -> bool {
    let snapshot = Snapshot::with_program(args);
    let message = err.to_string();

    match err {
        CsplitError::InvalidPattern(pattern, problem) => {
            let Some(index) = index_of_pattern(args, operands, pattern) else {
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
        CsplitError::LineNumberIsZero => render_line_number(&snapshot, args, operands, 0, &message),
        CsplitError::LineNumberSmallerThanPrevious(current, _) => {
            render_line_number(&snapshot, args, operands, *current, &message)
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
    snapshot: &Snapshot,
    args: &[OsString],
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
    let Some(index) = index_of_pattern(args, operands, pattern) else {
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
/// The patterns are positional operands following FILE, but three of csplit's
/// options take a separate value, so a positional cannot simply be counted
/// off: in `csplit -n 3 file 5` the `3` is not one. The list is walked with
/// those three in mind, and the argument arrived at is checked against the
/// pattern as typed, so a walk thrown off by a spelling this does not know
/// draws nothing rather than the wrong thing.
fn index_of_pattern(args: &[OsString], operands: &Operands, pattern: &str) -> Option<usize> {
    let nth = operands.patterns.iter().position(|p| *p == pattern)?;
    // FILE is the first positional; the patterns follow it.
    let index = *positional_indices(args).get(nth + 1)?;
    (args[index].as_encoded_bytes() == pattern.as_bytes()).then_some(index)
}

/// The indices in `args` of the positional operands, program name excluded.
fn positional_indices(args: &[OsString]) -> Vec<usize> {
    let mut positionals = Vec::new();
    let mut options_ended = false;
    let mut expect_value = false;

    for (index, arg) in args.iter().enumerate().skip(1) {
        let bytes = arg.as_encoded_bytes();
        if expect_value {
            expect_value = false;
            continue;
        }
        if options_ended {
            positionals.push(index);
            continue;
        }
        if bytes == b"--" {
            options_ended = true;
        } else if bytes == b"-" || !bytes.starts_with(b"-") {
            positionals.push(index);
        } else if let Some(long) = bytes.strip_prefix(b"--") {
            // `--name=value` carries its own value; `--name` takes the next
            // argument. Long names may be abbreviated, so a prefix counts.
            let name = String::from_utf8_lossy(long);
            expect_value =
                !name.contains('=') && VALUE_LONGS.iter().any(|long| long.starts_with(&*name));
        } else {
            // A cluster of short options: the first that takes a value
            // swallows the rest of the cluster, or the next argument when the
            // cluster ends there.
            let cluster = &bytes[1..];
            if let Some(at) = cluster.iter().position(|c| VALUE_SHORTS.contains(c)) {
                expect_value = at + 1 == cluster.len();
            }
        }
    }
    positionals
}

#[cfg(test)]
mod tests {
    use super::positional_indices;
    use std::ffi::OsString;

    fn indices(args: &[&str]) -> Vec<usize> {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        positional_indices(&args)
    }

    #[test]
    fn a_separate_option_value_is_not_a_positional() {
        assert_eq!(indices(&["csplit", "-n", "3", "file", "5"]), vec![3, 4]);
        assert_eq!(indices(&["csplit", "-n3", "file", "5"]), vec![2, 3]);
        assert_eq!(
            indices(&["csplit", "--digits", "3", "file", "5"]),
            vec![3, 4]
        );
        assert_eq!(indices(&["csplit", "--digits=3", "file", "5"]), vec![2, 3]);
        // An abbreviated long name still takes its value.
        assert_eq!(indices(&["csplit", "--dig", "3", "file", "5"]), vec![3, 4]);
    }

    #[test]
    fn a_flag_takes_nothing_and_a_cluster_ends_in_its_value() {
        assert_eq!(indices(&["csplit", "-k", "file", "5"]), vec![2, 3]);
        assert_eq!(indices(&["csplit", "-kn", "3", "file", "5"]), vec![3, 4]);
        assert_eq!(indices(&["csplit", "-kn3", "file", "5"]), vec![2, 3]);
    }

    #[test]
    fn everything_after_a_double_dash_is_positional() {
        assert_eq!(indices(&["csplit", "--", "-file", "5"]), vec![2, 3]);
        assert_eq!(indices(&["csplit", "-", "5"]), vec![1, 2]);
    }
}
