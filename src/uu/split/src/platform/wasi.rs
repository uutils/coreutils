// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::platform::Writer;
use std::ffi::OsStr;
use uucore::{display::Quotable, translate};

// WASI has no process spawning (the `--filter` writer) and no fd-based inode
// comparison, so it falls back to a path-based identity check via canonicalize.
pub fn paths_refer_to_same_file(p1: &OsStr, p2: &OsStr) -> bool {
    match (std::fs::canonicalize(p1), std::fs::canonicalize(p2)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

pub fn instantiate_current_writer(
    _filter: Option<&str>,
    input: &OsStr,
    filename: &OsStr,
    is_new: bool,
) -> std::io::Result<Writer> {
    // Refuse to truncate/overwrite the input. WASI cannot do the fd-based check
    // unix/windows use, so this is a best-effort path comparison.
    if paths_refer_to_same_file(input, filename) {
        return Err(std::io::Error::other(
            translate!("split-error-would-overwrite-input", "file" => filename.quote()),
        ));
    }
    let file = if is_new {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(std::path::Path::new(filename))?
    } else {
        std::fs::OpenOptions::new()
            .append(true)
            .open(std::path::Path::new(filename))?
    };
    Ok(Writer::File(file))
}
