// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore parseargs

//! Maps a [`ParseError`] onto the part of the operand it came from, so that
//! [`uucore::diagnostics`] can render it with a caret.
//!
//! Every dd operand is a `KEY=VALUE` pair, so an error is about the key, the
//! value, or one flag inside a comma-separated value; the caret says which.

use std::ffi::{OsStr, OsString};
use std::ops::Range;

use uucore::diagnostics::{Snapshot, list_items};
use uucore::error::UError;
use uucore::translate;

use crate::parseargs::ParseError;

/// The error to raise for an operand dd rejected.
///
/// Draws the caret when the arguments as typed were kept, and quiets `error`
/// when it did: the report has already said everything the one-line message
/// would, and the exit code is all that is left to carry.
///
/// # Arguments
///
/// * `diag_args` - The arguments as typed, program name included, or `None`
///   when they were not kept.
/// * `operand` - The `KEY=VALUE` operand at fault, as typed.
/// * `error` - What the parser made of it.
pub fn operand_error(
    diag_args: Option<&[OsString]>,
    operand: &str,
    error: ParseError,
) -> Box<dyn UError> {
    uucore::diagnostics::error_after_report(diag_args, error, |args, error| {
        render(args, operand, error)
    })
}

/// Render `error` against `args`, with a caret under the part of `operand`
/// that is at fault.
///
/// # Returns
///
/// `false` when the error is not about a part of the operand, or when the
/// operand cannot be found among the arguments.
fn render(args: &[OsString], operand: &str, error: &ParseError) -> bool {
    let key_end = operand.find('=').unwrap_or(operand.len());
    // The value starts past the `=`, or ends the operand when there is none.
    let value_start = operand.len().min(key_end + 1);
    let value = || value_start..operand.len();
    // A flag inside a comma-separated value, at its place in the list rather
    // than wherever its text first turns up.
    let flag = |flag: &str| {
        list_items(&operand[value_start..], &[','])
            .find(|&(part, _)| part == flag)
            .map(|(_, span)| value_start + span.start..value_start + span.end)
    };

    // The label says what is wrong with the span, the help what would have
    // been right; a flag list names the flags rather than the commas, which
    // are not what the parser tripped on.
    let (span, label, help): (Range<usize>, Option<&str>, &str) = match error {
        ParseError::UnrecognizedOperand(_) => (0..key_end, None, "dd-diag-help-operand"),
        ParseError::FlagNoMatch(name) => (
            flag(name).unwrap_or_else(value),
            Some("dd-diag-label-iflag"),
            "dd-diag-help-iflag",
        ),
        ParseError::OutputFlagNoMatch(name) => (
            flag(name).unwrap_or_else(value),
            Some("dd-diag-label-oflag"),
            "dd-diag-help-oflag",
        ),
        ParseError::ConvFlagNoMatch(name) => (
            flag(name).unwrap_or_else(value),
            Some("dd-diag-label-conv"),
            "dd-diag-help-conv",
        ),
        ParseError::StatusLevelNotRecognized(_) => (value(), None, "dd-diag-help-status"),
        ParseError::MultiplierStringParseFailure(_)
        | ParseError::MultiplierStringOverflow(_)
        | ParseError::InvalidNumber(_)
        | ParseError::InvalidNumberWithErrMsg(_, _)
        | ParseError::BsOutOfRange(_) => (value(), None, "dd-diag-help-number"),
        // The rest is about how operands combine rather than about one of
        // them, so there is nothing to point a caret at.
        _ => return false,
    };

    let snapshot = Snapshot::with_program(args);
    let Some(index) = snapshot.index_of(OsStr::new(operand)) else {
        return false;
    };
    let label = label.map(|key| translate!(key));
    snapshot.render_inside_at(
        index,
        operand,
        span,
        &error.to_string(),
        label.as_deref(),
        Some(&translate!(help)),
    )
}
