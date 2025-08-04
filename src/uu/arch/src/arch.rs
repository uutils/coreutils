// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use platform_info::*;

use clap::Command;
use uucore::LocalizedCommand;
use uucore::error::{UResult, USimpleError};
use uucore::translate;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_app().try_get_matches_from_localized(args);

    let uts =
        PlatformInfo::new().map_err(|_e| USimpleError::new(1, translate!("cannot-get-system")))?;

    println!("{}", uts.machine().to_string_lossy().trim());
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("arch-about"))
        .after_help(translate!("arch-after-help"))
        .infer_long_args(true)
}
