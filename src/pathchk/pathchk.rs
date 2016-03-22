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
use std::io::Write;

// operating mode
enum Mode {
    Basic,
    Extra,
    Both,
    Help,
    Version
}

static NAME: &'static str = "pathchk";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

// a few global constants as used in the GNU implememntation
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
    } else if (matches.opt_present("posix") &&
               matches.opt_present("posix-special")) ||
              matches.opt_present("portability") {
        Mode::Both
    } else if matches.opt_present("posix") {
        Mode::Basic
    } else if matches.opt_present("posix-special") {
        Mode::Extra
    } else {
        Mode::Help
    };

    // take necessary actions
    match mode {
        Mode::Help => { help(opts); 0 }
        Mode::Version => { version(); 0 }
        _ => {
            let mut res = true;
            for p in matches.free {
                let mut path = Vec::with_capacity(p.len());
                for path_segment in p.split('/') {
                    path.push(path_segment.to_string());
                }
                res &= check_path(&mode, &path);
            }
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

// check a path, given as a slice of it's components
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
    let mut char_num = 0;
    let joined_path = path.join("/");
    if joined_path.len() > POSIX_PATH_MAX {
        writeln!(&mut std::io::stderr(),
            "limit {} exceeded by length {} of file name {}",
            POSIX_PATH_MAX, joined_path.len(), joined_path);
        return false;
    }
    for p in path {
        char_num += p.len();
        if p.len() > POSIX_NAME_MAX {
            writeln!(&mut std::io::stderr(),
                "limit {} exceeded by length {} of file name component {}",
                POSIX_NAME_MAX, p.len(), p);
            return false;
        }
        match portable_chars_only(&p) {
            Some(ch) => {
                writeln!(&mut std::io::stderr(),
                    "nonportable character '{}' in file name '{}'",
                    ch, joined_path);
                return false;
            }
            None => continue
        }
    }
    if char_num == 0 {
        writeln!(&mut std::io::stderr(), "empty file name");
        return false;
    }
    true
}

// check a path in extra compatibility mode
fn check_extra(path: &[String]) -> bool {
    let mut char_num = 0;
    for p in path {
        char_num += p.len();
        if !no_leading_hyphen(&p) {
            writeln!(&mut std::io::stderr(),
                "leading hyphen in path segment '{}'", p);
            return false;
        }
    }
    if char_num == 0 {
        writeln!(&mut std::io::stderr(), "empty file name");
        return false;
    }
    true
}

// check a path in default mode (using the file system)
fn check_default(path: &[String]) -> bool {
    // TODO: lines 288-296
    // get PATH_MAX here
    // get NAME_MAX here
    true
}

// check for a hypthen at the beginning of a path segment
fn no_leading_hyphen(path_segment: &String) -> bool {
    !path_segment.starts_with('-')
}

// check whether a path segment contains only valid (read: portable) characters
fn portable_chars_only(path_segment: &String) -> Option<char> {
    let valid_str =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"
        .to_string();
    for ch in path_segment.chars() {
        if !valid_str.contains(ch) {
            return Some(ch);
        }
    }
    None
}
