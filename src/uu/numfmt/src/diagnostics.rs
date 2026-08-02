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

/// Render `err`, raised while parsing `format`, against `args`.
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

    Snapshot::new(args).render_inside(
        format,
        err.span.clone(),
        &err.message,
        &translate!(label),
        Some(&translate!("numfmt-diag-help-format-syntax")),
    )
}
