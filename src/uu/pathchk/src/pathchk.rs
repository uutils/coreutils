#![allow(unused_must_use)] // because we of writeln!

//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Inokentiy Babushkin <inokentiy.babushkin@googlemail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE file
//  * that was distributed with this source code.

// spell-checker:ignore (ToDO) lstat

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs;
use std::io::{ErrorKind, Write};

// operating mode
enum Mode {
    Default, // use filesystem to determine information and limits
    Basic,   // check basic compatibility with POSIX
    Extra,   // check for leading dashes and empty names
    Both,    // a combination of `Basic` and `Extra`
}

static NAME: &str = "pathchk";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Check whether file names are valid or portable";

mod options {
    pub const POSIX: &str = "posix";
    pub const POSIX_SPECIAL: &str = "posix-special";
    pub const PORTABILITY: &str = "portability";
    pub const PATH: &str = "path";
}

// a few global constants as used in the GNU implementation
const POSIX_PATH_MAX: usize = 256;
const POSIX_NAME_MAX: usize = 14;

fn get_usage() -> String {
    format!("{0} [OPTION]... NAME...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(options::POSIX)
                .short("p")
                .help("check for most POSIX systems"),
        )
        .arg(
            Arg::with_name(options::POSIX_SPECIAL)
                .short("P")
                .help(r#"check for empty names and leading "-""#),
        )
        .arg(
            Arg::with_name(options::PORTABILITY)
                .long(options::PORTABILITY)
                .help("check for all POSIX systems (equivalent to -p -P)"),
        )
        .arg(Arg::with_name(options::PATH).hidden(true).multiple(true))
        .get_matches_from(args);

    // set working mode
    let is_posix = matches.values_of(options::POSIX).is_some();
    let is_posix_special = matches.values_of(options::POSIX_SPECIAL).is_some();
    let is_portability = matches.values_of(options::PORTABILITY).is_some();

    let mode = if (is_posix && is_posix_special) || is_portability {
        Mode::Both
    } else if is_posix {
        Mode::Basic
    } else if is_posix_special {
        Mode::Extra
    } else {
        Mode::Default
    };

    // take necessary actions
    let paths = matches.values_of(options::PATH);
    let mut res = if paths.is_none() {
        show_error!("missing operand\nTry {} --help for more information", NAME);
        false
    } else {
        true
    };

    if res {
        // free strings are path operands
        // FIXME: TCS, seems inefficient and overly verbose (?)
        for p in paths.unwrap() {
            let mut path = Vec::new();
            for path_segment in p.split('/') {
                path.push(path_segment.to_string());
            }
            res &= check_path(&mode, &path);
        }
    }

    // determine error code
    if res {
        0
    } else {
        1
    }
}

// check a path, given as a slice of it's components and an operating mode
fn check_path(mode: &Mode, path: &[String]) -> bool {
    match *mode {
        Mode::Basic => check_basic(&path),
        Mode::Extra => check_default(&path) && check_extra(&path),
        Mode::Both => check_basic(&path) && check_extra(&path),
        _ => check_default(&path),
    }
}

// check a path in basic compatibility mode
fn check_basic(path: &[String]) -> bool {
    let joined_path = path.join("/");
    let total_len = joined_path.len();
    // path length
    if total_len > POSIX_PATH_MAX {
        writeln!(
            &mut std::io::stderr(),
            "limit {} exceeded by length {} of file name {}",
            POSIX_PATH_MAX,
            total_len,
            joined_path
        );
        return false;
    } else if total_len == 0 {
        writeln!(&mut std::io::stderr(), "empty file name");
        return false;
    }
    // components: character portability and length
    for p in path {
        let component_len = p.len();
        if component_len > POSIX_NAME_MAX {
            writeln!(
                &mut std::io::stderr(),
                "limit {} exceeded by length {} of file name component '{}'",
                POSIX_NAME_MAX,
                component_len,
                p
            );
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
            writeln!(
                &mut std::io::stderr(),
                "leading hyphen in file name component '{}'",
                p
            );
            return false;
        }
    }
    // path length
    if path.join("/").is_empty() {
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
        writeln!(
            &mut std::io::stderr(),
            "limit {} exceeded by length {} of file name '{}'",
            libc::PATH_MAX,
            total_len,
            joined_path
        );
        return false;
    }
    // components: length
    for p in path {
        let component_len = p.len();
        if component_len > libc::FILENAME_MAX as usize {
            writeln!(
                &mut std::io::stderr(),
                "limit {} exceeded by length {} of file name component '{}'",
                libc::FILENAME_MAX,
                component_len,
                p
            );
            return false;
        }
    }
    // permission checks
    check_searchable(&joined_path)
}

// check whether a path is or if other problems arise
fn check_searchable(path: &str) -> bool {
    // we use lstat, just like the original implementation
    match fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                true
            } else {
                writeln!(&mut std::io::stderr(), "{}", e);
                false
            }
        }
    }
}

// check for a hyphen at the beginning of a path segment
fn no_leading_hyphen(path_segment: &str) -> bool {
    !path_segment.starts_with('-')
}

// check whether a path segment contains only valid (read: portable) characters
fn check_portable_chars(path_segment: &str) -> bool {
    let valid_str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-".to_string();
    for ch in path_segment.chars() {
        if !valid_str.contains(ch) {
            writeln!(
                &mut std::io::stderr(),
                "nonportable character '{}' in file name component '{}'",
                ch,
                path_segment
            );
            return false;
        }
    }
    true
}
