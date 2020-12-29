//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Nicholas Juszczak <juszczakn@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs;
use std::path::Path;

static ABOUT: &str = "Create the given DIRECTORY(ies) if they do not exist";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static OPT_MODE: &str = "mode";
static OPT_PARENTS: &str = "parents";
static OPT_VERBOSE: &str = "verbose";

static ARG_DIRS: &str = "dirs";

fn get_usage() -> String {
    format!("{0} [OPTION]... [USER]", executable!())
}

/**
 * Handles option parsing
 */
pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    // Linux-specific options, not implemented
    // opts.optflag("Z", "context", "set SELinux security context" +
    // " of each created directory to CTX"),
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_MODE)
                .short("m")
                .long(OPT_MODE)
                .help("set file mode")
                .default_value("755"),
        )
        .arg(
            Arg::with_name(OPT_PARENTS)
                .short("p")
                .long(OPT_PARENTS)
                .alias("parent")
                .help("make parent directories as needed"),
        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
                .short("v")
                .long(OPT_VERBOSE)
                .help("print a message for each printed directory"),
        )
        .arg(
            Arg::with_name(ARG_DIRS)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
        .get_matches_from(args);

    let dirs: Vec<String> = matches
        .values_of(ARG_DIRS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let verbose = matches.is_present(OPT_VERBOSE);
    let recursive = matches.is_present(OPT_PARENTS);

    // Translate a ~str in octal form to u16, default to 755
    // Not tested on Windows
    let mode_match = matches.value_of(OPT_MODE);
    let mode: u16 = match mode_match {
        Some(m) => {
            let res: Option<u16> = u16::from_str_radix(&m, 8).ok();
            match res {
                Some(r) => r,
                _ => crash!(1, "no mode given"),
            }
        }
        _ => 0o755_u16,
    };

    exec(dirs, recursive, mode, verbose)
}

/**
 * Create the list of new directories
 */
fn exec(dirs: Vec<String>, recursive: bool, mode: u16, verbose: bool) -> i32 {
    let mut status = 0;
    let empty = Path::new("");
    for dir in &dirs {
        let path = Path::new(dir);
        if !recursive {
            if let Some(parent) = path.parent() {
                if parent != empty && !parent.exists() {
                    show_info!(
                        "cannot create directory '{}': No such file or directory",
                        path.display()
                    );
                    status = 1;
                    continue;
                }
            }
        }
        status |= mkdir(path, recursive, mode, verbose);
    }
    status
}

/**
 * Wrapper to catch errors, return 1 if failed
 */
fn mkdir(path: &Path, recursive: bool, mode: u16, verbose: bool) -> i32 {
    let create_dir = if recursive {
        fs::create_dir_all
    } else {
        fs::create_dir
    };
    if let Err(e) = create_dir(path) {
        show_info!("{}: {}", path.display(), e.to_string());
        return 1;
    }

    if verbose {
        println!("{}: created directory '{}'", executable!(), path.display());
    }

    #[cfg(any(unix, target_os = "redox"))]
    fn chmod(path: &Path, mode: u16) -> i32 {
        use std::fs::{set_permissions, Permissions};
        use std::os::unix::fs::PermissionsExt;

        let mode = Permissions::from_mode(u32::from(mode));

        if let Err(err) = set_permissions(path, mode) {
            show_error!("{}: {}", path.display(), err);
            return 1;
        }
        0
    }
    #[cfg(windows)]
    #[allow(unused_variables)]
    fn chmod(path: &Path, mode: u16) -> i32 {
        // chmod on Windows only sets the readonly flag, which isn't even honored on directories
        0
    }
    chmod(path, mode)
}
