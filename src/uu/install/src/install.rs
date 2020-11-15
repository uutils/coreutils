//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Ben Eills <ben@beneills.com>
//  *
//  * For the full copyright and license information, please view the LICENSE file
//  * that was distributed with this source code.

// spell-checker:ignore (ToDO) rwxr sourcepath targetpath

extern crate clap;
extern crate libc;

mod mode;

#[macro_use]
extern crate uucore;

use clap::{App, Arg, ArgMatches};
use std::fs;
use std::path::{Path, PathBuf};
use std::result::Result;

const DEFAULT_MODE: u32 = 755;

#[allow(dead_code)]
pub struct Behavior {
    main_function: MainFunction,
    specified_mode: Option<u32>,
    suffix: String,
    verbose: bool,
}

#[derive(Clone, Eq, PartialEq)]
pub enum MainFunction {
    /// Create directories
    Directory,
    /// Install files to locations (primary functionality)
    Standard,
}

impl Behavior {
    /// Determine the mode for chmod after copy.
    pub fn mode(&self) -> u32 {
        match self.specified_mode {
            Some(x) => x,
            None => DEFAULT_MODE,
        }
    }
}

static ABOUT: &str = "Copy SOURCE to DEST or multiple SOURCE(s) to the existing
 DIRECTORY, while setting permission modes and owner/group";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static OPT_COMPARE: &str = "compare";
static OPT_BACKUP: &str = "backup";
static OPT_BACKUP_2: &str = "backup2";
static OPT_DIRECTORY: &str = "directory";
static OPT_IGNORED: &str = "ignored";
static OPT_CREATED: &str = "created";
static OPT_GROUP: &str = "group";
static OPT_MODE: &str = "mode";
static OPT_OWNER: &str = "owner";
static OPT_PRESERVE_TIMESTAMPS: &str = "preserve-timestamps";
static OPT_STRIP: &str = "strip";
static OPT_STRIP_PROGRAM: &str = "strip-program";
static OPT_SUFFIX: &str = "suffix";
static OPT_TARGET_DIRECTORY: &str = "target-directory";
static OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
static OPT_VERBOSE: &str = "verbose";
static OPT_PRESERVE_CONTEXT: &str = "preserve-context";
static OPT_CONTEXT: &str = "context";

static ARG_FILES: &str = "files";

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}

/// Main install utility function, called from main.rs.
///
/// Returns a program return code.
///
pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
                Arg::with_name(OPT_BACKUP)
                .long(OPT_BACKUP)
                .help("(unimplemented) make a backup of each existing destination file")
                .value_name("CONTROL")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_BACKUP_2)
            .short("b")
            .help("(unimplemented) like --backup but does not accept an argument")
        )
        .arg(
            Arg::with_name(OPT_IGNORED)
            .short("c")
            .help("ignored")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_COMPARE)
            .short("C")
            .long(OPT_COMPARE)
            .help("(unimplemented) compare each pair of source and destination files, and in some cases, do not modify the destination at all")
        )
        .arg(
            Arg::with_name(OPT_DIRECTORY)
                .short("d")
                .long(OPT_DIRECTORY)
                .help("treat all arguments as directory names. create all components of the specified directories")
        )

        .arg(
            // TODO implement flag
            Arg::with_name(OPT_CREATED)
                .short("D")
                .help("(unimplemented) create all leading components of DEST except the last, then copy SOURCE to DEST")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_GROUP)
                .short("g")
                .long(OPT_GROUP)
                .help("(unimplemented) set group ownership, instead of process's current group")
                .value_name("GROUP")
        )
        .arg(
            Arg::with_name(OPT_MODE)
                .short("m")
                .long(OPT_MODE)
                .help("set permission mode (as in chmod), instead of rwxr-xr-x")
                .value_name("MODE")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_OWNER)
                .short("o")
                .long(OPT_OWNER)
                .help("(unimplemented) set ownership (super-user only)")
                .value_name("OWNER")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_PRESERVE_TIMESTAMPS)
                .short("p")
                .long(OPT_PRESERVE_TIMESTAMPS)
                .help("(unimplemented) apply access/modification times of SOURCE files to corresponding destination files")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_STRIP)
            .short("s")
            .long(OPT_STRIP)
            .help("(unimplemented) strip symbol tables")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_STRIP_PROGRAM)
                .long(OPT_STRIP_PROGRAM)
                .help("(unimplemented) program used to strip binaries")
                .value_name("PROGRAM")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_SUFFIX)
                .short("S")
                .long(OPT_SUFFIX)
                .help("(unimplemented) override the usual backup suffix")
                .value_name("SUFFIX")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_TARGET_DIRECTORY)
                .short("t")
                .long(OPT_TARGET_DIRECTORY)
                .help("(unimplemented) move all SOURCE arguments into DIRECTORY")
                .value_name("DIRECTORY")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_NO_TARGET_DIRECTORY)
                .short("T")
                .long(OPT_NO_TARGET_DIRECTORY)
                .help("(unimplemented) treat DEST as a normal file")

        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
            .short("v")
            .long(OPT_VERBOSE)
            .help("explain what is being done")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_PRESERVE_CONTEXT)
                .short("P")
                .long(OPT_PRESERVE_CONTEXT)
                .help("(unimplemented) preserve security context")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_CONTEXT)
                .short("Z")
                .long(OPT_CONTEXT)
                .help("(unimplemented) set security context of files and directories")
                .value_name("CONTEXT")
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true))
        .get_matches_from(args);

    let mut paths: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if let Err(s) = check_unimplemented(&matches) {
        show_error!("Unimplemented feature: {}", s);
        return 2;
    }

    let behavior = match behavior(&matches) {
        Ok(x) => x,
        Err(ret) => {
            return ret;
        }
    };

    match behavior.main_function {
        MainFunction::Directory => directory(paths, behavior),
        MainFunction::Standard => standard(paths, behavior),
    }
}

/// Check for unimplemented command line arguments.
///
/// Either return the degenerate Ok value, or an Err with string.
///
/// # Errors
///
/// Error datum is a string of the unimplemented argument.
///
///
fn check_unimplemented<'a>(matches: &ArgMatches) -> Result<(), &'a str> {
    if matches.is_present(OPT_BACKUP) {
        Err("--backup")
    } else if matches.is_present(OPT_BACKUP_2) {
        Err("-b")
    } else if matches.is_present(OPT_COMPARE) {
        Err("--compare, -C")
    } else if matches.is_present(OPT_CREATED) {
        Err("-D")
    } else if matches.is_present(OPT_GROUP) {
        Err("--group, -g")
    } else if matches.is_present(OPT_OWNER) {
        Err("--owner, -o")
    } else if matches.is_present(OPT_PRESERVE_TIMESTAMPS) {
        Err("--preserve-timestamps, -p")
    } else if matches.is_present(OPT_STRIP) {
        Err("--strip, -s")
    } else if matches.is_present(OPT_STRIP_PROGRAM) {
        Err("--strip-program")
    } else if matches.is_present(OPT_SUFFIX) {
        Err("--suffix, -S")
    } else if matches.is_present(OPT_TARGET_DIRECTORY) {
        Err("--target-directory, -t")
    } else if matches.is_present(OPT_NO_TARGET_DIRECTORY) {
        Err("--no-target-directory, -T")
    } else if matches.is_present(OPT_PRESERVE_CONTEXT) {
        Err("--preserve-context, -P")
    } else if matches.is_present(OPT_CONTEXT) {
        Err("--context, -Z")
    } else {
        Ok(())
    }
}

/// Determine behavior, given command line arguments.
///
/// If successful, returns a filled-out Behavior struct.
///
/// # Errors
///
/// In event of failure, returns an integer intended as a program return code.
///
fn behavior(matches: &ArgMatches) -> Result<Behavior, i32> {
    let main_function = if matches.is_present("directory") {
        MainFunction::Directory
    } else {
        MainFunction::Standard
    };

    let considering_dir: bool = MainFunction::Directory == main_function;

    let specified_mode: Option<u32> = if matches.is_present(OPT_MODE) {
        match matches.value_of(OPT_MODE) {
            Some(x) => match mode::parse(&x[..], considering_dir) {
                Ok(y) => Some(y),
                Err(err) => {
                    show_error!("Invalid mode string: {}", err);
                    return Err(1);
                }
            },
            None => {
                show_error!(
                    "option '--mode' requires an argument\n \
                     Try '{} --help' for more information.",
                    executable!()
                );
                return Err(1);
            }
        }
    } else {
        None
    };

    let backup_suffix = if matches.is_present(OPT_SUFFIX) {
        match matches.value_of(OPT_SUFFIX) {
            Some(x) => x,
            None => {
                show_error!(
                    "option '--suffix' requires an argument\n\
                     Try '{} --help' for more information.",
                    executable!()
                );
                return Err(1);
            }
        }
    } else {
        "~"
    };

    Ok(Behavior {
        main_function,
        specified_mode,
        suffix: backup_suffix.to_string(),
        verbose: matches.is_present(OPT_VERBOSE),
    })
}

/// Creates directories.
///
/// GNU man pages describe this functionality as creating 'all components of
/// the specified directories'.
///
/// Returns an integer intended as a program return code.
///
fn directory(paths: Vec<String>, b: Behavior) -> i32 {
    if paths.is_empty() {
        println!("{} with -d requires at least one argument.", executable!());
        1
    } else {
        let mut all_successful = true;

        for directory in paths.iter() {
            let path = Path::new(directory);

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
        if all_successful {
            0
        } else {
            1
        }
    }
}

/// Test if the path is a a new file path that can be
/// created immediately
fn is_new_file_path(path: &Path) -> bool {
    path.is_file() || !path.exists() && path.parent().map(Path::is_dir).unwrap_or(true)
}

/// Perform an install, given a list of paths and behavior.
///
/// Returns an integer intended as a program return code.
///
fn standard(paths: Vec<String>, b: Behavior) -> i32 {
    if paths.len() < 2 {
        println!("{} requires at least 2 arguments.", executable!());
        1
    } else {
        let sources = &paths[0..paths.len() - 1]
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let target = Path::new(paths.last().unwrap());

        if (target.is_file() || is_new_file_path(target)) && sources.len() == 1 {
            copy_file_to_file(&sources[0], &target.to_path_buf(), &b)
        } else {
            copy_files_into_dir(sources, &target.to_path_buf(), &b)
        }
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
fn copy_files_into_dir(files: &[PathBuf], target_dir: &PathBuf, b: &Behavior) -> i32 {
    if !target_dir.is_dir() {
        show_error!("target ‘{}’ is not a directory", target_dir.display());
        return 1;
    }

    let mut all_successful = true;
    for sourcepath in files.iter() {
        let targetpath = match sourcepath.as_os_str().to_str() {
            Some(name) => target_dir.join(name),
            None => {
                show_error!(
                    "cannot stat ‘{}’: No such file or directory",
                    sourcepath.display()
                );

                all_successful = false;
                continue;
            }
        };

        if copy(sourcepath, &targetpath, b).is_err() {
            all_successful = false;
        }
    }
    if all_successful {
        0
    } else {
        1
    }
}

/// Copy a file to another file.
///
/// Prints verbose information and error messages.
/// Returns an integer intended as a program return code.
///
/// # Parameters
///
/// _file_ must exist as a non-directory.
/// _target_ must be a non-directory
///
fn copy_file_to_file(file: &PathBuf, target: &PathBuf, b: &Behavior) -> i32 {
    if copy(file, &target, b).is_err() {
        1
    } else {
        0
    }
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
fn copy(from: &PathBuf, to: &PathBuf, b: &Behavior) -> Result<(), ()> {
    let io_result = fs::copy(from, to);

    if let Err(err) = io_result {
        show_error!(
            "install: cannot install ‘{}’ to ‘{}’: {}",
            from.display(),
            to.display(),
            err
        );
        return Err(());
    }

    if mode::chmod(&to, b.mode()).is_err() {
        return Err(());
    }

    if b.verbose {
        show_info!("'{}' -> '{}'", from.display(), to.display());
    }

    Ok(())
}
