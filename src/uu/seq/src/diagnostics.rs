// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps a [`FormatError`] onto the part of the `-f` format it came from, so
//! that [`uucore::diagnostics`] can render it with a caret.

use std::ffi::OsString;
use std::ops::Range;

use uucore::diagnostics::{OptionValue, Snapshot};
use uucore::format::FormatError;
use uucore::translate;

/// Render `error` against `args` — the whole argument list, program name
/// included — where `format` is the value of `-f`/`--format` as typed.
///
/// # Returns
///
/// `false` when the error is not about the format string, or when the format
/// cannot be found among the arguments, in which case the caller should fall
/// back to the plain one-line message.
pub fn render(args: &[OsString], format: &str, error: &FormatError) -> bool {
    let span: Range<usize> = match error {
        FormatError::SpecError(_, span)
        | FormatError::MissingHex(Some(span))
        | FormatError::InvalidCharacter(_, _, Some(span)) => span.clone(),
        // These are about the format as a whole — it holds no directive, or
        // more than one, or one seq cannot print a number with — so the caret
        // takes all of it.
        FormatError::TooManySpecs(_)
        | FormatError::NeedAtLeastOneSpec(_)
        | FormatError::EndsWithPercent(_)
        | FormatError::WrongSpecType => 0..format.len(),
        _ => return false,
    };

    Snapshot::with_program(args).render_option(
        &OptionValue::new(format, 'f', crate::OPT_FORMAT),
        span,
        &error.to_string(),
        None,
        Some(&translate!("seq-diag-help-format")),
    )
}
