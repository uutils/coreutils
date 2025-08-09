// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![allow(unused_must_use)] // because we of writeln!

// spell-checker:ignore (ToDO) lstat
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Write};
use uucore::display::Quotable;
use uucore::error::{UResult, UUsageError, set_exit_code};
use uucore::format_usage;
use uucore::translate;

// operating mode
enum Mode {
    Default, // use filesystem to determine information and limits
    Basic,   // check basic compatibility with POSIX
    Extra,   // check for leading dashes and empty names
    Both,    // a combination of `Basic` and `Extra`
}

mod options {
    pub const POSIX: &str = "posix";
    pub const POSIX_SPECIAL: &str = "posix-special";
    pub const PORTABILITY: &str = "portability";
    pub const PATH: &str = "path";
}

// a few global constants as used in the GNU implementation
const POSIX_PATH_MAX: usize = 256;
const POSIX_NAME_MAX: usize = 14;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    // set working mode
    let is_posix = matches.get_flag(options::POSIX);
    let is_posix_special = matches.get_flag(options::POSIX_SPECIAL);
    let is_portability = matches.get_flag(options::PORTABILITY);

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
    let paths = matches.get_many::<OsString>(options::PATH);
    if paths.is_none() {
        return Err(UUsageError::new(
            1,
            translate!("pathchk-error-missing-operand"),
        ));
    }

    // free strings are path operands
    // FIXME: TCS, seems inefficient and overly verbose (?)
    let mut res = true;
    for p in paths.unwrap() {
        let path_str = p.to_string_lossy();
        let mut path = Vec::new();
        for path_segment in path_str.split('/') {
            path.push(path_segment.to_string());
        }
        res &= check_path(&mode, &path);
    }

    // determine error code
    if !res {
        set_exit_code(1);
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("pathchk-about"))
        .override_usage(format_usage(&translate!("pathchk-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::POSIX)
                .short('p')
                .help(translate!("pathchk-help-posix"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::POSIX_SPECIAL)
                .short('P')
                .help(translate!("pathchk-help-posix-special"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PORTABILITY)
                .long(options::PORTABILITY)
                .help(translate!("pathchk-help-portability"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PATH)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(clap::value_parser!(OsString)),
        )
}

/// check a path, given as a slice of it's components and an operating mode
fn check_path(mode: &Mode, path: &[String]) -> bool {
    match *mode {
        Mode::Basic => check_basic(path),
        Mode::Extra => check_default(path) && check_extra(path),
        Mode::Both => check_basic(path) && check_extra(path),
        Mode::Default => check_default(path),
    }
}

/// check a path in basic compatibility mode
fn check_basic(path: &[String]) -> bool {
    let joined_path = path.join("/");
    let total_len = joined_path.len();
    // path length
    if total_len > POSIX_PATH_MAX {
        writeln!(
            std::io::stderr(),
            "{}",
            translate!("pathchk-error-posix-path-length-exceeded", "limit" => POSIX_PATH_MAX, "length" => total_len, "path" => joined_path)
        );
        return false;
    } else if total_len == 0 {
        writeln!(
            std::io::stderr(),
            "{}",
            translate!("pathchk-error-empty-file-name")
        );
        return false;
    }
    // components: character portability and length
    for p in path {
        let component_len = p.len();
        if component_len > POSIX_NAME_MAX {
            writeln!(
                std::io::stderr(),
                "{}",
                translate!("pathchk-error-posix-name-length-exceeded", "limit" => POSIX_NAME_MAX, "length" => component_len, "component" => p.quote())
            );
            return false;
        }
        if !check_portable_chars(p) {
            return false;
        }
    }
    // permission checks
    check_searchable(&joined_path)
}

/// check a path in extra compatibility mode
fn check_extra(path: &[String]) -> bool {
    // components: leading hyphens
    for p in path {
        if p.starts_with('-') {
            writeln!(
                std::io::stderr(),
                "{}",
                translate!("pathchk-error-leading-hyphen", "component" => p.quote())
            );
            return false;
        }
    }
    // path length
    if path.join("/").is_empty() {
        writeln!(
            std::io::stderr(),
            "{}",
            translate!("pathchk-error-empty-file-name")
        );
        return false;
    }
    true
}

/// check a path in default mode (using the file system)
fn check_default(path: &[String]) -> bool {
    let joined_path = path.join("/");
    let total_len = joined_path.len();
    // path length
    if total_len > libc::PATH_MAX as usize {
        writeln!(
            std::io::stderr(),
            "{}",
            translate!("pathchk-error-path-length-exceeded", "limit" => libc::PATH_MAX, "length" => total_len, "path" => joined_path.quote())
        );
        return false;
    }
    if total_len == 0 {
        // Check whether a file name component is in a directory that is not searchable,
        // or has some other serious problem. POSIX does not allow "" as a file name,
        // but some non-POSIX hosts do (as an alias for "."),
        // so allow "" if `symlink_metadata` (corresponds to `lstat`) does.
        if fs::symlink_metadata(&joined_path).is_err() {
            writeln!(
                std::io::stderr(),
                "{}",
                translate!("pathchk-error-empty-path-not-found")
            );
            return false;
        }
    }

    // components: length
    for p in path {
        let component_len = p.len();
        if component_len > libc::FILENAME_MAX as usize {
            writeln!(
                std::io::stderr(),
                "{}",
                translate!("pathchk-error-name-length-exceeded", "limit" => libc::FILENAME_MAX, "length" => component_len, "component" => p.quote())
            );
            return false;
        }
    }
    // permission checks
    check_searchable(&joined_path)
}

/// check whether a path is or if other problems arise
fn check_searchable(path: &str) -> bool {
    // we use lstat, just like the original implementation
    match fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                true
            } else {
                writeln!(std::io::stderr(), "{e}");
                false
            }
        }
    }
}

/// check whether a path segment contains only valid (read: portable) characters
fn check_portable_chars(path_segment: &str) -> bool {
    const VALID_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-";
    for (i, ch) in path_segment.as_bytes().iter().enumerate() {
        if !VALID_CHARS.contains(ch) {
            let invalid = path_segment[i..].chars().next().unwrap();
            writeln!(
                std::io::stderr(),
                "{}",
                translate!("pathchk-error-nonportable-character", "character" => invalid, "component" => path_segment.quote())
            );
            return false;
        }
    }
    true
}
