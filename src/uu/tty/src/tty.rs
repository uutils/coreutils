//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
//  *
//  * Synced with http://lingrok.org/xref/coreutils/src/tty.c

// spell-checker:ignore (ToDO) ttyname filedesc

use clap::{crate_version, Arg, Command};
use std::ffi::CStr;
use std::io::Write;
use uucore::error::{UResult, UUsageError};
use uucore::{format_usage, InvalidEncodingHandling};

static ABOUT: &str = "Print the file name of the terminal connected to standard input.";
const USAGE: &str = "{} [OPTION]...";

mod options {
    pub const SILENT: &str = "silent";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = uu_app()
        .try_get_matches_from(args)
        .map_err(|e| UUsageError::new(2, format!("{}", e)))?;

    let silent = matches.is_present(options::SILENT);

    // Call libc function ttyname
    let tty = unsafe {
        let ptr = libc::ttyname(libc::STDIN_FILENO);
        if !ptr.is_null() {
            String::from_utf8_lossy(CStr::from_ptr(ptr).to_bytes()).to_string()
        } else {
            "".to_owned()
        }
    };

    let mut stdout = std::io::stdout();

    if !silent {
        let write_result = if !tty.chars().all(|c| c.is_whitespace()) {
            writeln!(stdout, "{}", tty)
        } else {
            writeln!(stdout, "not a tty")
        };
        if write_result.is_err() || stdout.flush().is_err() {
            // Don't return to prevent a panic later when another flush is attempted
            // because the `uucore_procs::main` macro inserts a flush after execution for every utility.
            std::process::exit(3);
        }
    }

    if atty::is(atty::Stream::Stdin) {
        Ok(())
    } else {
        Err(libc::EXIT_FAILURE.into())
    }
}

pub fn uu_app<'a>() -> Command<'a> {
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
                .required(false),
        )
}
