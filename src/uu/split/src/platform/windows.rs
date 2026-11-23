// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::platform::Writer;
use crate::platform::output_file::open_output_file;
use std::ffi::OsStr;
use std::io::Result;
use std::path::Path;
use uucore::fs;

/// Get a file writer
///
/// Unlike the unix version of this function, this _always_ returns
/// a file writer
pub fn instantiate_current_writer(
    _filter: Option<&str>,
    input: &OsStr,
    filename: &OsStr,
    is_new: bool,
) -> Result<Writer> {
    Ok(Writer::File(open_output_file(input, filename, is_new)?))
}

pub fn paths_refer_to_same_file(p1: &OsStr, p2: &OsStr) -> bool {
    // Windows doesn't support many of the unix ways of paths being equals
    fs::paths_refer_to_same_file(Path::new(p1), Path::new(p2), true)
}
