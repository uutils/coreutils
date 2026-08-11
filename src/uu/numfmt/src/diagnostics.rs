// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps a [`FormatError`] onto the part of the `--format` string it came from,
//! so that [`uucore::diagnostics`] can render it with a caret.

use std::ffi::OsString;

use uucore::diagnostics::Snapshot;
use uucore::translate;

use crate::options::{FormatError, FormatErrorKind};

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
