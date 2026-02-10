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
    // avoid large clap call for binary size
    let args: Vec<_> = args.collect();
    if args.len() == 1 {
        let uts = PlatformInfo::new()
            .map_err(|_e| USimpleError::new(1, translate!("cannot-get-system")))?;

        writeln!(stdout(), "{}", uts.machine().to_string_lossy().trim())?;
        return Ok(());
    }
    let arg_bytes = &args[1].as_encoded_bytes();
    if arg_bytes.starts_with(b"--v") && b"--version".starts_with(arg_bytes) || args[1] == "-V" {
        write!(stdout(), "{}", uu_app().render_version())?;
    } else if arg_bytes.starts_with(b"--h") && b"--help".starts_with(arg_bytes) || args[1] == "-h" {
        uu_app().print_help()?;
    } else {
        return Err(uu_app()
            .error(
                clap::error::ErrorKind::UnknownArgument,
                format!("unexpected argument '{}' found", args[1].to_string_lossy()),
            )
            .into());
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("arch-about"))
        .after_help(translate!("arch-after-help"))
        .infer_long_args(true)
}
