#![crate_name = "uu_mkdir"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Nicholas Juszczak <juszczakn@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

static NAME: &'static str = "mkdir";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

/**
 * Handles option parsing
 */
pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    // Linux-specific options, not implemented
    // opts.optflag("Z", "context", "set SELinux security context" +
    // " of each created directory to CTX"),
    opts.optopt("m", "mode", "set file mode", "755");
    opts.optflag("p", "parents", "make parent directories as needed");
    opts.optflag("v", "verbose", "print a message for each printed directory");
    opts.optflag("h", "help", "display this help");
    opts.optflag("V", "version", "display this version");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };

    if args.len() == 1 || matches.opt_present("help") {
        print_help(&opts);
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }
    let verbose = matches.opt_present("verbose");
    let recursive = matches.opt_present("parents");

    // Translate a ~str in octal form to u16, default to 755
    // Not tested on Windows
    let mode_match = matches.opts_str(&["mode".to_owned()]);
    let mode: u16 = if mode_match.is_some() {
        let m = mode_match.unwrap();
        let res: Option<u16> = u16::from_str_radix(&m, 8).ok();
        if res.is_some() {
            res.unwrap()
        } else {
            crash!(1, "no mode given");
        }
    } else {
        0o755 as u16
    };

    let dirs = matches.free;
    if dirs.is_empty() {
        crash!(1, "missing operand");
    }
    exec(dirs, recursive, mode, verbose)
}

fn print_help(opts: &getopts::Options) {
    println!("{} {}", NAME, VERSION);
    println!("");
    println!("Usage:");
    print!("{}", opts.usage("Create the given DIRECTORY(ies) if they do not exist"));
}

/**
 * Create the list of new directories
 */
fn exec(dirs: Vec<String>, recursive: bool, mode: u16, verbose: bool) -> i32 {
    let mut status = 0;
    let empty = Path::new("");
    for dir in &dirs {
        let path = Path::new(dir);
        if recursive {
            let mut pathbuf = PathBuf::new();
            for component in path.components() {
                pathbuf.push(component.as_os_str());
                if !path.is_dir() {
                    status |= mkdir(pathbuf.as_path(), mode, verbose);
                }
            }
        } else {
            match path.parent() {
                Some(parent) => {
                    if parent != empty && !parent.exists() {
                        show_info!("cannot create directory '{}': No such file or directory", path.display());
                        status = 1;
                    } else {
                        status |= mkdir(path, mode, verbose);
                    }
                },
                None => {
                    status |= mkdir(path, mode, verbose);
                }
            }
        }
    }
    status
}

/**
 * Wrapper to catch errors, return 1 if failed
 */
fn mkdir(path: &Path, mode: u16, verbose: bool) -> i32 {
    if let Err(e) = fs::create_dir(path) {
        show_info!("{}: {}", path.display(), e.to_string());
        return 1;
    }

    if verbose {
        show_info!("created directory '{}'", path.display());
    }

    #[cfg(unix)]
    fn chmod(path: &Path, mode: u16) -> i32 {
        use std::ffi::CString;
        use std::io::Error;

        let directory = CString::new(path.as_os_str().to_str().unwrap()).unwrap_or_else(|e| crash!(1, "{}", e));
        let mode = mode as libc::mode_t;

        if unsafe { libc::chmod(directory.as_ptr(), mode) } != 0 {
            show_info!("{}: errno {}", path.display(), Error::last_os_error().raw_os_error().unwrap());
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
