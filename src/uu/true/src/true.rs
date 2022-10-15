//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
use clap::{Arg, ArgAction, Command};
use std::{ffi::OsString, io::Write};
use uucore::error::{set_exit_code, UResult};

static ABOUT: &str = "\
Returns true, a successful exit status.

Immediately returns with the exit status `0`, except when invoked with one of the recognized
options. In those cases it will try to write the help or version text. Any IO error during this
operation causes the program to return `1` instead.
";

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
                writeln!(std::io::stdout(), "{}", command.render_version())
            }
            _ => Ok(()),
        };

        if let Err(print_fail) = error {
            // Try to display this error.
            let _ = writeln!(std::io::stderr(), "{}: {}", uucore::util_name(), print_fail);
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
        .version(clap::crate_version!())
        .about(ABOUT)
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
