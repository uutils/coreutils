// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::io::Write;
use std::io::{BufWriter, Error, ErrorKind, Result};
use std::path::Path;
use uucore::fs;

/// Get a file writer
///
/// Unlike the unix version of this function, this _always_ returns
/// a file writer
pub fn instantiate_current_writer(
    _filter: &Option<String>,
    filename: &str,
) -> Result<BufWriter<Box<dyn Write>>> {
    Ok(BufWriter::new(Box::new(
        // write to the next file
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(std::path::Path::new(&filename))
            .map_err(|_| {
                Error::new(
                    ErrorKind::Other,
                    format!("'{filename}' would overwrite input; aborting"),
                )
            })?,
    ) as Box<dyn Write>))
}

pub fn paths_refer_to_same_file(p1: &str, p2: &str) -> bool {
    // Windows doesn't support many of the unix ways of paths being equals
    fs::paths_refer_to_same_file(Path::new(p1), Path::new(p2), true)
}
