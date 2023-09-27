// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) lstat
use clap::{crate_version, Arg, ArgAction, Command};
use std::fs;
use std::io::ErrorKind;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::{format_usage, help_about, help_usage, show, show_if_err};

// operating mode
enum Mode {
    Default, // use filesystem to determine information and limits
    Basic,   // check basic compatibility with POSIX
    Extra,   // check for leading dashes and empty names
    Both,    // a combination of `Basic` and `Extra`
}

const ABOUT: &str = help_about!("pathchk.md");
const USAGE: &str = help_usage!("pathchk.md");

mod options {
    pub const POSIX: &str = "posix";
    pub const POSIX_SPECIAL: &str = "posix-special";
    pub const PORTABILITY: &str = "portability";
    pub const PATH: &str = "path";
}

// a few global constants as used in the GNU implementation
const POSIX_PATH_MAX: usize = 255;
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
    let paths = matches.get_many::<String>(options::PATH);
    if paths.is_none() {
        return Err(UUsageError::new(1, "missing operand"));
    }

    // free strings are path operands
    // FIXME: TCS, seems inefficient and overly verbose (?)
    for p in paths.unwrap() {
        let mut path = Vec::new();
        for path_segment in p.split('/') {
            path.push(path_segment.to_string());
        }
        show_if_err!(check_path(&mode, &path));
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::POSIX)
                .short('p')
                .help("check for most POSIX systems")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::POSIX_SPECIAL)
                .short('P')
                .help(r#"check for empty names and leading "-""#)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PORTABILITY)
                .long(options::PORTABILITY)
                .help("check for all POSIX systems (equivalent to -p -P)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PATH)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}

// check a path, given as a slice of it's components and an operating mode
fn check_path(mode: &Mode, path: &[String]) -> UResult<()> {
    match *mode {
        Mode::Basic => check_basic(path),
        Mode::Extra => check_default(path).and_then(|_| check_extra(path)),
        Mode::Both => check_basic(path).and_then(|_| check_extra(path)),
        Mode::Default => check_default(path),
    }
}

// check a path in basic compatibility mode
fn check_basic(path: &[String]) -> UResult<()> {
    let joined_path = path.join("/");
    let total_len = joined_path.len();
    // path length
    if total_len > POSIX_PATH_MAX {
        return Err(USimpleError::new(
            1,
            format!(
                "limit {POSIX_PATH_MAX} exceeded by length {total_len} of file name {joined_path}"
            ),
        ));
    } else if total_len == 0 {
        return Err(USimpleError::new(1, "empty file name"));
    }
    // components: character portability and length
    for p in path {
        let component_len = p.len();
        if component_len > POSIX_NAME_MAX {
            return Err(USimpleError::new(
                1,
                format!(
                    "limit {} exceeded by length {} of file name component {}",
                    POSIX_NAME_MAX,
                    component_len,
                    p.quote()
                ),
            ));
        }
        check_portable_chars(p)?;
    }
    // permission checks
    check_searchable(&joined_path)
}

// check a path in extra compatibility mode
fn check_extra(path: &[String]) -> UResult<()> {
    // components: leading hyphens
    for p in path {
        if p.starts_with('-') {
            return Err(USimpleError::new(
                1,
                format!("leading '-' in a component of file name {}", p.quote()),
            ));
        }
    }
    // path length
    if path.join("/").is_empty() {
        return Err(USimpleError::new(1, "empty file name"));
    }
    Ok(())
}

// check a path in default mode (using the file system)
fn check_default(path: &[String]) -> UResult<()> {
    let joined_path = path.join("/");
    let total_len = joined_path.len();
    // path length
    if total_len > libc::PATH_MAX as usize {
        return Err(USimpleError::new(
            1,
            format!(
                "limit {} exceeded by length {} of file name {}",
                libc::PATH_MAX,
                total_len,
                joined_path.quote()
            ),
        ));
    }
    if total_len == 0 {
        // Check whether a file name component is in a directory that is not searchable,
        // or has some other serious problem. POSIX does not allow "" as a file name,
        // but some non-POSIX hosts do (as an alias for "."),
        // so allow "" if `symlink_metadata` (corresponds to `lstat`) does.
        if fs::symlink_metadata(&joined_path).is_err() {
            writeln!(std::io::stderr(), "pathchk: '': No such file or directory",);
            return false;
        }
    }

    // components: length
    for p in path {
        let component_len = p.len();
        if component_len > libc::FILENAME_MAX as usize {
            return Err(USimpleError::new(
                1,
                format!(
                    "limit {} exceeded by length {} of file name component {}",
                    libc::FILENAME_MAX,
                    component_len,
                    p.quote()
                ),
            ));
        }
    }

    // permission checks
    check_searchable(&joined_path)
}

// check whether a path is or if other problems arise
fn check_searchable(path: &str) -> UResult<()> {
    // we use lstat, just like the original implementation
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                Ok(())
            } else {
                Err(USimpleError::new(1, e.to_string()))
            }
        }
    }
}

// check whether a path segment contains only valid (read: portable) characters
fn check_portable_chars(path_segment: &str) -> UResult<()> {
    const VALID_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-";
    for (i, ch) in path_segment.as_bytes().iter().enumerate() {
        if !VALID_CHARS.contains(ch) {
            let invalid = path_segment[i..].chars().next().unwrap();
            return Err(USimpleError::new(
                1,
                format!(
                    "nonportable character '{}' in file name {}",
                    invalid,
                    path_segment.quote()
                ),
            ));
        }
    }
    Ok(())
}
