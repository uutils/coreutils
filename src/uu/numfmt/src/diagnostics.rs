// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps an error onto the part of the command line it came from, so that
//! [`uucore::diagnostics`] can render it with a caret: a [`FormatError`], an
//! [`OptionValueError`], or a number that does not convert. A bad `--field`
//! list goes through [`RangeError`], which knows how to render itself.

use std::ffi::{OsStr, OsString};

use uucore::diagnostics::{Snapshot, ValueOptions};
use uucore::ranges::RangeError;
use uucore::translate;

use crate::format::{holds_number, invalid_span};
use crate::options::{FormatError, FormatErrorKind, NumfmtOptions, OptionValueError};
use crate::units::Unit;

/// Render `err`, raised while parsing `format`, against `args` — the whole
/// argument list, program name included.
///
/// Returns `false` when the format cannot be found among the arguments, in
/// which case the caller should fall back to the plain one-line message.
pub fn render(args: &[OsString], format: &str, err: &FormatError) -> bool {
    // Labelled only where a label would add to the message, per the convention
    // in `uucore::diagnostics`.
    let label = match err.kind {
        FormatErrorKind::MissingDirective | FormatErrorKind::UnexpectedCharacter => None,
        FormatErrorKind::UnexpectedConversion => Some("numfmt-diag-label-bad-conversion"),
        FormatErrorKind::NumberOverflow => Some("numfmt-diag-label-number-overflow"),
        FormatErrorKind::StrayPercent => Some("numfmt-diag-label-stray-percent"),
    };

    let snapshot = Snapshot::with_program(args);
    let Some(index) = snapshot.index_of_value(format, None, Some("format")) else {
        return false;
    };
    snapshot.render_inside_at(
        index,
        format,
        err.span.clone(),
        &err.message,
        label.map(|label| translate!(label)).as_deref(),
        Some(&translate!("numfmt-diag-help-format-syntax")),
    )
}

/// The options whose value can be a separate argument. `--header` requires an
/// `=`, so its value never is one.
const VALUE_OPTIONS: ValueOptions = ValueOptions {
    shorts: &['d'],
    longs: &[
        "delimiter",
        "field",
        "format",
        "from",
        "from-unit",
        "invalid",
        "padding",
        "round",
        "suffix",
        "to",
        "to-unit",
        "unit-separator",
    ],
};

/// Render `message`, raised while converting `input` — the `n`-th number given
/// on the command line — against `args`.
///
/// Returns `false` when the number cannot be pointed at, in which case the
/// caller should fall back to the plain one-line message.
pub fn render_input(
    args: &[OsString],
    input: &[u8],
    n: usize,
    message: &str,
    options: &NumfmtOptions,
) -> bool {
    // With fields, the offsets below are of the whole line, not the number.
    if options.delimiter.is_some() {
        return false;
    }
    let Ok(input) = std::str::from_utf8(input) else {
        return false;
    };
    if input.split_whitespace().count() > 1 {
        return false;
    }

    let snapshot = Snapshot::with_program(args);
    let Some(index) = snapshot
        .index_of_operand(n, &VALUE_OPTIONS)
        .or_else(|| snapshot.index_of(OsStr::new(input)))
    else {
        return false;
    };
    let span = invalid_span(input, options);
    // Without --from there is no suffix to spell out, only the option to reach
    // for; an input with no number in it at all is not a suffix question, and
    // neither is one whose leading part only looks like the start of one.
    let help = match options.transform.from {
        _ if !holds_number(input) => None,
        Unit::None => Some("numfmt-diag-help-input-no-from"),
        _ => Some("numfmt-diag-help-input-suffixes"),
    };
    snapshot.render_inside_at(
        index,
        input,
        span,
        message,
        None,
        help.map(|help| translate!(help)).as_deref(),
    )
}

/// Render `err`, an option value that is wrong from end to end, against `args`.
///
/// Returns `false` when no argument carries the value, in which case the caller
/// should fall back to the plain one-line message.
pub fn render_value(args: &[OsString], err: &OptionValueError) -> bool {
    Snapshot::with_program(args).render_option_value(
        &err.value,
        None,
        Some(err.option),
        0..err.value.len(),
        &err.message,
        err.label.map(|label| translate!(label)).as_deref(),
        Some(&translate!(err.help)),
    )
}

/// Render `err`, raised while parsing `fields` — the value of `--field` —
/// against `args`.
///
/// # Returns
///
/// `false` when the list cannot be found among the arguments, in which case
/// the caller should fall back to the plain one-line message.
pub fn render_field(args: &[OsString], fields: &str, err: &RangeError) -> bool {
    err.render_option_value(
        args,
        fields,
        None,
        Some("field"),
        &translate!("numfmt-diag-label-zero-field"),
        &translate!("numfmt-diag-help-field-syntax"),
    )
}

#[cfg(test)]
mod tests {
    use super::VALUE_OPTIONS;
    use std::ffi::OsString;
    use uucore::diagnostics::Snapshot;

    fn args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    /// The walk over `args` with numfmt's own option table.
    fn index_of_operand(args: &[OsString], n: usize) -> Option<usize> {
        Snapshot::with_program(args).index_of_operand(n, &VALUE_OPTIONS)
    }

    #[test]
    fn operand_is_found_past_a_detached_option_value() {
        let args = args(&["numfmt", "--suffix", "q", "--from", "si", "q"]);
        assert_eq!(index_of_operand(&args, 0), Some(5));
        assert_eq!(index_of_operand(&args, 1), None);
    }

    #[test]
    fn operands_are_counted_in_order() {
        let args = args(&["numfmt", "1", "-d", ",", "--grouping", "2", "-z", "3"]);
        for (n, index) in [(0, 1), (1, 5), (2, 7)] {
            assert_eq!(index_of_operand(&args, n), Some(index), "operand {n}");
        }
    }

    #[test]
    fn a_dash_and_everything_past_a_double_dash_is_an_operand() {
        let args = args(&["numfmt", "-", "--", "-1", "--from=si"]);
        for (n, index) in [(0, 1), (1, 3), (2, 4)] {
            assert_eq!(index_of_operand(&args, n), Some(index), "operand {n}");
        }
    }

    #[test]
    fn an_attached_value_does_not_swallow_the_operand() {
        for args in [
            args(&["numfmt", "--suffix=q", "q"]),
            args(&["numfmt", "-d,", "q"]),
            args(&["numfmt", "--header=2", "q"]),
        ] {
            assert_eq!(index_of_operand(&args, 0), Some(2), "in {args:?}");
        }
    }

    #[test]
    fn an_abbreviated_long_option_still_takes_its_value() {
        let args = args(&["numfmt", "--suf", "q", "q"]);
        assert_eq!(index_of_operand(&args, 0), Some(3));
    }
}
