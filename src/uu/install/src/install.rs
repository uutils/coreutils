//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Ben Eills <ben@beneills.com>
//  *
//  * For the full copyright and license information, please view the LICENSE file
//  * that was distributed with this source code.

// spell-checker:ignore (ToDO) rwxr sourcepath targetpath

mod mode;

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg, ArgMatches};
use file_diff::diff;
use filetime::{set_file_times, FileTime};
use uucore::entries::{grp2gid, usr2uid};
use uucore::perms::{wrap_chgrp, wrap_chown, Verbosity};

use libc::{getegid, geteuid};
use std::fs;
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::result::Result;

const DEFAULT_MODE: u32 = 0o755;
const DEFAULT_STRIP_PROGRAM: &str = "strip";

#[allow(dead_code)]
pub struct Behavior {
    main_function: MainFunction,
    specified_mode: Option<u32>,
    suffix: String,
    owner: String,
    group: String,
    verbose: bool,
    preserve_timestamps: bool,
    compare: bool,
    strip: bool,
    strip_program: String,
    create_leading: bool,
    target_dir: Option<String>,
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

static OPT_COMPARE: &str = "compare";
static OPT_BACKUP: &str = "backup";
static OPT_BACKUP_2: &str = "backup2";
static OPT_DIRECTORY: &str = "directory";
static OPT_IGNORED: &str = "ignored";
static OPT_CREATE_LEADING: &str = "create-leading";
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
        .version(crate_version!())
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
            Arg::with_name(OPT_COMPARE)
            .short("C")
            .long(OPT_COMPARE)
            .help("compare each pair of source and destination files, and in some cases, do not modify the destination at all")
        )
        .arg(
            Arg::with_name(OPT_DIRECTORY)
                .short("d")
                .long(OPT_DIRECTORY)
                .help("treat all arguments as directory names. create all components of the specified directories")
        )

        .arg(
            // TODO implement flag
            Arg::with_name(OPT_CREATE_LEADING)
                .short("D")
                .help("create all leading components of DEST except the last, then copy SOURCE to DEST")
        )
        .arg(
            Arg::with_name(OPT_GROUP)
                .short("g")
                .long(OPT_GROUP)
                .help("set group ownership, instead of process's current group")
                .value_name("GROUP")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(OPT_MODE)
                .short("m")
                .long(OPT_MODE)
                .help("set permission mode (as in chmod), instead of rwxr-xr-x")
                .value_name("MODE")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(OPT_OWNER)
                .short("o")
                .long(OPT_OWNER)
                .help("set ownership (super-user only)")
                .value_name("OWNER")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(OPT_PRESERVE_TIMESTAMPS)
                .short("p")
                .long(OPT_PRESERVE_TIMESTAMPS)
                .help("apply access/modification times of SOURCE files to corresponding destination files")
        )
        .arg(
            Arg::with_name(OPT_STRIP)
            .short("s")
            .long(OPT_STRIP)
            .help("strip symbol tables (no action Windows)")
        )
        .arg(
            Arg::with_name(OPT_STRIP_PROGRAM)
                .long(OPT_STRIP_PROGRAM)
                .help("program used to strip binaries (no action Windows)")
                .value_name("PROGRAM")
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_SUFFIX)
                .short("S")
                .long(OPT_SUFFIX)
                .help("(unimplemented) override the usual backup suffix")
                .value_name("SUFFIX")
                .takes_value(true)
                .min_values(1)
        )
        .arg(
            // TODO implement flag
            Arg::with_name(OPT_TARGET_DIRECTORY)
                .short("t")
                .long(OPT_TARGET_DIRECTORY)
                .help("move all SOURCE arguments into DIRECTORY")
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
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true).min_values(1))
        .get_matches_from(args);

    let paths: Vec<String> = matches
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
    } else if matches.is_present(OPT_SUFFIX) {
        Err("--suffix, -S")
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
    let main_function = if matches.is_present(OPT_DIRECTORY) {
        MainFunction::Directory
    } else {
        MainFunction::Standard
    };

    let considering_dir: bool = MainFunction::Directory == main_function;

    let specified_mode: Option<u32> = if matches.is_present(OPT_MODE) {
        let x = matches.value_of(OPT_MODE).ok_or(1)?;
        Some(mode::parse(x, considering_dir).map_err(|err| {
            show_error!("Invalid mode string: {}", err);
            1
        })?)
    } else {
        None
    };

    let backup_suffix = if matches.is_present(OPT_SUFFIX) {
        matches.value_of(OPT_SUFFIX).ok_or(1)?
    } else {
        "~"
    };

    let target_dir = matches.value_of(OPT_TARGET_DIRECTORY).map(|d| d.to_owned());

    Ok(Behavior {
        main_function,
        specified_mode,
        suffix: backup_suffix.to_string(),
        owner: matches.value_of(OPT_OWNER).unwrap_or("").to_string(),
        group: matches.value_of(OPT_GROUP).unwrap_or("").to_string(),
        verbose: matches.is_present(OPT_VERBOSE),
        preserve_timestamps: matches.is_present(OPT_PRESERVE_TIMESTAMPS),
        compare: matches.is_present(OPT_COMPARE),
        strip: matches.is_present(OPT_STRIP),
        strip_program: String::from(
            matches
                .value_of(OPT_STRIP_PROGRAM)
                .unwrap_or(DEFAULT_STRIP_PROGRAM),
        ),
        create_leading: matches.is_present(OPT_CREATE_LEADING),
        target_dir,
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

        for path in paths.iter().map(Path::new) {
            // if the path already exist, don't try to create it again
            if !path.exists() {
                // Differently than the primary functionality (MainFunction::Standard), the directory
                // functionality should create all ancestors (or components) of a directory regardless
                // of the presence of the "-D" flag.
                // NOTE: the GNU "install" sets the expected mode only for the target directory. All
                // created ancestor directories will have the default mode. Hence it is safe to use
                // fs::create_dir_all and then only modify the target's dir mode.
                if let Err(e) = fs::create_dir_all(path) {
                    show_error!("{}: {}", path.display(), e);
                    all_successful = false;
                    continue;
                }

                if b.verbose {
                    show_error!("creating directory '{}'", path.display());
                }
            }

            if mode::chmod(path, b.mode()).is_err() {
                all_successful = false;
                continue;
            }
        }
        if all_successful {
            0
        } else {
            1
        }
    }
}

/// Test if the path is a new file path that can be
/// created immediately
fn is_new_file_path(path: &Path) -> bool {
    !path.exists()
        && (path.parent().map(Path::is_dir).unwrap_or(true)
            || path.parent().unwrap().to_string_lossy().is_empty()) // In case of a simple file
}

/// Perform an install, given a list of paths and behavior.
///
/// Returns an integer intended as a program return code.
///
fn standard(mut paths: Vec<String>, b: Behavior) -> i32 {
    let target: PathBuf = b
        .target_dir
        .clone()
        .unwrap_or_else(|| paths.pop().unwrap())
        .into();

    let sources = &paths.iter().map(PathBuf::from).collect::<Vec<_>>();

    if sources.len() > 1 || (target.exists() && target.is_dir()) {
        copy_files_into_dir(sources, &target, &b)
    } else {
        if let Some(parent) = target.parent() {
            if !parent.exists() && b.create_leading {
                if let Err(e) = fs::create_dir_all(parent) {
                    show_error!("failed to create {}: {}", parent.display(), e);
                    return 1;
                }

                if mode::chmod(parent, b.mode()).is_err() {
                    show_error!("failed to chmod {}", parent.display());
                    return 1;
                }
            }
        }

        if target.is_file() || is_new_file_path(&target) {
            copy_file_to_file(&sources[0], &target, &b)
        } else {
            show_error!(
                "invalid target {}: No such file or directory",
                target.display()
            );
            1
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
fn copy_files_into_dir(files: &[PathBuf], target_dir: &Path, b: &Behavior) -> i32 {
    if !target_dir.is_dir() {
        show_error!("target '{}' is not a directory", target_dir.display());
        return 1;
    }

    let mut all_successful = true;
    for sourcepath in files.iter() {
        if !sourcepath.exists() {
            show_error!(
                "cannot stat '{}': No such file or directory",
                sourcepath.display()
            );

            all_successful = false;
            continue;
        }

        if sourcepath.is_dir() {
            show_error!("omitting directory '{}'", sourcepath.display());
            all_successful = false;
            continue;
        }

        let mut targetpath = target_dir.to_path_buf();
        let filename = sourcepath.components().last().unwrap();
        targetpath.push(filename);

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
fn copy_file_to_file(file: &Path, target: &Path, b: &Behavior) -> i32 {
    if copy(file, target, b).is_err() {
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
#[allow(clippy::cognitive_complexity)]
fn copy(from: &Path, to: &Path, b: &Behavior) -> Result<(), ()> {
    if b.compare && !need_copy(from, to, b) {
        return Ok(());
    }

    if from.to_string_lossy() == "/dev/null" {
        /* workaround a limitation of fs::copy
         * https://github.com/rust-lang/rust/issues/79390
         */
        if let Err(err) = File::create(to) {
            show_error!(
                "install: cannot install '{}' to '{}': {}",
                from.display(),
                to.display(),
                err
            );
            return Err(());
        }
    } else if let Err(err) = fs::copy(from, to) {
        show_error!(
            "cannot install '{}' to '{}': {}",
            from.display(),
            to.display(),
            err
        );
        return Err(());
    }

    if b.strip && cfg!(not(windows)) {
        match Command::new(&b.strip_program).arg(to).output() {
            Ok(o) => {
                if !o.status.success() {
                    crash!(
                        1,
                        "strip program failed: {}",
                        String::from_utf8(o.stderr).unwrap_or_default()
                    );
                }
            }
            Err(e) => crash!(1, "strip program execution failed: {}", e),
        }
    }

    if mode::chmod(to, b.mode()).is_err() {
        return Err(());
    }

    if !b.owner.is_empty() {
        let meta = match fs::metadata(to) {
            Ok(meta) => meta,
            Err(f) => crash!(1, "{}", f.to_string()),
        };

        let owner_id = match usr2uid(&b.owner) {
            Ok(g) => g,
            _ => crash!(1, "no such user: {}", b.owner),
        };
        let gid = meta.gid();
        match wrap_chown(
            to,
            &meta,
            Some(owner_id),
            Some(gid),
            false,
            Verbosity::Normal,
        ) {
            Ok(n) => {
                if !n.is_empty() {
                    show_error!("{}", n);
                }
            }
            Err(e) => show_error!("{}", e),
        }
    }

    if !b.group.is_empty() {
        let meta = match fs::metadata(to) {
            Ok(meta) => meta,
            Err(f) => crash!(1, "{}", f.to_string()),
        };

        let group_id = match grp2gid(&b.group) {
            Ok(g) => g,
            _ => crash!(1, "no such group: {}", b.group),
        };
        match wrap_chgrp(to, &meta, group_id, false, Verbosity::Normal) {
            Ok(n) => {
                if !n.is_empty() {
                    show_error!("{}", n);
                }
            }
            Err(e) => show_error!("{}", e),
        }
    }

    if b.preserve_timestamps {
        let meta = match fs::metadata(from) {
            Ok(meta) => meta,
            Err(f) => crash!(1, "{}", f.to_string()),
        };

        let modified_time = FileTime::from_last_modification_time(&meta);
        let accessed_time = FileTime::from_last_access_time(&meta);

        match set_file_times(to, accessed_time, modified_time) {
            Ok(_) => {}
            Err(e) => show_error!("{}", e),
        }
    }

    if b.verbose {
        show_error!("'{}' -> '{}'", from.display(), to.display());
    }

    Ok(())
}

/// Return true if a file is necessary to copy. This is the case when:
/// - _from_ or _to_ is nonexistent;
/// - either file has a sticky bit or set[ug]id bit, or the user specified one;
/// - either file isn't a regular file;
/// - the sizes of _from_ and _to_ differ;
/// - _to_'s owner differs from intended; or
/// - the contents of _from_ and _to_ differ.
///
/// # Parameters
///
/// _from_ and _to_, if existent, must be non-directories.
///
/// # Errors
///
/// Crashes the program if a nonexistent owner or group is specified in _b_.
///
fn need_copy(from: &Path, to: &Path, b: &Behavior) -> bool {
    let from_meta = match fs::metadata(from) {
        Ok(meta) => meta,
        Err(_) => return true,
    };
    let to_meta = match fs::metadata(to) {
        Ok(meta) => meta,
        Err(_) => return true,
    };

    // setuid || setgid || sticky
    let extra_mode: u32 = 0o7000;

    if b.specified_mode.unwrap_or(0) & extra_mode != 0
        || from_meta.mode() & extra_mode != 0
        || to_meta.mode() & extra_mode != 0
    {
        return true;
    }

    if !from_meta.is_file() || !to_meta.is_file() {
        return true;
    }

    if from_meta.len() != to_meta.len() {
        return true;
    }

    // TODO: if -P (#1809) and from/to contexts mismatch, return true.

    if !b.owner.is_empty() {
        let owner_id = match usr2uid(&b.owner) {
            Ok(id) => id,
            _ => crash!(1, "no such user: {}", b.owner),
        };
        if owner_id != to_meta.uid() {
            return true;
        }
    } else if !b.group.is_empty() {
        let group_id = match grp2gid(&b.group) {
            Ok(id) => id,
            _ => crash!(1, "no such group: {}", b.group),
        };
        if group_id != to_meta.gid() {
            return true;
        }
    } else {
        #[cfg(not(target_os = "windows"))]
        unsafe {
            if to_meta.uid() != geteuid() || to_meta.gid() != getegid() {
                return true;
            }
        }
    }

    if !diff(from.to_str().unwrap(), to.to_str().unwrap()) {
        return true;
    }

    false
}
