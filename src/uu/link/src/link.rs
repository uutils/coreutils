// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use std::fs::hard_link;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().try_get_matches_from(args)?;

    let files: Vec<_> = matches
        .get_many::<OsString>(crate::options::FILES)
        .unwrap_or_default()
        .collect();
    let old = Path::new(files[0]);
    let new = Path::new(files[1]);

    hard_link(old, new)
        .map_err_context(|| format!("cannot create link {} to {}", new.quote(), old.quote()))
}
