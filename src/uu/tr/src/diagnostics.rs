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

/// Render `err` against `args`, where `sets` are the set operands as typed.
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

    let (label, help) = describe(&err.error);
    // No span means the set as a whole is the problem, so underline all of it.
    let span = err.span.clone().unwrap_or(0..set.len());

    Snapshot::new(args).render_inside(
        set,
        span,
        &err.to_string(),
        &translate!(label),
        help.map(|key| translate!(key)).as_deref(),
    )
}

/// The caret label for `error`, and the advice that goes under it.
fn describe(error: &BadSequence) -> (&'static str, Option<&'static str>) {
    match error {
        BadSequence::MissingCharClassName => (
            "tr-diag-label-missing-char-class-name",
            Some("tr-diag-help-char-class"),
        ),
        BadSequence::InvalidCharClass(_) => (
            "tr-diag-label-invalid-char-class",
            Some("tr-diag-help-char-class"),
        ),
        BadSequence::MissingEquivalentClassChar => (
            "tr-diag-label-missing-equivalence-char",
            Some("tr-diag-help-equivalence"),
        ),
        BadSequence::MultipleCharInEquivalence(_) => (
            "tr-diag-label-multiple-char-in-equivalence",
            Some("tr-diag-help-equivalence"),
        ),
        BadSequence::InvalidRepeatCount(_) => (
            "tr-diag-label-invalid-repeat-count",
            Some("tr-diag-help-repeat"),
        ),
        BadSequence::BackwardsRange { .. } => (
            "tr-diag-label-backwards-range",
            Some("tr-diag-help-backwards-range"),
        ),
        BadSequence::CharRepeatInSet1 => (
            "tr-diag-label-char-repeat-in-set1",
            Some("tr-diag-help-repeat"),
        ),
        BadSequence::MultipleCharRepeatInSet2 => {
            ("tr-diag-label-multiple-char-repeat-in-set2", None)
        }
        BadSequence::ClassExceptLowerUpperInSet2 => {
            ("tr-diag-label-class-except-lower-upper-in-set2", None)
        }
        BadSequence::ClassInSet2NotMatchedBySet1 => {
            ("tr-diag-label-class-in-set2-not-matched", None)
        }
        BadSequence::Set1LongerSet2EndsInClass => {
            ("tr-diag-label-set1-longer-set2-ends-in-class", None)
        }
        BadSequence::ComplementMoreThanOneUniqueInSet2 => {
            ("tr-diag-label-complement-more-than-one-unique", None)
        }
        // Raised against the solved sets, so it never carries a location and is
        // filtered out before reaching here.
        BadSequence::EmptySet2WhenNotTruncatingSet1 => ("tr-diag-label-empty-set2", None),
    }
}
