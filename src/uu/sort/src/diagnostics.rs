// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Maps a [`KeyError`] onto the part of the `-k` spec it came from, so that
//! [`uucore::diagnostics`] can render it with a caret.

use std::ffi::OsString;

use uucore::diagnostics::Snapshot;
use uucore::translate;

use crate::{KeyError, KeyErrorKind};

/// Render `err`, raised while parsing `key`, against `args` — the whole
/// argument list, program name included.
///
/// Returns `false` when the key cannot be found among the arguments, in which
/// case the caller should fall back to the plain one-line message.
pub fn render(args: &[OsString], key: &str, err: &KeyError) -> bool {
    let (label, help) = match err.kind {
        KeyErrorKind::MissingCount => ("sort-diag-label-missing-count", true),
        KeyErrorKind::ZeroCount => ("sort-diag-label-zero-count", false),
        KeyErrorKind::StrayCharacter => ("sort-diag-label-stray-character", true),
        KeyErrorKind::IncompatibleOptions => ("sort-diag-label-incompatible-options", false),
    };

    let snapshot = Snapshot::with_program(args);
    let Some(index) = snapshot.index_of_value(key, Some('k'), Some("key")) else {
        return false;
    };
    snapshot.render_inside_at(
        index,
        key,
        err.span.clone(),
        &err.message,
        &translate!(label),
        help.then(|| translate!("sort-diag-help-key-syntax"))
            .as_deref(),
    )
}
