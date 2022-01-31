//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
use clap::{App, AppSettings, ErrorKind};
use std::io::Write;
use uucore::error::{set_exit_code, UResult};

static ABOUT: &str = "
 Returns true, a successful exit status.

 Immediately returns with the exit status `0`, except when invoked with one of the recognized
 options. In those cases it will try to write the help or version text. Any IO error during this
 operation causes the program to return `1` instead.
";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let app = uu_app();

    if let Err(err) = app.try_get_matches_from(args) {
        if let ErrorKind::DisplayHelp | ErrorKind::DisplayVersion = err.kind {
            if let Err(print_fail) = err.print() {
                // Try to display this error.
                let _ = writeln!(std::io::stderr(), "{}: {}", uucore::util_name(), print_fail);
                // Mirror GNU options. When failing to print warnings or version flags, then we exit
                // with FAIL. This avoids allocation some error information which may result in yet
                // other types of failure.
                set_exit_code(1);
            }
        }
    }

    Ok(())
}

pub fn uu_app<'a>() -> App<'a> {
    App::new(uucore::util_name())
        .version(clap::crate_version!())
        .about(ABOUT)
        .setting(AppSettings::InferLongArgs)
}
