//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Nicholas Juszczak <juszczakn@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::OsValues;
use clap::{crate_version, App, Arg};
use std::fs;
use std::path::Path;
use uucore::error::{FromIo, UResult, USimpleError};

static ABOUT: &str = "Create the given DIRECTORY(ies) if they do not exist";
mod options {
    pub const MODE: &str = "mode";
    pub const PARENTS: &str = "parents";
    pub const VERBOSE: &str = "verbose";
    pub const DIRS: &str = "dirs";
}

fn get_usage() -> String {
    format!("{0} [OPTION]... [USER]", executable!())
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = get_usage();

    // Linux-specific options, not implemented
    // opts.optflag("Z", "context", "set SELinux security context" +
    // " of each created directory to CTX"),
    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let dirs = matches.values_of_os(options::DIRS).unwrap_or_default();
    let verbose = matches.is_present(options::VERBOSE);
    let recursive = matches.is_present(options::PARENTS);

    // Translate a ~str in octal form to u16, default to 755
    // Not tested on Windows
    let mode: u16 = match matches.value_of(options::MODE) {
        Some(m) => u16::from_str_radix(m, 8)
            .map_err(|_| USimpleError::new(1, format!("invalid mode '{}'", m)))?,
        None => 0o755_u16,
    };

    exec(dirs, recursive, mode, verbose)
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(util_name!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::MODE)
                .short("m")
                .long(options::MODE)
                .help("set file mode (not implemented on windows)")
                .default_value("755"),
        )
        .arg(
            Arg::with_name(options::PARENTS)
                .short("p")
                .long(options::PARENTS)
                .alias("parent")
                .help("make parent directories as needed"),
        )
        .arg(
            Arg::with_name(options::VERBOSE)
                .short("v")
                .long(options::VERBOSE)
                .help("print a message for each printed directory"),
        )
        .arg(
            Arg::with_name(options::DIRS)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
}

/**
 * Create the list of new directories
 */
fn exec(dirs: OsValues, recursive: bool, mode: u16, verbose: bool) -> UResult<()> {
    for dir in dirs {
        let path = Path::new(dir);
        show_if_err!(mkdir(path, recursive, mode, verbose));
    }
    Ok(())
}

fn mkdir(path: &Path, recursive: bool, mode: u16, verbose: bool) -> UResult<()> {
    let create_dir = if recursive {
        fs::create_dir_all
    } else {
        fs::create_dir
    };

    create_dir(path).map_err_context(|| format!("cannot create directory '{}'", path.display()))?;

    if verbose {
        println!("{}: created directory '{}'", executable!(), path.display());
    }

    chmod(path, mode)
}

#[cfg(any(unix, target_os = "redox"))]
fn chmod(path: &Path, mode: u16) -> UResult<()> {
    use std::fs::{set_permissions, Permissions};
    use std::os::unix::fs::PermissionsExt;

    let mode = Permissions::from_mode(u32::from(mode));

    set_permissions(path, mode)
        .map_err_context(|| format!("cannot set permissions '{}'", path.display()))
}

#[cfg(windows)]
fn chmod(_path: &Path, _mode: u16) -> UResult<()> {
    // chmod on Windows only sets the readonly flag, which isn't even honored on directories
    Ok(())
}
