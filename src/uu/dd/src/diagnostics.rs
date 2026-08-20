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

use uucore::diagnostics::Snapshot;
use uucore::error::{UError, quiet_if_reported};
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
    let reported = diag_args.is_some_and(|args| render(args, operand, &error));
    quiet_if_reported(reported, error)
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
    // A flag inside a comma-separated value. The list is walked the way the
    // parser walks it rather than searched for the flag's text, which would
    // match inside an earlier flag the failing one is a prefix of — the `noc`
    // of `nocache,noc`.
    let flag = |flag: &str| {
        let mut at = value_start;
        for part in operand[value_start..].split(',') {
            if part == flag {
                return Some(at..at + part.len());
            }
            // Every separator is one byte wide.
            at += part.len() + 1;
        }
        None
    };

    let (span, help): (Range<usize>, &str) = match error {
        ParseError::UnrecognizedOperand(_) => (0..key_end, "dd-diag-help-operand"),
        ParseError::FlagNoMatch(name) | ParseError::ConvFlagNoMatch(name) => {
            (flag(name).unwrap_or_else(value), "dd-diag-help-flags")
        }
        ParseError::StatusLevelNotRecognized(_) => (value(), "dd-diag-help-status"),
        ParseError::MultiplierStringParseFailure(_)
        | ParseError::MultiplierStringOverflow(_)
        | ParseError::InvalidNumber(_)
        | ParseError::InvalidNumberWithErrMsg(_, _)
        | ParseError::BsOutOfRange(_) => (value(), "dd-diag-help-number"),
        // The rest is about how operands combine rather than about one of
        // them, so there is nothing to point a caret at.
        _ => return false,
    };

    let snapshot = Snapshot::with_program(args);
    let Some(index) = snapshot.index_of(OsStr::new(operand)) else {
        return false;
    };
    snapshot.render_inside_at(
        index,
        operand,
        span,
        &error.to_string(),
        None,
        Some(&translate!(help)),
    )
}
