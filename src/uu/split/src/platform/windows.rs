// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::ffi::OsStr;
use std::io::{BufWriter, Error, Result};
use std::io::{ErrorKind, Write};
use std::path::Path;
use uucore::fs;
use uucore::translate;

/// Get a file writer
///
/// Unlike the unix version of this function, this _always_ returns
/// a file writer
pub fn instantiate_current_writer(
    _filter: Option<&str>,
    filename: &str,
    is_new: bool,
) -> Result<BufWriter<Box<dyn Write>>> {
    let file = if is_new {
        // create new file
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(Path::new(&filename))
            .map_err(|e| match e.kind() {
                ErrorKind::IsADirectory => {
                    Error::other(translate!("split-error-is-a-directory", "dir" => filename))
                }
                _ => {
                    Error::other(translate!("split-error-unable-to-open-file", "file" => filename))
                }
            })?
    } else {
        // re-open file that we previously created to append to it
        std::fs::OpenOptions::new()
            .append(true)
            .open(Path::new(&filename))
            .map_err(|_| {
                Error::other(translate!("split-error-unable-to-reopen-file", "file" => filename))
            })?
    };
    Ok(BufWriter::new(Box::new(file) as Box<dyn Write>))
}

pub fn paths_refer_to_same_file(p1: &OsStr, p2: &OsStr) -> bool {
    // Windows doesn't support many of the unix ways of paths being equals
    fs::paths_refer_to_same_file(Path::new(p1), Path::new(p2), true)
}
