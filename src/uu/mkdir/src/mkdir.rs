// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ugoa cmode

use clap::builder::ValueParser;
use clap::parser::ValuesRef;
use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use uucore::error::FromIo;
use uucore::error::{UResult, USimpleError};
#[cfg(not(windows))]
use uucore::mode;
use uucore::{display::Quotable, fs::dir_strip_dot_for_creation};
use uucore::{format_usage, help_about, help_section, help_usage, show, show_if_err};

static DEFAULT_PERM: u32 = 0o777;

const ABOUT: &str = help_about!("mkdir.md");
const USAGE: &str = help_usage!("mkdir.md");
const AFTER_HELP: &str = help_section!("after help", "mkdir.md");

mod options {
    pub const MODE: &str = "mode";
    pub const PARENTS: &str = "parents";
    pub const VERBOSE: &str = "verbose";
    pub const DIRS: &str = "dirs";
}

#[cfg(windows)]
fn get_mode(_matches: &ArgMatches, _mode_had_minus_prefix: bool) -> Result<u32, String> {
    Ok(DEFAULT_PERM)
}

#[cfg(not(windows))]
fn get_mode(matches: &ArgMatches, mode_had_minus_prefix: bool) -> Result<u32, String> {
    let digits: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    // Translate a ~str in octal form to u16, default to 777
    // Not tested on Windows
    let mut new_mode = DEFAULT_PERM;
    match matches.get_one::<String>(options::MODE) {
        Some(m) => {
            for mode in m.split(',') {
                if mode.contains(digits) {
                    new_mode = mode::parse_numeric(new_mode, m, true)?;
                } else {
                    let cmode = if mode_had_minus_prefix {
                        // clap parsing is finished, now put prefix back
                        format!("-{mode}")
                    } else {
                        mode.to_string()
                    };
                    new_mode = mode::parse_symbolic(new_mode, &cmode, mode::get_umask(), true)?;
                }
            }
            Ok(new_mode)
        }
        None => {
            // If no mode argument is specified return the mode derived from umask
            Ok(!mode::get_umask() & 0o0777)
        }
    }
}

#[cfg(windows)]
fn strip_minus_from_mode(_args: &mut [String]) -> bool {
    false
}

#[cfg(not(windows))]
fn strip_minus_from_mode(args: &mut Vec<String>) -> bool {
    mode::strip_minus_from_mode(args)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut args = args.collect_lossy();

    // Before we can parse 'args' with clap (and previously getopts),
    // a possible MODE prefix '-' needs to be removed (e.g. "chmod -x FILE").
    let mode_had_minus_prefix = strip_minus_from_mode(&mut args);

    // Linux-specific options, not implemented
    // opts.optflag("Z", "context", "set SELinux security context" +
    // " of each created directory to CTX"),
    let matches = uu_app().after_help(AFTER_HELP).try_get_matches_from(args)?;

    let dirs = matches
        .get_many::<OsString>(options::DIRS)
        .unwrap_or_default();
    let verbose = matches.get_flag(options::VERBOSE);
    let recursive = matches.get_flag(options::PARENTS);

    match get_mode(&matches, mode_had_minus_prefix) {
        Ok(mode) => exec(dirs, recursive, mode, verbose),
        Err(f) => Err(USimpleError::new(1, f)),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MODE)
                .short('m')
                .long(options::MODE)
                .help("set file mode (not implemented on windows)"),
        )
        .arg(
            Arg::new(options::PARENTS)
                .short('p')
                .long(options::PARENTS)
                .help("make parent directories as needed")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help("print a message for each printed directory")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIRS)
                .action(ArgAction::Append)
                .num_args(1..)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::DirPath),
        )
}

/**
 * Create the list of new directories
 */
fn exec(dirs: ValuesRef<OsString>, recursive: bool, mode: u32, verbose: bool) -> UResult<()> {
    for dir in dirs {
        // Special case to match GNU's behavior:
        // mkdir -p foo/. should work and just create foo/
        // std::fs::create_dir("foo/."); fails in pure Rust
        let path = if recursive {
            dir_strip_dot_for_creation(&PathBuf::from(dir))
        } else {
            // Normal case
            PathBuf::from(dir)
        };
        show_if_err!(mkdir(path.as_path(), recursive, mode, verbose));
    }
    Ok(())
}

fn mkdir(path: &Path, recursive: bool, mode: u32, verbose: bool) -> UResult<()> {
    create_dir(path, recursive, verbose, false)?;
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

// `is_parent` argument is not used on windows
#[allow(unused_variables)]
fn create_dir(path: &Path, recursive: bool, verbose: bool, is_parent: bool) -> UResult<()> {
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
            Some(p) => create_dir(p, recursive, verbose, true)?,
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
            #[cfg(not(windows))]
            if is_parent {
                // directories created by -p have permission bits set to '=rwx,u+wx',
                // which is umask modified by 'u+wx'
                chmod(path, (!mode::get_umask() & 0o0777) | 0o0300)?;
            }
            Ok(())
        }
        Err(_) if path.is_dir() => Ok(()),
        Err(e) => Err(e.into()),
    }
}
