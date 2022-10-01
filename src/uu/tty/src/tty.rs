//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
//  *
//  * Synced with http://lingrok.org/xref/coreutils/src/tty.c

// spell-checker:ignore (ToDO) ttyname filedesc

use clap::{crate_version, Arg, ArgAction, Command};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use uucore::error::{set_exit_code, UResult};
use uucore::format_usage;

static ABOUT: &str = "Print the file name of the terminal connected to standard input.";
const USAGE: &str = "{} [OPTION]...";

mod options {
    pub const SILENT: &str = "silent";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let silent = matches.get_flag(options::SILENT);

    // If silent, we don't need the name, only whether or not stdin is a tty.
    if silent {
        return if atty::is(atty::Stream::Stdin) {
            Ok(())
        } else {
            Err(1.into())
        };
    };

    let mut stdout = std::io::stdout();

    // Get the ttyname via nix
    let name = nix::unistd::ttyname(std::io::stdin().as_raw_fd());

    let write_result = match name {
        Ok(name) => writeln!(stdout, "{}", name.display()),
        Err(_) => {
            set_exit_code(1);
            writeln!(stdout, "not a tty")
        }
    };

    if write_result.is_err() || stdout.flush().is_err() {
        // Don't return to prevent a panic later when another flush is attempted
        // because the `uucore_procs::main` macro inserts a flush after execution for every utility.
        std::process::exit(3);
    };

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SILENT)
                .long(options::SILENT)
                .visible_alias("quiet")
                .short('s')
                .help("print nothing, only return an exit status")
                .action(ArgAction::SetTrue),
        )
}
