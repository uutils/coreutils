#![crate_name = "uu_install"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Ben Eills <ben@beneills.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

mod mode;

#[macro_use]
extern crate uucore;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::result::Result;

static NAME: &'static str = "install";
static SUMMARY: &'static str = "Copy SOURCE to DEST or multiple SOURCE(s) to the existing
 DIRECTORY, while setting permission modes and owner/group";
static LONG_HELP: &'static str = "";

const DEFAULT_MODE: u32 = 755;

#[allow(dead_code)]
pub struct Behaviour {
    main_function: MainFunction,
    specified_mode: Option<u32>,
    suffix: String,
    verbose: bool
}

#[derive(Clone, Eq, PartialEq)]
pub enum MainFunction {
    /// Create directories
    Directory,
    /// Install files to locations (primary functionality)
    Standard
}

impl Behaviour {
    /// Determine the mode for chmod after copy.
    pub fn mode(&self) -> u32 {
        match self.specified_mode {
            Some(x) => x,
            None => DEFAULT_MODE
        }
    }
}

/// Main install utility function, called from main.rs.
///
/// Returns a program return code.
///
pub fn uumain(args: Vec<String>) -> i32 {
    let matches = parse_opts(args);

    if let Err(s) = check_unimplemented(&matches) {
        show_error!("Unimplemented feature: {}", s);
        return 2;
    }

    let behaviour = match behaviour(&matches) {
        Ok(x) => x,
        Err(ret) => {
            return ret;
        }
    };

    let paths: Vec<PathBuf> = {
        fn string_to_path<'a>(s: &'a String) -> &'a Path {
            Path::new(s)
        };
        let to_owned = |p: &Path| p.to_owned();
        let arguments = matches.free.iter().map(string_to_path);

        arguments.map(to_owned).collect()
    };

    match behaviour.main_function {
        MainFunction::Directory => {
            directory(&paths[..], behaviour)
        },
        MainFunction::Standard => {
            standard(&paths[..], behaviour)
        }
    }
}

/// Build a specification of the command line.
///
/// Returns a getopts::Options struct.
///
fn parse_opts(args: Vec<String>) -> getopts::Matches {
    let syntax = format!("SOURCE DEST
 {} SOURCE... DIRECTORY", NAME);
     new_coreopts!(&syntax, SUMMARY, LONG_HELP)
    // TODO implement flag
        .optflagopt("",  "backup", "(unimplemented) make a backup of each existing destination\n \
                                    file", "CONTROL")
    // TODO implement flag
        .optflag("b", "", "(unimplemented) like --backup but does not accept an argument")
    // TODO implement flag
        .optflag("C", "compare", "(unimplemented) compare each pair of source and destination\n \
                                  files, and in some cases, do not modify the destination at all")
        .optflag("d", "directory", "treat all arguments as directory names\n \
                                    create all components of the specified directories")
    // TODO implement flag
        .optflag("D", "", "(unimplemented) create all leading components of DEST except the\n \
                           last, then copy SOURCE to DEST")
    // TODO implement flag
        .optflagopt("g", "group", "(unimplemented) set group ownership, instead of process'\n \
                                   current group", "GROUP")
        .optflagopt("m", "mode", "set permission mode (as in chmod), instead\n \
                                  of rwxr-xr-x", "MODE")
    // TODO implement flag
        .optflagopt("o", "owner", "(unimplemented) set ownership (super-user only)",
                    "OWNER")
    // TODO implement flag
        .optflag("p", "preserve-timestamps", "(unimplemented) apply access/modification times\n \
                                              of SOURCE files to corresponding destination files")
    // TODO implement flag
        .optflag("s", "strip", "(unimplemented) strip symbol tables")
    // TODO implement flag
        .optflagopt("", "strip-program", "(unimplemented) program used to strip binaries",
                    "PROGRAM")
    // TODO implement flag
        .optopt("S", "suffix", "(unimplemented) override the usual backup suffix", "SUFFIX")
    // TODO implement flag
        .optopt("t", "target-directory", "(unimplemented) move all SOURCE arguments into\n \
                                          DIRECTORY", "DIRECTORY")
    // TODO implement flag
        .optflag("T", "no-target-directory", "(unimplemented) treat DEST as a normal file")
    // TODO implement flag
        .optflag("v", "verbose", "(unimplemented) explain what is being done")
    // TODO implement flag
        .optflag("P", "preserve-context", "(unimplemented) preserve security context")
    // TODO implement flag
        .optflagopt("Z", "context", "(unimplemented) set security context of files and\n \
                                     directories", "CONTEXT")
        .parse(args)
}

/// Check for unimplemented command line arguments.
///
/// Either return the degenerate Ok value, or an Err with string.
///
/// # Errors
///
/// Error datum is a string of the unimplemented argument.
///
fn check_unimplemented(matches: &getopts::Matches) -> Result<(), &str> {
    if matches.opt_present("backup") {
        Err("--backup")
    } else if matches.opt_present("b") {
        Err("-b")
    } else if matches.opt_present("compare") {
        Err("--compare, -C")
    } else if matches.opt_present("D") {
        Err("-D")
    } else if matches.opt_present("group") {
        Err("--group, -g")
    } else if matches.opt_present("owner") {
        Err("--owner, -o")
    } else if matches.opt_present("preserve-timestamps") {
        Err("--preserve-timestamps, -p")
    } else if matches.opt_present("strip") {
        Err("--strip, -s")
    } else if matches.opt_present("strip-program") {
        Err("--strip-program")
    } else if matches.opt_present("suffix") {
        Err("--suffix, -S")
    } else if matches.opt_present("target-directory") {
        Err("--target-directory, -t")
    } else if matches.opt_present("no-target-directory") {
        Err("--no-target-directory, -T")
    } else if matches.opt_present("verbose") {
        Err("--verbose, -v")
    } else if matches.opt_present("preserve-context") {
        Err("--preserve-context, -P")
    } else if matches.opt_present("context") {
        Err("--context, -Z")
    } else {
        Ok(())
    }
}

/// Determine behaviour, given command line arguments.
///
/// If successful, returns a filled-out Behaviour struct.
///
/// # Errors
///
/// In event of failure, returns an integer intended as a program return code.
///
fn behaviour(matches: &getopts::Matches) -> Result<Behaviour, i32> {
    let main_function = if matches.opt_present("directory") {
        MainFunction::Directory
    } else {
        MainFunction::Standard
    };

    let considering_dir: bool = MainFunction::Directory == main_function;

    let specified_mode: Option<u32> = if matches.opt_present("mode") {
        match matches.opt_str("mode") {
            Some(x) => {
                match mode::parse(&x[..], considering_dir) {
                    Ok(y) => Some(y),
                    Err(err) => {
                        show_error!("Invalid mode string: {}", err);
                        return Err(1);
                    }
                }
            },
            None => {
                show_error!("option '--mode' requires an argument\n \
                            Try '{} --help' for more information.", NAME);
                return Err(1);
            }
        }
    } else {
        None
    };

    let backup_suffix = if matches.opt_present("suffix") {
        match matches.opt_str("suffix") {
            Some(x) => x,
            None => {
                show_error!("option '--suffix' requires an argument\n\
                            Try '{} --help' for more information.", NAME);
                return Err(1);
            }
        }
    } else {
        "~".to_owned()
    };

    Ok(Behaviour {
        main_function: main_function,
        specified_mode: specified_mode,
        suffix: backup_suffix,
        verbose: matches.opt_present("v"),
    })
}

/// Creates directories.
///
/// GNU man pages describe this functionality as creating 'all components of
/// the specified directories'.
///
/// Returns an integer intended as a program return code.
///
fn directory(paths: &[PathBuf], b: Behaviour) -> i32 {
    if paths.len() < 1 {
        println!("{} with -d requires at least one argument.", NAME);
        1
    } else {
        let mut all_successful = true;

        for directory in paths.iter() {
            let path = directory.as_path();

            if path.exists() {
                show_info!("cannot create directory '{}': File exists", path.display());
                all_successful = false;
            }

            if let Err(e) = fs::create_dir(directory) {
                show_info!("{}: {}", path.display(), e.to_string());
                all_successful = false;
            }

            if mode::chmod(&path, b.mode()).is_err() {
                all_successful = false;
            }

            if b.verbose {
                show_info!("created directory '{}'", path.display());
            }
        }
        if all_successful { 0 } else { 1 }
    }
}

/// Perform an install, given a list of paths and behaviour.
///
/// Returns an integer intended as a program return code.
///
fn standard(paths: &[PathBuf], b: Behaviour) -> i32 {
    if paths.len() < 2 {
        println!("{} requires at least 2 arguments.", NAME);
        1
    } else {
        let sources = &paths[0..paths.len() - 1];
        let target_directory = &paths[paths.len() - 1];

        copy_files_into_dir(sources, target_directory, &b)
    }
}

/// Copy some files into a directory.
///
/// Prints verbose information and error messages.
/// Returns an integer intended as a program return code.
///
/// # Parameters
///
/// _files_ must all exist as non-directories.
/// _target_dir_ must be a directory.
///
fn copy_files_into_dir(files: &[PathBuf], target_dir: &PathBuf, b: &Behaviour) -> i32 {
    if !target_dir.is_dir() {
        show_error!("target ‘{}’ is not a directory", target_dir.display());
        return 1;
    }

    let mut all_successful = true;
    for sourcepath in files.iter() {
        let targetpath = match sourcepath.as_os_str().to_str() {
            Some(name) => target_dir.join(name),
            None => {
                show_error!("cannot stat ‘{}’: No such file or directory",
                            sourcepath.display());

                all_successful = false;
                continue;
            }
        };

        if copy(sourcepath, &targetpath, b).is_err() {
            all_successful = false;
        }
    };
    if all_successful { 0 } else { 1 }
}

/// Copy one file to a new location, changing metadata.
///
/// # Parameters
///
/// _from_ must exist as a non-directory.
/// _to_ must be a non-existent file, whose parent directory exists.
///
/// # Errors
///
/// If the copy system call fails, we print a verbose error and return an empty error value.
///
fn copy(from: &PathBuf, to: &PathBuf, b: &Behaviour) -> Result<(), ()> {
    let io_result = fs::copy(from, to);

    if let Err(err) = io_result {
        show_error!("install: cannot install ‘{}’ to ‘{}’: {}",
                    from.display(), to.display(), err);
        return Err(());
    }

    if mode::chmod(&to, b.mode()).is_err() {
        return Err(());
    }

    if b.verbose {
        print!("‘{}’ -> ‘{}’", from.display(), to.display());
    }

    Ok(())
}
