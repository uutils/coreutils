// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::Command;
use std::ffi::OsString;
use uucore::display::println_verbatim;
use uucore::error::{FromIo, UResult};
use uucore::translate;

mod platform;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    let username = whoami()?;
    println_verbatim(username).map_err_context(|| translate!("whoami-error-failed-to-print"))?;
    Ok(())
}

/// Get the current username
pub fn whoami() -> UResult<OsString> {
    platform::get_username().map_err_context(|| translate!("whoami-error-failed-to-get"))
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("whoami-about"))
        .override_usage(translate!("whoami-usage"))
        .infer_long_args(true)
}
