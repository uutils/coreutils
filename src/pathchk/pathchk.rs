#![allow(unused_must_use)] // because we of writeln!
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
use std::fs;
use std::io::{Write, ErrorKind};

// operating mode
enum Mode {
    Default, // use filesystem to determine information and limits
    Basic,   // check basic compatibility with POSIX
    Extra,   // check for leading dashes and empty names
    Both,    // a combination of `Basic` and `Extra`
    Help,    // show help
    Version  // show version information
}

static NAME: &'static str = "pathchk";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

// a few global constants as used in the GNU implementation
static POSIX_PATH_MAX: usize = 256;
static POSIX_NAME_MAX: usize = 14;

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
    } else if matches.opt_present("help") {
        Mode::Help
    } else if (matches.opt_present("posix") &&
               matches.opt_present("posix-special")) ||
              matches.opt_present("portability") {
        Mode::Both
    } else if matches.opt_present("posix") {
        Mode::Basic
    } else if matches.opt_present("posix-special") {
        Mode::Extra
    } else {
        Mode::Default
    };

    // take necessary actions
    match mode {
        Mode::Help => { help(opts); 0 }
        Mode::Version => { version(); 0 }
        _ => {
            let mut res = true;
            if matches.free.len() == 0 {
                show_error!(
                    "missing operand\nTry {} --help for more information", NAME
                );
                res = false;
            }
            // free strings are path operands
            // FIXME: TCS, seems inefficient and overly verbose (?)
            for p in matches.free {
                let mut path = Vec::new();
                for path_segment in p.split('/') {
                    path.push(path_segment.to_string());
                }
                res &= check_path(&mode, &path);
            }
            // determine error code
            if res { 0 } else { 1 }
        }
    }
}

// print help
fn help(opts: Options) {
    let msg = format!("Usage: {} [OPTION]... NAME...\n\n\
    Diagnose invalid or unportable file names.", NAME);

    print!("{}", opts.usage(&msg));
}

// print version information
fn version() {
    println!("{} {}", NAME, VERSION);
}

// check a path, given as a slice of it's components and an operating mode
fn check_path(mode: &Mode, path: &[String]) -> bool {
    match *mode {
        Mode::Basic => check_basic(&path),
        Mode::Extra => check_default(&path) && check_extra(&path),
        Mode::Both => check_basic(&path) && check_extra(&path),
        _ => check_default(&path)
    }
}

// check a path in basic compatibility mode
fn check_basic(path: &[String]) -> bool {
    let joined_path = path.join("/");
    let total_len = joined_path.len();
    // path length
    if total_len > POSIX_PATH_MAX {
        writeln!(&mut std::io::stderr(),
            "limit {} exceeded by length {} of file name {}",
            POSIX_PATH_MAX, total_len, joined_path);
        return false;
    } else if total_len == 0 {
        writeln!(&mut std::io::stderr(), "empty file name");
        return false;
    }
    // components: character portability and length
    for p in path {
        let component_len = p.len();
        if component_len > POSIX_NAME_MAX {
            writeln!(&mut std::io::stderr(),
                "limit {} exceeded by length {} of file name component '{}'",
                POSIX_NAME_MAX, component_len, p);
            return false;
        }
        if !check_portable_chars(&p) {
            return false;
        }
    }
    // permission checks
    check_searchable(&joined_path)
}

// check a path in extra compatibility mode
fn check_extra(path: &[String]) -> bool {
    // components: leading hyphens
    for p in path {
        if !no_leading_hyphen(&p) {
            writeln!(&mut std::io::stderr(),
                "leading hyphen in file name component '{}'", p);
            return false;
        }
    }
    // path length
    if path.join("/").len() == 0 {
        writeln!(&mut std::io::stderr(), "empty file name");
        return false;
    }
    true
}

// check a path in default mode (using the file system)
fn check_default(path: &[String]) -> bool {
    let joined_path = path.join("/");
    let total_len = joined_path.len();
    // path length
    if total_len > libc::PATH_MAX as usize {
        writeln!(&mut std::io::stderr(),
            "limit {} exceeded by length {} of file name '{}'",
            libc::PATH_MAX, total_len, joined_path);
        return false;
    }
    // components: length
    for p in path {
        let component_len = p.len();
        if component_len > libc::FILENAME_MAX as usize {
            writeln!(&mut std::io::stderr(),
                "limit {} exceeded by length {} of file name component '{}'",
                libc::FILENAME_MAX, component_len, p);
            return false;
        }
    }
    // permission checks
    check_searchable(&joined_path)
}

// check whether a path is or if other problems arise
fn check_searchable(path: &String) -> bool {
    // we use lstat, just like the original implementation
    match fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(e) => if e.kind() == ErrorKind::NotFound {
            true
        } else {
            writeln!(&mut std::io::stderr(), "{}", e);
            false
        }
    }
}

// check for a hyphen at the beginning of a path segment
fn no_leading_hyphen(path_segment: &String) -> bool {
    !path_segment.starts_with('-')
}

// check whether a path segment contains only valid (read: portable) characters
fn check_portable_chars(path_segment: &String) -> bool {
    let valid_str =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"
        .to_string();
    for ch in path_segment.chars() {
        if !valid_str.contains(ch) {
            writeln!(&mut std::io::stderr(),
                "nonportable character '{}' in file name component '{}'",
                ch, path_segment);
            return false;
        }
    }
    true
}
