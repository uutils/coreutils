// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use std::fs::remove_file;
use std::path::Path;

use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().try_get_matches_from(args)?;

    let path: &Path = matches
        .get_one::<OsString>(crate::options::OPT_PATH)
        .unwrap()
        .as_ref();

    remove_file(path).map_err_context(|| format!("cannot unlink {}", path.quote()))
}
