// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps a [`ModeError`] onto the clause of the mode it came from, so that
//! [`uucore::diagnostics`] can render it with a caret.

use std::ffi::OsString;

use uucore::diagnostics::Snapshot;
use uucore::mode::{ModeError, ModeErrorKind};
use uucore::translate;

/// Render `err` against `args`.
///
/// `mode` is the whole mode operand and `clause_start` where the clause that
/// failed begins inside it, since a mode is parsed one comma-separated clause
/// at a time.
///
/// Returns `false` when the mode cannot be found among the arguments, in which
/// case the caller should fall back to the plain one-line message.
pub fn render(args: &[OsString], mode: &str, clause_start: usize, err: &ModeError) -> bool {
    let span = clause_start + err.span.start..clause_start + err.span.end;
    let label = match err.kind {
        ModeErrorKind::InvalidOperator => "chmod-diag-label-invalid-operator",
        ModeErrorKind::MissingOperator => "chmod-diag-label-missing-operator",
        ModeErrorKind::InvalidNumber => "chmod-diag-label-invalid-number",
    };

    Snapshot::new(args).render_inside(
        mode,
        span,
        &err.message,
        &translate!(label),
        Some(&translate!("chmod-diag-help-mode-syntax")),
    )
}
