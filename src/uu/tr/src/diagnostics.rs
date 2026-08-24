// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps a [`SequenceError`] onto the part of the set it came from, so that
//! [`uucore::diagnostics`] can render it with a caret.

use std::ffi::OsString;

use uucore::diagnostics::Snapshot;
use uucore::translate;

use crate::operation::{BadSequence, SequenceError};

/// Render `err` against `args` — the whole argument list, program name
/// included — where `sets` are the set operands as typed.
///
/// Returns `false` when the set cannot be found among the arguments, in which
/// case the caller should fall back to the plain one-line message.
pub fn render(args: &[OsString], sets: &[OsString], err: &SequenceError) -> bool {
    let Some(set) = sets.get(usize::from(err.set) - 1) else {
        return false;
    };
    // Offsets into the set are byte offsets; a set that is not text cannot be
    // pointed into.
    let Some(set) = set.to_str() else {
        return false;
    };

    let Some((label, help)) = describe(&err.error) else {
        return false;
    };
    // No span means the set as a whole is the problem, so underline all of it.
    let span = err.span.clone().unwrap_or(0..set.len());

    // The sets are positional operands, numbered as they appear; this holds
    // because none of tr's options takes a separate value.
    let snapshot = Snapshot::with_program(args);
    let Some(index) = snapshot.index_of_positional(usize::from(err.set) - 1) else {
        return false;
    };
    snapshot.render_inside_at(
        index,
        set,
        span,
        &err.to_string(),
        label.as_deref(),
        help.map(|key| translate!(key)).as_deref(),
    )
}

/// The caret label for `error`, translated, and the advice that goes under
/// it, or `None` for an error that a caret cannot be placed for.
///
/// Labelled only where a label would add to the message — a fix, or the
/// decoding of an opaque one — per the convention in [`uucore::diagnostics`].
fn describe(error: &BadSequence) -> Option<(Option<String>, Option<&'static str>)> {
    match error {
        BadSequence::MissingCharClassName | BadSequence::InvalidCharClass(_) => {
            Some((None, Some("tr-diag-help-char-class")))
        }
        BadSequence::MissingEquivalentClassChar | BadSequence::MultipleCharInEquivalence(_) => {
            Some((None, Some("tr-diag-help-equivalence")))
        }
        BadSequence::InvalidRepeatCount(_) | BadSequence::CharRepeatInSet1 => {
            Some((None, Some("tr-diag-help-repeat")))
        }
        // The one fix is to swap the endpoints; suggest exactly that.
        BadSequence::BackwardsRange { end, start } => Some((
            Some(translate!(
                "tr-diag-label-backwards-range",
                "suggestion" => format!(
                    "{}-{}",
                    crate::operation::range_endpoint_to_string(*end),
                    crate::operation::range_endpoint_to_string(*start)
                )
            )),
            Some("tr-diag-help-backwards-range"),
        )),
        BadSequence::MultipleCharRepeatInSet2
        | BadSequence::ClassExceptLowerUpperInSet2
        | BadSequence::ClassInSet2NotMatchedBySet1 => Some((None, None)),
        BadSequence::Set1LongerSet2EndsInClass => Some((
            Some(translate!("tr-diag-label-set1-longer-set2-ends-in-class")),
            None,
        )),
        BadSequence::ComplementMoreThanOneUniqueInSet2 => Some((
            Some(translate!("tr-diag-label-complement-more-than-one-unique")),
            None,
        )),
        // Raised against the solved sets rather than what was typed, so there
        // is nothing to point a caret at.
        BadSequence::EmptySet2WhenNotTruncatingSet1 => None,
    }
}
