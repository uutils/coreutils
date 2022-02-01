//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
use clap::{App, Arg, ArgSettings, ErrorKind};
use std::io::Write;
use uucore::error::{set_exit_code, UResult};

static ABOUT: &str = "
 Returns false, an unsuccessful exit status.

 Immediately returns with the exit status `1`. When invoked with one of the recognized options it
 will try to write the help or version text. Any IO error during this operation is diagnosed, yet
 the program will also return `1`.
";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let app = uu_app();

    // Mirror GNU options, always return `1`. In particular even the 'successful' cases of no-op,
    // and the interrupted display of help and version should return `1`. Also, we return Ok in all
    // paths to avoid the allocation of an error object, an operation that could, in theory, fail
    // and unwind through the standard library allocation handling machinery.
    set_exit_code(1);

    if let Err(err) = app.try_get_matches_from(args) {
        if let ErrorKind::DisplayHelp | ErrorKind::DisplayVersion = err.kind {
            if let Err(print_fail) = err.print() {
                // Try to display this error.
                let _ = writeln!(std::io::stderr(), "{}: {}", uucore::util_name(), print_fail);
            }
        }
    }

    Ok(())
}

pub fn uu_app<'a>() -> App<'a> {
    App::new(uucore::util_name())
        .version(clap::crate_version!())
        .about(ABOUT)
        // Hide the default -V and -h for version and help.
        // This requires us to overwrite short, not short_aliases.
        .arg(
            Arg::new("dummy-help")
                .short('h')
                .setting(ArgSettings::Hidden),
        )
        .arg(
            Arg::new("dummy-version")
                .short('V')
                .setting(ArgSettings::Hidden),
        )
}
