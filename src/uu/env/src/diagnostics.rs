// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps an [`EnvError`] onto the part of the `-S` string it came from, so that
//! [`uucore::diagnostics`] can render it with a caret.
//!
//! Every error the splitter raises already knows the offset it stopped at —
//! the plain message reports it as `at position 12` — so the caret has only to
//! be placed there.

use std::ffi::OsString;
use std::ops::Range;

use uucore::diagnostics::{self, Snapshot};
use uucore::translate;

use crate::EnvError;

/// Render `err`, raised while splitting `payload`, against `args`.
///
/// # Arguments
///
/// * `args` - The whole argument list, program name included.
/// * `index` - Position of the argument carrying the `-S` string.
/// * `payload` - The `-S` string as typed. It is the tail of the argument at
///   `index`, whether it was written `-S 'a b'`, `-S'a b'` or
///   `--split-string='a b'`.
/// * `message` - The error message, already localized.
///
/// # Returns
///
/// `false` when the error carries no position to point at, in which case the
/// caller should fall back to the plain one-line message.
pub fn render(
    args: &[OsString],
    index: usize,
    payload: &str,
    message: &str,
    err: &EnvError,
) -> bool {
    let Some((span, label, help)) = describe(payload, err) else {
        return false;
    };
    Snapshot::with_program(args).render_inside_at(
        index,
        payload,
        span,
        message,
        label.map(|key| translate!(key)).as_deref(),
        Some(&translate!(help)),
    )
}

/// Where to point inside `payload` for `err`, the caret label, and the advice
/// that goes under it.
///
/// Labelled only where a label would add to the message, per the convention in
/// [`uucore::diagnostics`].
fn describe(
    payload: &str,
    err: &EnvError,
) -> Option<(Range<usize>, Option<&'static str>, &'static str)> {
    match err {
        // The string ran out while a quote was still open. The parser stopped
        // at the end, which is exactly where the missing quote belongs.
        EnvError::EnvMissingClosingQuote(pos, _) => {
            let at = byte_offset(payload, *pos);
            Some((at..at, None, "env-diag-help-quoting"))
        }
        EnvError::EnvInvalidBackslashAtEndOfStringInMinusS(pos, _) => {
            let at = byte_offset(payload, *pos);
            Some((at..at, None, "env-diag-help-backslash"))
        }
        EnvError::EnvBackslashCNotAllowedInDoubleQuotes(pos) => Some((
            span_back_to(payload, *pos, '\\'),
            None,
            "env-diag-help-backslash-c",
        )),
        // The offset is the character after the backslash; the sequence that
        // was rejected is the two of them together.
        EnvError::EnvInvalidSequenceBackslashXInMinusS(pos, _) => Some((
            span_back_to(payload, *pos, '\\'),
            None,
            "env-diag-help-escape",
        )),
        // Raised anywhere inside a `$…` reference, so the caret covers the
        // reference from its `$` rather than the one character that stopped
        // the parse.
        EnvError::EnvParsingOfVariableUnexpectedNumber(pos, _) => Some((
            span_back_to(payload, *pos, '$'),
            Some("env-diag-label-variable-digit"),
            "env-diag-help-variable",
        )),
        EnvError::EnvParsingOfVariableMissingClosingBrace(pos) => Some((
            span_back_to(payload, *pos, '$'),
            Some("env-diag-label-missing-brace"),
            "env-diag-help-variable",
        )),
        EnvError::EnvParsingOfMissingVariable(pos)
        | EnvError::EnvParsingOfVariableOnlyBracedName(pos) => Some((
            span_back_to(payload, *pos, '$'),
            None,
            "env-diag-help-variable",
        )),
        // Control-flow signals and internal errors: nothing a reader typed.
        EnvError::EnvReachedEnd
        | EnvError::EnvContinueWithDelimiter
        | EnvError::EnvInternalError(_, _) => None,
    }
}

/// `pos`, as a byte offset into `payload`.
///
/// The splitter counts in `NativeCharInt`s, which are bytes on unix but UTF-16
/// units on Windows; a caret needs the offset in the UTF-8 bytes the report is
/// drawn from. Anything past the end of `payload` stays past it, so that
/// [`diagnostics::char_span`] still reads it as "nothing left to point at".
fn byte_offset(payload: &str, pos: usize) -> usize {
    #[cfg(not(windows))]
    {
        let _ = payload;
        pos
    }
    #[cfg(windows)]
    {
        let mut units = 0;
        for (offset, c) in payload.char_indices() {
            if units >= pos {
                return offset;
            }
            units += c.len_utf16();
        }
        payload.len()
    }
}

/// The character the parser stopped at, extended back to the nearest `opener`
/// in front of it — the `\` of an escape, or the `$` of a variable reference.
///
/// Blaming the whole of what was written reads better than a caret on the one
/// character the parse gave up on, which on its own says little.
fn span_back_to(payload: &str, pos: usize, opener: char) -> Range<usize> {
    let stopped = diagnostics::char_span(payload, byte_offset(payload, pos));
    let start = payload[..stopped.start]
        .rfind(opener)
        .unwrap_or(stopped.start);
    start..stopped.end
}
