// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{Arg, ArgAction, Command};
use std::{ffi::OsString, io::Write};
use uucore::error::{UResult, set_exit_code};

use uucore::translate;

#[uucore::main]
// TODO: modify proc macro to allow no-result uumain
#[expect(clippy::unnecessary_wraps, reason = "proc macro requires UResult")]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<OsString> = args.collect();
    if args.len() != 2 {
        return Ok(());
    }

    // args[0] is the name of the binary.
    let error = if args[1] == "--help" {
        uu_app().print_help()
    } else if args[1] == "--version" {
        write!(std::io::stdout(), "{}", uu_app().render_version())
    } else {
        Ok(())
    };

    if let Err(print_fail) = error {
        // Try to display this error.
        let _ = writeln!(std::io::stderr(), "{}: {print_fail}", uucore::util_name());
        // Mirror GNU options. When failing to print warnings or version flags, then we exit
        // with FAIL. This avoids allocation some error information which may result in yet
        // other types of failure.
        set_exit_code(1);
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("true-about"))
        // We provide our own help and version options, to ensure maximum compatibility with GNU.
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("help")
                .long("help")
                .help(translate!("true-help-text"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .help(translate!("true-version-text"))
                .action(ArgAction::Version),
        )
}
