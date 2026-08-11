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
    let label = match err.kind {
        FormatErrorKind::MissingDirective => "numfmt-diag-label-missing-directive",
        FormatErrorKind::UnexpectedCharacter => "numfmt-diag-label-unexpected-character",
        FormatErrorKind::NumberOverflow => "numfmt-diag-label-number-overflow",
        FormatErrorKind::StrayPercent => "numfmt-diag-label-stray-percent",
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
        &translate!(label),
        Some(&translate!("numfmt-diag-help-format-syntax")),
    )
}
