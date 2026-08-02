// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps an [`ExprError`] onto the argument it points at, so that
//! [`uucore::diagnostics`] can render it with a caret.

use uucore::diagnostics::Snapshot;
use uucore::translate;

use crate::ExprError;

/// What to point at: the argument index, the text for the caret, and optional
/// advice.
struct Located {
    index: usize,
    label: String,
    help: Option<String>,
}

/// Render `err` against `args`.
///
/// `stopped_at` is the number of arguments the parser had consumed, and is
/// `None` for errors raised once the expression was already parsed. Returns
/// `false` when the error cannot be tied to an argument, in which case the
/// caller should fall back to the plain one-line message.
pub fn render(args: &[Vec<u8>], err: &ExprError, stopped_at: Option<usize>) -> bool {
    let snapshot = Snapshot::from_bytes(args);
    let Some(located) = locate(&snapshot, err, stopped_at) else {
        return false;
    };
    snapshot.render(
        located.index,
        &err.to_string(),
        &located.label,
        located.help.as_deref(),
    )
}

fn locate(snapshot: &Snapshot, err: &ExprError, stopped_at: Option<usize>) -> Option<Located> {
    if snapshot.is_empty() {
        return None;
    }

    // The parser ran out of arguments, so the culprit is the last one it did
    // consume: the operator or parenthesis left dangling.
    let previous = || stopped_at.map(|index| index.saturating_sub(1));

    let (index, label, help) = match err {
        ExprError::UnexpectedArgument(_) => (
            // The parser stopped on the argument it did not expect.
            stopped_at,
            "expr-diag-label-unexpected-argument",
            Some("expr-diag-help-unexpected-argument"),
        ),
        ExprError::MissingArgument(_) => (
            previous(),
            "expr-diag-label-missing-argument",
            Some("expr-diag-help-missing-argument"),
        ),
        ExprError::ExpectedClosingBraceAfter(_) => (
            previous(),
            "expr-diag-label-expected-closing-brace-after",
            None,
        ),
        ExprError::ExpectedClosingBraceInsteadOf(_) => (
            previous(),
            "expr-diag-label-expected-closing-brace-instead-of",
            None,
        ),
        // Raised while evaluating, so the position is recovered from the value.
        // An operand computed by a subexpression will not be found, and the
        // error falls back to its plain form.
        ExprError::NonIntegerArgument(operand) => (
            snapshot.index_of_bytes(operand),
            "expr-diag-label-non-integer-argument",
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
        label: translate!(label),
        help: help.map(|key| translate!(key)),
    })
}
