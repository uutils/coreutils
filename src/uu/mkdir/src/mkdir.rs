//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Nicholas Juszczak <juszczakn@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) ugoa cmode

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, ArgMatches, Command, OsValues};
use std::path::Path;
use uucore::display::Quotable;
#[cfg(not(windows))]
use uucore::error::FromIo;
use uucore::error::{UResult, USimpleError};
#[cfg(not(windows))]
use uucore::mode;
use uucore::{format_usage, InvalidEncodingHandling};

static DEFAULT_PERM: u32 = 0o755;

static ABOUT: &str = "Create the given DIRECTORY(ies) if they do not exist";
const USAGE: &str = "{} [OPTION]... [USER]";

mod options {
    pub const MODE: &str = "mode";
    pub const PARENTS: &str = "parents";
    pub const VERBOSE: &str = "verbose";
    pub const DIRS: &str = "dirs";
}

fn get_long_usage() -> String {
    String::from("Each MODE is of the form '[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+'.")
}

#[cfg(windows)]
fn get_mode(_matches: &ArgMatches, _mode_had_minus_prefix: bool) -> Result<u32, String> {
    Ok(DEFAULT_PERM)
}

#[cfg(not(windows))]
fn get_mode(matches: &ArgMatches, mode_had_minus_prefix: bool) -> Result<u32, String> {
    let digits: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    // Translate a ~str in octal form to u16, default to 755
    // Not tested on Windows
    let mut new_mode = DEFAULT_PERM;
    match matches.value_of(options::MODE) {
        Some(m) => {
            for mode in m.split(',') {
                if mode.contains(digits) {
                    new_mode = mode::parse_numeric(new_mode, m, true)?;
                } else {
                    let cmode = if mode_had_minus_prefix {
                        // clap parsing is finished, now put prefix back
                        format!("-{}", mode)
                    } else {
                        mode.to_string()
                    };
                    new_mode = mode::parse_symbolic(new_mode, &cmode, mode::get_umask(), true)?;
                }
            }
            Ok(new_mode)
        }
        None => Ok(DEFAULT_PERM),
    }
}

#[cfg(windows)]
fn strip_minus_from_mode(_args: &mut Vec<String>) -> bool {
    false
}

#[cfg(not(windows))]
fn strip_minus_from_mode(args: &mut Vec<String>) -> bool {
    mode::strip_minus_from_mode(args)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    // Before we can parse 'args' with clap (and previously getopts),
    // a possible MODE prefix '-' needs to be removed (e.g. "chmod -x FILE").
    let mode_had_minus_prefix = strip_minus_from_mode(&mut args);
    let after_help = get_long_usage();

    // Linux-specific options, not implemented
    // opts.optflag("Z", "context", "set SELinux security context" +
    // " of each created directory to CTX"),
    let matches = uu_app().after_help(&after_help[..]).get_matches_from(args);

    let dirs = matches.values_of_os(options::DIRS).unwrap_or_default();
    let verbose = matches.is_present(options::VERBOSE);
    let recursive = matches.is_present(options::PARENTS);

    match get_mode(&matches, mode_had_minus_prefix) {
        Ok(mode) => exec(dirs, recursive, mode, verbose),
        Err(f) => Err(USimpleError::new(1, f)),
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MODE)
                .short('m')
                .long(options::MODE)
                .help("set file mode (not implemented on windows)")
                .default_value("755"),
        )
        .arg(
            Arg::new(options::PARENTS)
                .short('p')
                .long(options::PARENTS)
                .help("make parent directories as needed"),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help("print a message for each printed directory"),
        )
        .arg(
            Arg::new(options::DIRS)
                .multiple_occurrences(true)
                .takes_value(true)
                .min_values(1)
                .allow_invalid_utf8(true),
        )
}

/**
 * Create the list of new directories
 */
fn exec(dirs: OsValues, recursive: bool, mode: u32, verbose: bool) -> UResult<()> {
    for dir in dirs {
        let path = Path::new(dir);
        show_if_err!(mkdir(path, recursive, mode, verbose));
    }
    Ok(())
}

fn mkdir(path: &Path, recursive: bool, mode: u32, verbose: bool) -> UResult<()> {
    create_dir(path, recursive, verbose)?;
    chmod(path, mode)
}

#[cfg(any(unix, target_os = "redox"))]
fn chmod(path: &Path, mode: u32) -> UResult<()> {
    use std::fs::{set_permissions, Permissions};
    use std::os::unix::fs::PermissionsExt;

    let mode = Permissions::from_mode(mode);

    set_permissions(path, mode)
        .map_err_context(|| format!("cannot set permissions {}", path.quote()))
}

#[cfg(windows)]
fn chmod(_path: &Path, _mode: u32) -> UResult<()> {
    // chmod on Windows only sets the readonly flag, which isn't even honored on directories
    Ok(())
}

fn create_dir(path: &Path, recursive: bool, verbose: bool) -> UResult<()> {
    if path.exists() && !recursive {
        return Err(USimpleError::new(
            1,
            format!("{}: File exists", path.display()),
        ));
    }
    if path == Path::new("") {
        return Ok(());
    }

    if recursive {
        match path.parent() {
            Some(p) => create_dir(p, recursive, verbose)?,
            None => {
                USimpleError::new(1, "failed to create whole tree");
            }
        }
    }

    match std::fs::create_dir(path) {
        Ok(()) => {
            if verbose {
                println!(
                    "{}: created directory {}",
                    uucore::util_name(),
                    path.quote()
                );
            }
            Ok(())
        }
        Err(_) if path.is_dir() => Ok(()),
        Err(e) => Err(e.into()),
    }
}
