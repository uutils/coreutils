// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use platform_info::*;

use clap::{crate_version, Command};
use uucore::error::{UResult, USimpleError};
use uucore::{help_about, help_section};

static ABOUT: &str = help_about!("arch.md");
static SUMMARY: &str = help_section!("after help", "arch.md");

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_app().try_get_matches_from(args)?;

    let uts = PlatformInfo::new().map_err(|_e| USimpleError::new(1, "cannot get system name"))?;

    println!("{}", uts.machine().to_string_lossy().trim());
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(SUMMARY)
        .infer_long_args(true)
}
