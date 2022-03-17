//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Ben Eills <ben@beneills.com>
//  *
//  * For the full copyright and license information, please view the LICENSE file
//  * that was distributed with this source code.

// spell-checker:ignore (ToDO) rwxr sourcepath targetpath Isnt uioerror

mod mode;

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, ArgMatches, Command};
use file_diff::diff;
use filetime::{set_file_times, FileTime};
use uucore::backup_control::{self, BackupMode};
use uucore::display::Quotable;
use uucore::entries::{grp2gid, usr2uid};
use uucore::error::{FromIo, UError, UIoError, UResult, UUsageError};
use uucore::format_usage;
use uucore::mode::get_umask;
use uucore::perms::{wrap_chown, Verbosity, VerbosityLevel};

use libc::{getegid, geteuid};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::fs;
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process;

const DEFAULT_MODE: u32 = 0o755;
const DEFAULT_STRIP_PROGRAM: &str = "strip";

#[allow(dead_code)]
pub struct Behavior {
    main_function: MainFunction,
    specified_mode: Option<u32>,
    backup_mode: BackupMode,
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

#[derive(Debug)]
enum InstallError {
    Unimplemented(String),
    DirNeedsArg(),
    CreateDirFailed(PathBuf, std::io::Error),
    ChmodFailed(PathBuf),
    InvalidTarget(PathBuf),
    TargetDirIsntDir(PathBuf),
    BackupFailed(PathBuf, PathBuf, std::io::Error),
    InstallFailed(PathBuf, PathBuf, std::io::Error),
    StripProgramFailed(String),
    MetadataFailed(std::io::Error),
    NoSuchUser(String),
    NoSuchGroup(String),
    OmittingDirectory(PathBuf),
}

impl UError for InstallError {
    fn code(&self) -> i32 {
        match self {
            InstallError::Unimplemented(_) => 2,
            _ => 1,
        }
    }

    fn usage(&self) -> bool {
        false
    }
}

impl Error for InstallError {}

impl Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use InstallError as IE;
        match self {
            IE::Unimplemented(opt) => write!(f, "Unimplemented feature: {}", opt),
            IE::DirNeedsArg() => {
                write!(
                    f,
                    "{} with -d requires at least one argument.",
                    uucore::util_name()
                )
            }
            IE::CreateDirFailed(dir, e) => {
                Display::fmt(&uio_error!(e, "failed to create {}", dir.quote()), f)
            }
            IE::ChmodFailed(file) => write!(f, "failed to chmod {}", file.quote()),
            IE::InvalidTarget(target) => write!(
                f,
                "invalid target {}: No such file or directory",
                target.quote()
            ),
            IE::TargetDirIsntDir(target) => {
                write!(f, "target {} is not a directory", target.quote())
            }
            IE::BackupFailed(from, to, e) => Display::fmt(
                &uio_error!(e, "cannot backup {} to {}", from.quote(), to.quote()),
                f,
            ),
            IE::InstallFailed(from, to, e) => Display::fmt(
                &uio_error!(e, "cannot install {} to {}", from.quote(), to.quote()),
                f,
            ),
            IE::StripProgramFailed(msg) => write!(f, "strip program failed: {}", msg),
            IE::MetadataFailed(e) => Display::fmt(&uio_error!(e, ""), f),
            IE::NoSuchUser(user) => write!(f, "no such user: {}", user.maybe_quote()),
            IE::NoSuchGroup(group) => write!(f, "no such group: {}", group.maybe_quote()),
            IE::OmittingDirectory(dir) => write!(f, "omitting directory {}", dir.quote()),
        }
    }
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
const USAGE: &str = "{} [OPTION]... [FILE]...";

static OPT_COMPARE: &str = "compare";
static OPT_DIRECTORY: &str = "directory";
static OPT_IGNORED: &str = "ignored";
static OPT_CREATE_LEADING: &str = "create-leading";
static OPT_GROUP: &str = "group";
static OPT_MODE: &str = "mode";
static OPT_OWNER: &str = "owner";
static OPT_PRESERVE_TIMESTAMPS: &str = "preserve-timestamps";
static OPT_STRIP: &str = "strip";
static OPT_STRIP_PROGRAM: &str = "strip-program";
static OPT_TARGET_DIRECTORY: &str = "target-directory";
static OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
static OPT_VERBOSE: &str = "verbose";
static OPT_PRESERVE_CONTEXT: &str = "preserve-context";
static OPT_CONTEXT: &str = "context";

static ARG_FILES: &str = "files";

/// Main install utility function, called from main.rs.
///
/// Returns a program return code.
///
#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let paths: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    check_unimplemented(&matches)?;

    let behavior = behavior(&matches)?;

    match behavior.main_function {
        MainFunction::Directory => directory(&paths, &behavior),
        MainFunction::Standard => standard(paths, &behavior),
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            backup_control::arguments::backup()
        )
        .arg(
            backup_control::arguments::backup_no_args()
        )
        .arg(
            Arg::new(OPT_IGNORED)
            .short('c')
            .help("ignored")
        )
        .arg(
            Arg::new(OPT_COMPARE)
            .short('C')
            .long(OPT_COMPARE)
            .help("compare each pair of source and destination files, and in some cases, do not modify the destination at all")
        )
        .arg(
            Arg::new(OPT_DIRECTORY)
                .short('d')
                .long(OPT_DIRECTORY)
                .help("treat all arguments as directory names. create all components of the specified directories")
        )

        .arg(
            // TODO implement flag
            Arg::new(OPT_CREATE_LEADING)
                .short('D')
                .help("create all leading components of DEST except the last, then copy SOURCE to DEST")
        )
        .arg(
            Arg::new(OPT_GROUP)
                .short('g')
                .long(OPT_GROUP)
                .help("set group ownership, instead of process's current group")
                .value_name("GROUP")
                .takes_value(true)
        )
        .arg(
            Arg::new(OPT_MODE)
                .short('m')
                .long(OPT_MODE)
                .help("set permission mode (as in chmod), instead of rwxr-xr-x")
                .value_name("MODE")
                .takes_value(true)
        )
        .arg(
            Arg::new(OPT_OWNER)
                .short('o')
                .long(OPT_OWNER)
                .help("set ownership (super-user only)")
                .value_name("OWNER")
                .takes_value(true)
        )
        .arg(
            Arg::new(OPT_PRESERVE_TIMESTAMPS)
                .short('p')
                .long(OPT_PRESERVE_TIMESTAMPS)
                .help("apply access/modification times of SOURCE files to corresponding destination files")
        )
        .arg(
            Arg::new(OPT_STRIP)
                .short('s')
                .long(OPT_STRIP)
                .help("strip symbol tables (no action Windows)")
        )
        .arg(
            Arg::new(OPT_STRIP_PROGRAM)
                .long(OPT_STRIP_PROGRAM)
                .help("program used to strip binaries (no action Windows)")
                .value_name("PROGRAM")
        )
        .arg(
            backup_control::arguments::suffix()
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_TARGET_DIRECTORY)
                .short('t')
                .long(OPT_TARGET_DIRECTORY)
                .help("move all SOURCE arguments into DIRECTORY")
                .value_name("DIRECTORY")
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_NO_TARGET_DIRECTORY)
                .short('T')
                .long(OPT_NO_TARGET_DIRECTORY)
                .help("(unimplemented) treat DEST as a normal file")

        )
        .arg(
            Arg::new(OPT_VERBOSE)
            .short('v')
            .long(OPT_VERBOSE)
            .help("explain what is being done")
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_PRESERVE_CONTEXT)
                .short('P')
                .long(OPT_PRESERVE_CONTEXT)
                .help("(unimplemented) preserve security context")
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_CONTEXT)
                .short('Z')
                .long(OPT_CONTEXT)
                .help("(unimplemented) set security context of files and directories")
                .value_name("CONTEXT")
        )
        .arg(Arg::new(ARG_FILES).multiple_occurrences(true).takes_value(true).min_values(1))
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
fn check_unimplemented(matches: &ArgMatches) -> UResult<()> {
    if matches.is_present(OPT_NO_TARGET_DIRECTORY) {
        Err(InstallError::Unimplemented(String::from("--no-target-directory, -T")).into())
    } else if matches.is_present(OPT_PRESERVE_CONTEXT) {
        Err(InstallError::Unimplemented(String::from("--preserve-context, -P")).into())
    } else if matches.is_present(OPT_CONTEXT) {
        Err(InstallError::Unimplemented(String::from("--context, -Z")).into())
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
fn behavior(matches: &ArgMatches) -> UResult<Behavior> {
    let main_function = if matches.is_present(OPT_DIRECTORY) {
        MainFunction::Directory
    } else {
        MainFunction::Standard
    };

    let considering_dir: bool = MainFunction::Directory == main_function;

    let specified_mode: Option<u32> = if matches.is_present(OPT_MODE) {
        let x = matches.value_of(OPT_MODE).ok_or(1)?;
        Some(mode::parse(x, considering_dir, get_umask()).map_err(|err| {
            show_error!("Invalid mode string: {}", err);
            1
        })?)
    } else {
        None
    };

    let backup_mode = backup_control::determine_backup_mode(matches)?;
    let target_dir = matches.value_of(OPT_TARGET_DIRECTORY).map(|d| d.to_owned());

    Ok(Behavior {
        main_function,
        specified_mode,
        backup_mode,
        suffix: backup_control::determine_backup_suffix(matches),
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
/// Returns a Result type with the Err variant containing the error message.
///
fn directory(paths: &[String], b: &Behavior) -> UResult<()> {
    if paths.is_empty() {
        Err(InstallError::DirNeedsArg().into())
    } else {
        for path in paths.iter().map(Path::new) {
            // if the path already exist, don't try to create it again
            if !path.exists() {
                // Differently than the primary functionality
                // (MainFunction::Standard), the directory functionality should
                // create all ancestors (or components) of a directory
                // regardless of the presence of the "-D" flag.
                //
                // NOTE: the GNU "install" sets the expected mode only for the
                // target directory. All created ancestor directories will have
                // the default mode. Hence it is safe to use fs::create_dir_all
                // and then only modify the target's dir mode.
                if let Err(e) =
                    fs::create_dir_all(path).map_err_context(|| path.maybe_quote().to_string())
                {
                    show!(e);
                    continue;
                }

                if b.verbose {
                    println!("creating directory {}", path.quote());
                }
            }

            if mode::chmod(path, b.mode()).is_err() {
                // Error messages are printed by the mode::chmod function!
                uucore::error::set_exit_code(1);
                continue;
            }
        }
        // If the exit code was set, or show! has been called at least once
        // (which sets the exit code as well), function execution will end after
        // this return.
        Ok(())
    }
}

/// Test if the path is a new file path that can be
/// created immediately
fn is_new_file_path(path: &Path) -> bool {
    !path.exists()
        && (path.parent().map(Path::is_dir).unwrap_or(true)
            || path.parent().unwrap().as_os_str().is_empty()) // In case of a simple file
}

/// Perform an install, given a list of paths and behavior.
///
/// Returns a Result type with the Err variant containing the error message.
///
fn standard(mut paths: Vec<String>, b: &Behavior) -> UResult<()> {
    let target: PathBuf = if let Some(path) = &b.target_dir {
        path.into()
    } else {
        paths
            .pop()
            .ok_or_else(|| UUsageError::new(1, "missing file operand"))?
            .into()
    };

    let sources = &paths.iter().map(PathBuf::from).collect::<Vec<_>>();

    if sources.len() > 1 || (target.exists() && target.is_dir()) {
        copy_files_into_dir(sources, &target, b)
    } else {
        if let Some(parent) = target.parent() {
            if !parent.exists() && b.create_leading {
                if let Err(e) = fs::create_dir_all(parent) {
                    return Err(InstallError::CreateDirFailed(parent.to_path_buf(), e).into());
                }

                // Silent the warning as we want to the error message
                #[allow(clippy::question_mark)]
                if mode::chmod(parent, b.mode()).is_err() {
                    return Err(InstallError::ChmodFailed(parent.to_path_buf()).into());
                }
            }
        }

        if target.is_file() || is_new_file_path(&target) {
            copy(
                sources.get(0).ok_or_else(|| {
                    UUsageError::new(
                        1,
                        format!(
                            "missing destination file operand after '{}'",
                            target.to_str().unwrap()
                        ),
                    )
                })?,
                &target,
                b,
            )
        } else {
            Err(InstallError::InvalidTarget(target).into())
        }
    }
}

/// Copy some files into a directory.
///
/// Prints verbose information and error messages.
/// Returns a Result type with the Err variant containing the error message.
///
/// # Parameters
///
/// _files_ must all exist as non-directories.
/// _target_dir_ must be a directory.
///
fn copy_files_into_dir(files: &[PathBuf], target_dir: &Path, b: &Behavior) -> UResult<()> {
    if !target_dir.is_dir() {
        return Err(InstallError::TargetDirIsntDir(target_dir.to_path_buf()).into());
    }
    for sourcepath in files.iter() {
        if let Err(err) = sourcepath
            .metadata()
            .map_err_context(|| format!("cannot stat {}", sourcepath.quote()))
        {
            show!(err);
            continue;
        }

        if sourcepath.is_dir() {
            let err = InstallError::OmittingDirectory(sourcepath.to_path_buf());
            show!(err);
            continue;
        }

        let mut targetpath = target_dir.to_path_buf();
        let filename = sourcepath.components().last().unwrap();
        targetpath.push(filename);

        show_if_err!(copy(sourcepath, &targetpath, b));
    }
    // If the exit code was set, or show! has been called at least once
    // (which sets the exit code as well), function execution will end after
    // this return.
    Ok(())
}

/// Copy one file to a new location, changing metadata.
///
/// Returns a Result type with the Err variant containing the error message.
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
fn copy(from: &Path, to: &Path, b: &Behavior) -> UResult<()> {
    if b.compare && !need_copy(from, to, b)? {
        return Ok(());
    }
    // Declare the path here as we may need it for the verbose output below.
    let mut backup_path = None;

    // Perform backup, if any, before overwriting 'to'
    //
    // The codes actually making use of the backup process don't seem to agree
    // on how best to approach the issue. (mv and ln, for example)
    if to.exists() {
        backup_path = backup_control::get_backup_path(b.backup_mode, to, &b.suffix);
        if let Some(ref backup_path) = backup_path {
            // TODO!!
            if let Err(err) = fs::rename(to, backup_path) {
                return Err(InstallError::BackupFailed(
                    to.to_path_buf(),
                    backup_path.to_path_buf(),
                    err,
                )
                .into());
            }
        }
    }

    if from.as_os_str() == "/dev/null" {
        /* workaround a limitation of fs::copy
         * https://github.com/rust-lang/rust/issues/79390
         */
        if let Err(err) = File::create(to) {
            return Err(
                InstallError::InstallFailed(from.to_path_buf(), to.to_path_buf(), err).into(),
            );
        }
    } else if let Err(err) = fs::copy(from, to) {
        return Err(InstallError::InstallFailed(from.to_path_buf(), to.to_path_buf(), err).into());
    }

    if b.strip && cfg!(not(windows)) {
        match process::Command::new(&b.strip_program).arg(to).output() {
            Ok(o) => {
                if !o.status.success() {
                    return Err(InstallError::StripProgramFailed(
                        String::from_utf8(o.stderr).unwrap_or_default(),
                    )
                    .into());
                }
            }
            Err(e) => return Err(InstallError::StripProgramFailed(e.to_string()).into()),
        }
    }

    // Silent the warning as we want to the error message
    #[allow(clippy::question_mark)]
    if mode::chmod(to, b.mode()).is_err() {
        return Err(InstallError::ChmodFailed(to.to_path_buf()).into());
    }

    if !b.owner.is_empty() {
        let meta = match fs::metadata(to) {
            Ok(meta) => meta,
            Err(e) => return Err(InstallError::MetadataFailed(e).into()),
        };

        let owner_id = match usr2uid(&b.owner) {
            Ok(g) => g,
            _ => return Err(InstallError::NoSuchUser(b.owner.clone()).into()),
        };
        let gid = meta.gid();
        match wrap_chown(
            to,
            &meta,
            Some(owner_id),
            Some(gid),
            false,
            Verbosity {
                groups_only: false,
                level: VerbosityLevel::Normal,
            },
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
            Err(e) => return Err(InstallError::MetadataFailed(e).into()),
        };

        let group_id = match grp2gid(&b.group) {
            Ok(g) => g,
            _ => return Err(InstallError::NoSuchGroup(b.group.clone()).into()),
        };
        match wrap_chown(
            to,
            &meta,
            Some(group_id),
            None,
            false,
            Verbosity {
                groups_only: true,
                level: VerbosityLevel::Normal,
            },
        ) {
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
            Err(e) => return Err(InstallError::MetadataFailed(e).into()),
        };

        let modified_time = FileTime::from_last_modification_time(&meta);
        let accessed_time = FileTime::from_last_access_time(&meta);

        match set_file_times(to, accessed_time, modified_time) {
            Ok(_) => {}
            Err(e) => show_error!("{}", e),
        }
    }

    if b.verbose {
        print!("{} -> {}", from.quote(), to.quote());
        match backup_path {
            Some(path) => println!(" (backup: {})", path.quote()),
            None => println!(),
        }
    }

    Ok(())
}

/// Return true if a file is necessary to copy. This is the case when:
///
/// - _from_ or _to_ is nonexistent;
/// - either file has a sticky bit or set\[ug\]id bit, or the user specified one;
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
fn need_copy(from: &Path, to: &Path, b: &Behavior) -> UResult<bool> {
    let from_meta = match fs::metadata(from) {
        Ok(meta) => meta,
        Err(_) => return Ok(true),
    };
    let to_meta = match fs::metadata(to) {
        Ok(meta) => meta,
        Err(_) => return Ok(true),
    };

    // setuid || setgid || sticky
    let extra_mode: u32 = 0o7000;

    if b.specified_mode.unwrap_or(0) & extra_mode != 0
        || from_meta.mode() & extra_mode != 0
        || to_meta.mode() & extra_mode != 0
    {
        return Ok(true);
    }

    if !from_meta.is_file() || !to_meta.is_file() {
        return Ok(true);
    }

    if from_meta.len() != to_meta.len() {
        return Ok(true);
    }

    // TODO: if -P (#1809) and from/to contexts mismatch, return true.

    if !b.owner.is_empty() {
        let owner_id = match usr2uid(&b.owner) {
            Ok(id) => id,
            _ => return Err(InstallError::NoSuchUser(b.owner.clone()).into()),
        };
        if owner_id != to_meta.uid() {
            return Ok(true);
        }
    } else if !b.group.is_empty() {
        let group_id = match grp2gid(&b.group) {
            Ok(id) => id,
            _ => return Err(InstallError::NoSuchGroup(b.group.clone()).into()),
        };
        if group_id != to_meta.gid() {
            return Ok(true);
        }
    } else {
        #[cfg(not(target_os = "windows"))]
        unsafe {
            if to_meta.uid() != geteuid() || to_meta.gid() != getegid() {
                return Ok(true);
            }
        }
    }

    if !diff(from.to_str().unwrap(), to.to_str().unwrap()) {
        return Ok(true);
    }

    Ok(false)
}
