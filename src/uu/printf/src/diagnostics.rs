// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps a [`FormatError`] onto the part of the format string it came from, so
//! that [`uucore::diagnostics`] can render it with a caret.

use std::ffi::{OsStr, OsString};

use uucore::diagnostics::Snapshot;
use uucore::format::FormatError;
use uucore::translate;

/// Render `error` against `args` — the whole argument list, program name
/// included — where `format` is the format operand as typed.
///
/// Returns `false` when the error does not point into the format string or
/// the format cannot be found among the arguments, in which case the caller
/// should fall back to the plain one-line message.
pub fn render(args: &[OsString], format: &[u8], error: &FormatError) -> bool {
    let (span, help) = match error {
        FormatError::SpecError(_, span) => (span, "printf-diag-help-spec"),
        // A `None` span is an escape parsed out of something other than the
        // format string, so there is nothing to point a caret at.
        FormatError::MissingHex(Some(span)) => (span, "printf-diag-help-escape-hex"),
        FormatError::InvalidCharacter(_, _, Some(span)) => (span, "printf-diag-help-unicode"),
        _ => return false,
    };
    // Offsets into the format are byte offsets; a format that is not text
    // cannot be pointed into.
    let Ok(format) = std::str::from_utf8(format) else {
        return false;
    };

    // The format is the first operand that spells it out: printf takes hyphen
    // values, so `printf -%y` is a format rather than an option, and counting
    // positionals would walk right past it.
    let snapshot = Snapshot::with_program(args);
    let Some(index) = snapshot.index_of(OsStr::new(format)) else {
        return false;
    };
    snapshot.render_inside_at(
        index,
        format,
        span.clone(),
        &error.to_string(),
        None,
        Some(&translate!(help)),
    )
}
