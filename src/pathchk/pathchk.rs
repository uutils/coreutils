#![crate_name = "uu_pathchk"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Inokentiy Babushkin <inokentiy.babushkin@googlemail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use getopts::Options;
use std::io::Write;

enum Mode {
    PosixMost,
    PosixSpecial,
    PosixAll,
    Help,
    Version
}

enum PathCheckResult {
    PathOk,
    EmptyFileName,
    LstatError(String),
    NoFileNameLimit(String),
    FileLimitExceeded(u32, u32, String),
    ComponentLimitExceeded(u32, u32, String),
    DirectoryError(u32, String) // TODO: meh
}

static NAME: &'static str = "pathchk";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

// a few global constants as used in the GNU implememntation
static POSIX_PATH_MAX: u32 = 256;
static POSIX_NAME_MAX: u32 = 14;
// static PATH_MAX_MINIMUM: u32 = POSIX_PATH_MAX;
// static NAME_MAX_MINIMUM: u32 = POSIX_NAME_MAX;
// TODO: what about PC_NAME_MAX etc?
// TODO: pathconf macro ?

pub fn uumain(args: Vec<String>) -> i32 {
    // add options
    let mut opts = Options::new();
    opts.optflag("p", "posix", "check for (most) POSIX systems");
    opts.optflag("P",
        "posix-special", "check for empty names and leading \"-\"");
    opts.optflag("",
        "portability", "check for all POSIX systems (equivalent to -p -P)");
    opts.optflag("h", "help", "display this help text and exit");
    opts.optflag("V", "version", "output version information and exit");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => { crash!(1, "{}", e) }
    };

    // set working mode
    let mode = if matches.opt_present("version") {
        Mode::Version
    } else if (matches.opt_present("posix") &&
               matches.opt_present("posix-special")) ||
              matches.opt_present("portability") {
        Mode::PosixAll
    } else if matches.opt_present("posix") {
        Mode::PosixMost
    } else if matches.opt_present("posix-special") {
        Mode::PosixSpecial
    } else {
        Mode::Help
    };

    match mode {
        Mode::Help => { help(opts); 0 }
        Mode::Version => { version(); 0 }
        _ => check_path(mode, matches.free)
    }
}

fn help(opts: Options) {
    let msg = format!("Usage: {} [OPTION]... NAME...\n\n\
    Diagnose invalid or unportable file names.", NAME);

    print!("{}", opts.usage(&msg));
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn check_path(mode: Mode, paths: Vec<String>) -> i32 {
}

fn no_leading_hyphen(path: &String) -> bool {
    !path.contains("/-") && !path.starts_with('-')
}

fn portable_chars_only(path: &String) -> bool {
    !path.contains(|c|
         !"/ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"
         .to_string().contains(c))
}
