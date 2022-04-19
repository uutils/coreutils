//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
use clap::{Arg, Command};
use std::io::Write;
use uucore::error::{set_exit_code, UResult};

static ABOUT: &str = "\
Returns false, an unsuccessful exit status.

Immediately returns with the exit status `1`. When invoked with one of the recognized options it
will try to write the help or version text. Any IO error during this operation is diagnosed, yet
the program will also return `1`.
";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut command = uu_app();

    // Mirror GNU options, always return `1`. In particular even the 'successful' cases of no-op,
    // and the interrupted display of help and version should return `1`. Also, we return Ok in all
    // paths to avoid the allocation of an error object, an operation that could, in theory, fail
    // and unwind through the standard library allocation handling machinery.
    set_exit_code(1);

    if let Ok(matches) = command.try_get_matches_from_mut(args) {
        let error = if matches.index_of("help").is_some() {
            command.print_long_help()
        } else if matches.index_of("version").is_some() {
            writeln!(std::io::stdout(), "{}", command.render_version())
        } else {
            Ok(())
        };

        // Try to display this error.
        if let Err(print_fail) = error {
            // Completely ignore any error here, no more failover and we will fail in any case.
            let _ = writeln!(std::io::stderr(), "{}: {}", uucore::util_name(), print_fail);
        }
    }

    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
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
                .exclusive(true),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .help("Print version information"),
        )
}
