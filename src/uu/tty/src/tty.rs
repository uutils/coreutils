//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
//  *
//  * Synced with http://lingrok.org/xref/coreutils/src/tty.c

// spell-checker:ignore (ToDO) ttyname filedesc

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::ffi::CStr;
use uucore::InvalidEncodingHandling;

static ABOUT: &str = "Print the file name of the terminal connected to standard input.";

mod options {
    pub const SILENT: &str = "silent";
}

fn get_usage() -> String {
    format!("{0} [OPTION]...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(options::SILENT)
                .long(options::SILENT)
                .visible_alias("quiet")
                .short("s")
                .help("print nothing, only return an exit status")
                .required(false),
        )
        .get_matches_from(args);

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

    if !silent {
        if !tty.chars().all(|c| c.is_whitespace()) {
            println!("{}", tty);
        } else {
            println!("not a tty");
        }
    }

    if atty::is(atty::Stream::Stdin) {
        libc::EXIT_SUCCESS
    } else {
        libc::EXIT_FAILURE
    }
}
