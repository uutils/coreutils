// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::Command;
use platform_info::{PlatformInfo, PlatformInfoAPI, UNameAPI};
use std::io::{Write, stdout};
use uucore::error::{UResult, USimpleError};
use uucore::translate;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let uts =
        PlatformInfo::new().map_err(|_e| USimpleError::new(1, translate!("cannot-get-system")))?;

    writeln!(stdout(), "{}", uts.machine().to_string_lossy().trim())?;
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("arch-about"))
        .after_help(translate!("arch-after-help"))
        .override_usage(translate!("arch-usage"))
        .infer_long_args(true)
}
