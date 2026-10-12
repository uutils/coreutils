// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps an [`ExprError`] onto the argument it points at, so that
//! [`uucore::diagnostics`] can render it with a caret.

use uucore::diagnostics::Snapshot;
use uucore::translate;

use crate::{ExprError, FailurePoint};

/// What to point at: the argument index, optional text for the caret, and
/// optional advice.
struct Located {
    index: usize,
    label: Option<String>,
    help: Option<String>,
}

/// Render `err` against `args`.
///
/// `at` says where the expression failed: how many arguments the parser had
/// consumed, or which argument evaluation blames. Returns `false` when the
/// error cannot be tied to an argument, in which case the caller should fall
/// back to the plain one-line message.
pub fn render(args: &[Vec<u8>], err: &ExprError, at: &FailurePoint) -> bool {
    let snapshot = Snapshot::from_bytes(args);
    let Some(located) = locate(&snapshot, err, at) else {
        return false;
    };
    snapshot.render(
        located.index,
        &err.to_string(),
        located.label.as_deref(),
        located.help.as_deref(),
    )
}

fn locate(snapshot: &Snapshot, err: &ExprError, at: &FailurePoint) -> Option<Located> {
    if snapshot.is_empty() {
        return None;
    }

    let stopped_at = match at {
        FailurePoint::Parse(index) => Some(*index),
        FailurePoint::Eval(_) => None,
    };
    // The parser ran out of arguments, so the culprit is the last one it did
    // consume: the operator or parenthesis left dangling.
    let previous = || stopped_at.map(|index| index.saturating_sub(1));

    // Labelled only where a label would add to the message, per the convention
    // in `uucore::diagnostics`.
    let (index, label, help) = match err {
        ExprError::UnexpectedArgument(_) => (
            // The parser stopped on the argument it did not expect.
            stopped_at,
            Some("diagnostics-label-expression-complete"),
            Some("expr-diag-help-unexpected-argument"),
        ),
        ExprError::MissingArgument(_) => {
            (previous(), None, Some("expr-diag-help-missing-argument"))
        }
        ExprError::ExpectedClosingBraceAfter(_) | ExprError::ExpectedClosingBraceInsteadOf(_) => {
            (previous(), None, None)
        }
        // Raised while evaluating; the evaluator says which argument it
        // blames. An operand computed by a subexpression has no argument of
        // its own, and the error falls back to its plain form.
        ExprError::NonIntegerArgument(_) => (
            match at {
                FailurePoint::Eval(index) => *index,
                FailurePoint::Parse(_) => None,
            },
            None,
            Some("expr-diag-help-non-integer-argument"),
        ),
        // An empty expression, a malformed regex, division by zero: nothing to
        // single out.
        ExprError::MissingOperand
        | ExprError::DivisionByZero
        | ExprError::InvalidRegexExpression
        | ExprError::UnmatchedOpeningParenthesis
        | ExprError::UnmatchedClosingParenthesis
        | ExprError::UnmatchedOpeningBrace
        | ExprError::InvalidBracketContent
        | ExprError::TrailingBackslash
        | ExprError::TooBigRangeQuantifierIndex
        | ExprError::UnsupportedNonUtf8Match(_) => return None,
    };

    Some(Located {
        index: index?,
        label: label.map(|key| translate!(key)),
        help: help.map(|key| translate!(key)),
    })
}
