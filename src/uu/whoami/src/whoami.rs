// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::Command;
use std::ffi::OsString;
use uucore::display::println_verbatim;
use uucore::error::{FromIo, UResult};
use uucore::locale::get_message;

mod platform;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_app().try_get_matches_from(args)?;
    let username = whoami()?;
    println_verbatim(username).map_err_context(|| get_message("whoami-error-failed-to-print"))?;
    Ok(())
}

/// Get the current username
pub fn whoami() -> UResult<OsString> {
    platform::get_username().map_err_context(|| get_message("whoami-error-failed-to-get"))
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(get_message("whoami-about"))
        .override_usage(uucore::util_name())
        .infer_long_args(true)
}
