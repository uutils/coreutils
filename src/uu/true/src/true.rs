// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{Arg, ArgAction, Command};
use std::{ffi::OsString, io::Write};
use uucore::error::{UResult, set_exit_code};

use uucore::locale::get_message;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut command = uu_app();

    let args: Vec<OsString> = args.collect();
    if args.len() > 2 {
        return Ok(());
    }

    if let Err(e) = command.try_get_matches_from_mut(args) {
        let error = match e.kind() {
            clap::error::ErrorKind::DisplayHelp => command.print_help(),
            clap::error::ErrorKind::DisplayVersion => {
                write!(std::io::stdout(), "{}", command.render_version())
            }
            _ => Ok(()),
        };

        if let Err(print_fail) = error {
            // Try to display this error.
            let _ = writeln!(std::io::stderr(), "{}: {print_fail}", uucore::util_name());
            // Mirror GNU options. When failing to print warnings or version flags, then we exit
            // with FAIL. This avoids allocation some error information which may result in yet
            // other types of failure.
            set_exit_code(1);
        }
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(get_message("true-about"))
        // We provide our own help and version options, to ensure maximum compatibility with GNU.
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("help")
                .long("help")
                .help("Print help information")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .help("Print version information")
                .action(ArgAction::Version),
        )
}
