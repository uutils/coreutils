// This file is part of the uutils coreutils package.
//
// (c) Smigle00 <smigle00@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use platform_info::*;

use clap::{crate_version, Command};
use uucore::error::{FromIo, UResult};

static ABOUT: &str = "Display machine architecture";
static SUMMARY: &str = "Determine architecture name for current machine.";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_app().get_matches_from(args);

    let uts = PlatformInfo::new().map_err_context(|| "cannot get system name".to_string())?;
    println!("{}", uts.machine().trim());
    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(SUMMARY)
        .infer_long_args(true)
}
