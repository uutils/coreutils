// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps a [`ParseError`] onto the argument it points at, so that
//! [`uucore::diagnostics`] can render it with a caret.

use std::ffi::OsString;

use uucore::diagnostics::Snapshot;
use uucore::translate;

use crate::error::{ErrorAt, ParseError, ParseErrorKind};

/// Render `err` against `args`.
///
/// Returns `false` when the error cannot be tied to an argument, in which case
/// the caller should fall back to the plain one-line message.
pub fn render(args: &[OsString], err: &ParseError) -> bool {
    let snapshot = Snapshot::new(args);

    let index = match &err.at {
        ErrorAt::Unknown => return false,
        ErrorAt::Token(index) => *index,
        // Recovered from the value, for errors raised after parsing.
        ErrorAt::Value(value) => match snapshot.index_of(value) {
            Some(index) => index,
            None => return false,
        },
    };

    // Labelled only where a label would add to the message, per the convention
    // in `uucore::diagnostics`.
    let (label, help) = match &err.kind {
        ParseErrorKind::Expected(_) => (None, None),
        ParseErrorKind::ExtraArgument(_) => (
            Some(translate!("test-diag-label-extra-argument")),
            Some(translate!("test-diag-help-extra-argument")),
        ),
        ParseErrorKind::MissingArgument(_) => {
            (None, Some(translate!("test-diag-help-missing-argument")))
        }
        ParseErrorKind::UnaryOperatorExpected(_) => (
            Some(translate!("test-diag-label-unary-operator-expected")),
            None,
        ),
        ParseErrorKind::InvalidInteger(_) => (
            None,
            Some(format!(
                "{}\n{}",
                translate!("test-diag-help-integer-op"),
                translate!("test-diag-help-integer-op-mnemonics")
            )),
        ),
        ParseErrorKind::InvalidFileDescriptor(_) => {
            (None, Some(translate!("test-diag-help-file-descriptor")))
        }
        ParseErrorKind::UnknownOperator(_) => (
            None,
            Some(translate!(
                "test-diag-help-unknown-operator",
                "name" => uucore::util_name()
            )),
        ),
        // Never carries a position, so it is filtered out above.
        ParseErrorKind::ExpectedValue => return false,
    };

    snapshot.render(
        index,
        &err.kind.to_string(),
        label.as_deref(),
        help.as_deref(),
    )
}
