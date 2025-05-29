// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use platform_info::*;

use clap::Command;
use uucore::error::{UResult, USimpleError};
use uucore::locale::{self, get_message};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    locale::setup_localization(uucore::util_name())?;
    uu_app().try_get_matches_from(args)?;

    let uts =
        PlatformInfo::new().map_err(|_e| USimpleError::new(1, get_message("cannot-get-system")))?;

    println!("{}", uts.machine().to_string_lossy().trim());
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(get_message("arch-about"))
        .after_help(get_message("arch-after-help"))
        .infer_long_args(true)
}
