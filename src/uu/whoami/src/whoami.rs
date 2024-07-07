// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;

use uucore::display::println_verbatim;
use uucore::error::{FromIo, UResult};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    crate::uu_app().try_get_matches_from(args)?;
    let username = whoami()?;
    println_verbatim(username).map_err_context(|| "failed to print username".into())?;
    Ok(())
}

/// Get the current username
pub fn whoami() -> UResult<OsString> {
    crate::platform::get_username().map_err_context(|| "failed to get username".into())
}
