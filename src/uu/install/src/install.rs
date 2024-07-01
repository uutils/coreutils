// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) rwxr sourcepath targetpath Isnt uioerror

mod mode;

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use file_diff::diff;
use filetime::{set_file_times, FileTime};
use quick_error::quick_error;
use std::fs;
use std::fs::File;
use std::io;
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::process;
use uucore::backup_control::{self, BackupMode};
use uucore::display::Quotable;
use uucore::entries::{grp2gid, usr2uid};
use uucore::error::{FromIo, UError, UResult, UUsageError};
use uucore::fs::dir_strip_dot_for_creation;
use uucore::mode::get_umask;
use uucore::perms::{wrap_chown, Verbosity, VerbosityLevel};
use uucore::process::{getegid, geteuid};
use uucore::{format_usage, help_about, help_usage, show, show_error, show_if_err};

const DEFAULT_MODE: u32 = 0o755;
const DEFAULT_STRIP_PROGRAM: &str = "strip";

#[allow(dead_code)]
pub struct Behavior {
    main_function: MainFunction,
    specified_mode: Option<u32>,
    backup_mode: BackupMode,
    suffix: String,
    owner_id: Option<u32>,
    group_id: Option<u32>,
    verbose: bool,
    preserve_timestamps: bool,
    compare: bool,
    strip: bool,
    strip_program: String,
    create_leading: bool,
    target_dir: Option<String>,
}

quick_error! {
    #[derive(Debug)]
    pub enum InstallError {
        Unimplemented(opt: String) {
            display("Unimplemented feature: {}", opt)
        }
        DirNeedsArg {
            display("{} with -d requires at least one argument.", uucore::util_name())
        }
        CreateDirFailed(dir: PathBuf, err: io::Error) {
            display("failed to create {}: {}", dir.quote(), err)
        }
        ChmodFailed(file: PathBuf) {
            display("failed to chmod {}", file.quote())
        }
        ChownFailed(file: PathBuf, msg: String) {
            display("failed to chown {}: {}", file.quote(), msg)
        }
        InvalidTarget(target: PathBuf) {
            display("invalid target {}: No such file or directory", target.quote())
        }
        TargetDirIsntDir(target: PathBuf) {
            display("target {} is not a directory", target.quote())
        }
        BackupFailed(from: PathBuf, to: PathBuf, err: io::Error) {
            display("cannot backup {} to {}: {}", from.quote(), to.quote(), err)
        }
        InstallFailed(from: PathBuf, to: PathBuf, err: io::Error) {
            display("cannot install {} to {}: {}", from.quote(), to.quote(), err)
        }
        StripProgramFailed(msg: String) {
            display("strip program failed: {}", msg)
        }
        MetadataFailed(err: io::Error) {
            display("{}", err)
        }
        InvalidUser(user: String) {
            display("invalid user: {}", user.quote())
        }
        InvalidGroup(group: String) {
            display("invalid group: {}", group.quote())
        }
        OmittingDirectory(dir: PathBuf) {
            display("omitting directory {}", dir.quote())
        }
        NotADirectory(dir: PathBuf) {
            display("failed to access {}: Not a directory", dir.quote())
        }
    }
}

impl UError for InstallError {
    fn code(&self) -> i32 {
        match self {
            Self::Unimplemented(_) => 2,
            _ => 1,
        }
    }

    fn usage(&self) -> bool {
        false
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

const ABOUT: &str = help_about!("install.md");
const USAGE: &str = help_usage!("install.md");

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
    let matches = uu_app().try_get_matches_from(args)?;

    let paths: Vec<String> = matches
        .get_many::<String>(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    check_unimplemented(&matches)?;

    let behavior = behavior(&matches)?;

    match behavior.main_function {
        MainFunction::Directory => directory(&paths, &behavior),
        MainFunction::Standard => standard(paths, &behavior),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(backup_control::arguments::backup())
        .arg(backup_control::arguments::backup_no_args())
        .arg(
            Arg::new(OPT_IGNORED)
                .short('c')
                .help("ignored")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_COMPARE)
                .short('C')
                .long(OPT_COMPARE)
                .help(
                    "compare each pair of source and destination files, and in some cases, \
                    do not modify the destination at all",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_DIRECTORY)
                .short('d')
                .long(OPT_DIRECTORY)
                .help(
                    "treat all arguments as directory names. create all components of \
                        the specified directories",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_CREATE_LEADING)
                .short('D')
                .help(
                    "create all leading components of DEST except the last, then copy \
                        SOURCE to DEST",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_GROUP)
                .short('g')
                .long(OPT_GROUP)
                .help("set group ownership, instead of process's current group")
                .value_name("GROUP"),
        )
        .arg(
            Arg::new(OPT_MODE)
                .short('m')
                .long(OPT_MODE)
                .help("set permission mode (as in chmod), instead of rwxr-xr-x")
                .value_name("MODE"),
        )
        .arg(
            Arg::new(OPT_OWNER)
                .short('o')
                .long(OPT_OWNER)
                .help("set ownership (super-user only)")
                .value_name("OWNER")
                .value_hint(clap::ValueHint::Username),
        )
        .arg(
            Arg::new(OPT_PRESERVE_TIMESTAMPS)
                .short('p')
                .long(OPT_PRESERVE_TIMESTAMPS)
                .help(
                    "apply access/modification times of SOURCE files to \
                    corresponding destination files",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_STRIP)
                .short('s')
                .long(OPT_STRIP)
                .help("strip symbol tables (no action Windows)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_STRIP_PROGRAM)
                .long(OPT_STRIP_PROGRAM)
                .help("program used to strip binaries (no action Windows)")
                .value_name("PROGRAM")
                .value_hint(clap::ValueHint::CommandName),
        )
        .arg(backup_control::arguments::suffix())
        .arg(
            // TODO implement flag
            Arg::new(OPT_TARGET_DIRECTORY)
                .short('t')
                .long(OPT_TARGET_DIRECTORY)
                .help("move all SOURCE arguments into DIRECTORY")
                .value_name("DIRECTORY")
                .value_hint(clap::ValueHint::DirPath),
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_NO_TARGET_DIRECTORY)
                .short('T')
                .long(OPT_NO_TARGET_DIRECTORY)
                .help("(unimplemented) treat DEST as a normal file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_VERBOSE)
                .short('v')
                .long(OPT_VERBOSE)
                .help("explain what is being done")
                .action(ArgAction::SetTrue),
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_PRESERVE_CONTEXT)
                .short('P')
                .long(OPT_PRESERVE_CONTEXT)
                .help("(unimplemented) preserve security context")
                .action(ArgAction::SetTrue),
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_CONTEXT)
                .short('Z')
                .long(OPT_CONTEXT)
                .help("(unimplemented) set security context of files and directories")
                .value_name("CONTEXT")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .num_args(1..)
                .value_hint(clap::ValueHint::AnyPath),
        )
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
    if matches.get_flag(OPT_NO_TARGET_DIRECTORY) {
        Err(InstallError::Unimplemented(String::from("--no-target-directory, -T")).into())
    } else if matches.get_flag(OPT_PRESERVE_CONTEXT) {
        Err(InstallError::Unimplemented(String::from("--preserve-context, -P")).into())
    } else if matches.get_flag(OPT_CONTEXT) {
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
    let main_function = if matches.get_flag(OPT_DIRECTORY) {
        MainFunction::Directory
    } else {
        MainFunction::Standard
    };

    let considering_dir: bool = MainFunction::Directory == main_function;

    let specified_mode: Option<u32> = if matches.contains_id(OPT_MODE) {
        let x = matches.get_one::<String>(OPT_MODE).ok_or(1)?;
        Some(mode::parse(x, considering_dir, get_umask()).map_err(|err| {
            show_error!("Invalid mode string: {}", err);
            1
        })?)
    } else {
        None
    };

    let backup_mode = backup_control::determine_backup_mode(matches)?;
    let target_dir = matches.get_one::<String>(OPT_TARGET_DIRECTORY).cloned();

    let preserve_timestamps = matches.get_flag(OPT_PRESERVE_TIMESTAMPS);
    let compare = matches.get_flag(OPT_COMPARE);
    let strip = matches.get_flag(OPT_STRIP);
    if preserve_timestamps && compare {
        show_error!("Options --compare and --preserve-timestamps are mutually exclusive");
        return Err(1.into());
    }
    if compare && strip {
        show_error!("Options --compare and --strip are mutually exclusive");
        return Err(1.into());
    }

    let owner = matches
        .get_one::<String>(OPT_OWNER)
        .map(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    let owner_id = if owner.is_empty() {
        None
    } else {
        match usr2uid(&owner) {
            Ok(u) => Some(u),
            Err(_) => return Err(InstallError::InvalidUser(owner.clone()).into()),
        }
    };

    let group = matches
        .get_one::<String>(OPT_GROUP)
        .map(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    let group_id = if group.is_empty() {
        None
    } else {
        match grp2gid(&group) {
            Ok(g) => Some(g),
            Err(_) => return Err(InstallError::InvalidGroup(group.clone()).into()),
        }
    };

    Ok(Behavior {
        main_function,
        specified_mode,
        backup_mode,
        suffix: backup_control::determine_backup_suffix(matches),
        owner_id,
        group_id,
        verbose: matches.get_flag(OPT_VERBOSE),
        preserve_timestamps,
        compare,
        strip,
        strip_program: String::from(
            matches
                .get_one::<String>(OPT_STRIP_PROGRAM)
                .map(|s| s.as_str())
                .unwrap_or(DEFAULT_STRIP_PROGRAM),
        ),
        create_leading: matches.get_flag(OPT_CREATE_LEADING),
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
        Err(InstallError::DirNeedsArg.into())
    } else {
        for path in paths.iter().map(Path::new) {
            // if the path already exist, don't try to create it again
            if !path.exists() {
                // Special case to match GNU's behavior:
                // install -d foo/. should work and just create foo/
                // std::fs::create_dir("foo/."); fails in pure Rust
                // See also mkdir.rs for another occurrence of this
                let path_to_create = dir_strip_dot_for_creation(path);
                // Differently than the primary functionality
                // (MainFunction::Standard), the directory functionality should
                // create all ancestors (or components) of a directory
                // regardless of the presence of the "-D" flag.
                //
                // NOTE: the GNU "install" sets the expected mode only for the
                // target directory. All created ancestor directories will have
                // the default mode. Hence it is safe to use fs::create_dir_all
                // and then only modify the target's dir mode.
                if let Err(e) = fs::create_dir_all(path_to_create.as_path())
                    .map_err_context(|| path_to_create.as_path().maybe_quote().to_string())
                {
                    show!(e);
                    continue;
                }

                if b.verbose {
                    println!("creating directory {}", path_to_create.quote());
                }
            }

            if mode::chmod(path, b.mode()).is_err() {
                // Error messages are printed by the mode::chmod function!
                uucore::error::set_exit_code(1);
                continue;
            }

            show_if_err!(chown_optional_user_group(path, b));
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

/// Test if the path is an existing directory or ends with a trailing separator.
///
/// Returns true, if one of the conditions above is met; else false.
///
#[cfg(unix)]
fn is_potential_directory_path(path: &Path) -> bool {
    let separator = MAIN_SEPARATOR as u8;
    path.as_os_str().as_bytes().last() == Some(&separator) || path.is_dir()
}

#[cfg(not(unix))]
fn is_potential_directory_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.ends_with(MAIN_SEPARATOR) || path_str.ends_with('/') || path.is_dir()
}

/// Perform an install, given a list of paths and behavior.
///
/// Returns a Result type with the Err variant containing the error message.
///
#[allow(clippy::cognitive_complexity)]
fn standard(mut paths: Vec<String>, b: &Behavior) -> UResult<()> {
    // first check that paths contains at least one element
    if paths.is_empty() {
        return Err(UUsageError::new(1, "missing file operand"));
    }

    // get the target from either "-t foo" param or from the last given paths argument
    let target: PathBuf = if let Some(path) = &b.target_dir {
        path.into()
    } else {
        let last_path: PathBuf = paths.pop().unwrap().into();

        // paths has to contain more elements
        if paths.is_empty() {
            return Err(UUsageError::new(
                1,
                format!(
                    "missing destination file operand after '{}'",
                    last_path.to_str().unwrap()
                ),
            ));
        }

        last_path
    };

    let sources = &paths.iter().map(PathBuf::from).collect::<Vec<_>>();

    if b.create_leading {
        // if -t is used in combination with -D, create whole target because it does not include filename
        let to_create: Option<&Path> = if b.target_dir.is_some() {
            Some(target.as_path())
        // if source and target are filenames used in combination with -D, create target's parent
        } else if !(sources.len() > 1 || is_potential_directory_path(&target)) {
            target.parent()
        } else {
            None
        };

        if let Some(to_create) = to_create {
            // if the path ends in /, remove it
            let to_create = if to_create.to_string_lossy().ends_with('/') {
                Path::new(to_create.to_str().unwrap().trim_end_matches('/'))
            } else {
                to_create
            };

            if !to_create.exists() {
                if b.verbose {
                    let mut result = PathBuf::new();
                    // When creating directories with -Dv, show directory creations step by step
                    for part in to_create.components() {
                        result.push(part.as_os_str());
                        if !result.is_dir() {
                            // Don't display when the directory already exists
                            println!("install: creating directory {}", result.quote());
                        }
                    }
                }

                if let Err(e) = fs::create_dir_all(to_create) {
                    return Err(InstallError::CreateDirFailed(to_create.to_path_buf(), e).into());
                }
            }
        }
        if b.target_dir.is_some() {
            let p = to_create.unwrap();

            if !p.exists() || !p.is_dir() {
                return Err(InstallError::NotADirectory(p.to_path_buf()).into());
            }
        }
    }

    if sources.len() > 1 || is_potential_directory_path(&target) {
        copy_files_into_dir(sources, &target, b)
    } else {
        let source = sources.first().unwrap();

        if source.is_dir() {
            return Err(InstallError::OmittingDirectory(source.clone()).into());
        }

        if target.is_file() || is_new_file_path(&target) {
            copy(source, &target, b)
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
    for sourcepath in files {
        if let Err(err) = sourcepath
            .metadata()
            .map_err_context(|| format!("cannot stat {}", sourcepath.quote()))
        {
            show!(err);
            continue;
        }

        if sourcepath.is_dir() {
            let err = InstallError::OmittingDirectory(sourcepath.clone());
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

/// Handle incomplete user/group parings for chown.
///
/// Returns a Result type with the Err variant containing the error message.
/// If the user is root, revert the uid & gid
///
/// # Parameters
///
/// _path_ must exist.
///
/// # Errors
///
/// If the owner or group are invalid or copy system call fails, we print a verbose error and
/// return an empty error value.
///
fn chown_optional_user_group(path: &Path, b: &Behavior) -> UResult<()> {
    // GNU coreutils doesn't print chown operations during install with verbose flag.
    let verbosity = Verbosity {
        groups_only: b.owner_id.is_none(),
        level: VerbosityLevel::Normal,
    };

    // Determine the owner and group IDs to be used for chown.
    let (owner_id, group_id) = if b.owner_id.is_some() || b.group_id.is_some() {
        (b.owner_id, b.group_id)
    } else if geteuid() == 0 {
        // Special case for root user.
        (Some(0), Some(0))
    } else {
        // No chown operation needed.
        return Ok(());
    };

    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(e) => return Err(InstallError::MetadataFailed(e).into()),
    };
    match wrap_chown(path, &meta, owner_id, group_id, false, verbosity) {
        Ok(msg) if b.verbose && !msg.is_empty() => println!("chown: {msg}"),
        Ok(_) => {}
        Err(e) => return Err(InstallError::ChownFailed(path.to_path_buf(), e).into()),
    }

    Ok(())
}

/// Perform backup before overwriting.
///
/// # Parameters
///
/// * `to` - The destination file path.
/// * `b` - The behavior configuration.
///
/// # Returns
///
/// Returns an Option containing the backup path, or None if backup is not needed.
///
fn perform_backup(to: &Path, b: &Behavior) -> UResult<Option<PathBuf>> {
    if to.exists() {
        if b.verbose {
            println!("removed {}", to.quote());
        }
        let backup_path = backup_control::get_backup_path(b.backup_mode, to, &b.suffix);
        if let Some(ref backup_path) = backup_path {
            // TODO!!
            if let Err(err) = fs::rename(to, backup_path) {
                return Err(
                    InstallError::BackupFailed(to.to_path_buf(), backup_path.clone(), err).into(),
                );
            }
        }
        Ok(backup_path)
    } else {
        Ok(None)
    }
}

/// Copy a file from one path to another.
///
/// # Parameters
///
/// * `from` - The source file path.
/// * `to` - The destination file path.
///
/// # Returns
///
/// Returns an empty Result or an error in case of failure.
///
fn copy_file(from: &Path, to: &Path) -> UResult<()> {
    // fs::copy fails if destination is a invalid symlink.
    // so lets just remove all existing files at destination before copy.
    if let Err(e) = fs::remove_file(to) {
        if e.kind() != std::io::ErrorKind::NotFound {
            show_error!(
                "Failed to remove existing file {}. Error: {:?}",
                to.display(),
                e
            );
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
    Ok(())
}

/// Strip a file using an external program.
///
/// # Parameters
///
/// * `to` - The destination file path.
/// * `b` - The behavior configuration.
///
/// # Returns
///
/// Returns an empty Result or an error in case of failure.
///
fn strip_file(to: &Path, b: &Behavior) -> UResult<()> {
    // Check if the filename starts with a hyphen and adjust the path
    let to_str = to.as_os_str().to_str().unwrap_or_default();
    let to = if to_str.starts_with('-') {
        let mut new_path = PathBuf::from(".");
        new_path.push(to);
        new_path
    } else {
        to.to_path_buf()
    };
    match process::Command::new(&b.strip_program).arg(&to).status() {
        Ok(status) => {
            if !status.success() {
                // Follow GNU's behavior: if strip fails, removes the target
                let _ = fs::remove_file(to);
                return Err(InstallError::StripProgramFailed(format!(
                    "strip process terminated abnormally - exit code: {}",
                    status.code().unwrap()
                ))
                .into());
            }
        }
        Err(e) => {
            // Follow GNU's behavior: if strip fails, removes the target
            let _ = fs::remove_file(to);
            return Err(InstallError::StripProgramFailed(e.to_string()).into());
        }
    }
    Ok(())
}

/// Set ownership and permissions on the destination file.
///
/// # Parameters
///
/// * `to` - The destination file path.
/// * `b` - The behavior configuration.
///
/// # Returns
///
/// Returns an empty Result or an error in case of failure.
///
fn set_ownership_and_permissions(to: &Path, b: &Behavior) -> UResult<()> {
    // Silent the warning as we want to the error message
    #[allow(clippy::question_mark)]
    if mode::chmod(to, b.mode()).is_err() {
        return Err(InstallError::ChmodFailed(to.to_path_buf()).into());
    }

    chown_optional_user_group(to, b)?;

    Ok(())
}

/// Preserve timestamps on the destination file.
///
/// # Parameters
///
/// * `from` - The source file path.
/// * `to` - The destination file path.
///
/// # Returns
///
/// Returns an empty Result or an error in case of failure.
///
fn preserve_timestamps(from: &Path, to: &Path) -> UResult<()> {
    let meta = match fs::metadata(from) {
        Ok(meta) => meta,
        Err(e) => return Err(InstallError::MetadataFailed(e).into()),
    };

    let modified_time = FileTime::from_last_modification_time(&meta);
    let accessed_time = FileTime::from_last_access_time(&meta);

    match set_file_times(to, accessed_time, modified_time) {
        Ok(_) => Ok(()),
        Err(e) => {
            show_error!("{}", e);
            Ok(())
        }
    }
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
fn copy(from: &Path, to: &Path, b: &Behavior) -> UResult<()> {
    if b.compare && !need_copy(from, to, b)? {
        return Ok(());
    }
    // Declare the path here as we may need it for the verbose output below.
    let backup_path = perform_backup(to, b)?;

    copy_file(from, to)?;

    #[cfg(not(windows))]
    if b.strip {
        strip_file(to, b)?;
    }

    set_ownership_and_permissions(to, b)?;

    if b.preserve_timestamps {
        preserve_timestamps(from, to)?;
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
    // Attempt to retrieve metadata for the source file.
    // If this fails, assume the file needs to be copied.
    let from_meta = match fs::metadata(from) {
        Ok(meta) => meta,
        Err(_) => return Ok(true),
    };

    // Attempt to retrieve metadata for the destination file.
    // If this fails, assume the file needs to be copied.
    let to_meta = match fs::metadata(to) {
        Ok(meta) => meta,
        Err(_) => return Ok(true),
    };

    // Define special file mode bits (setuid, setgid, sticky).
    let extra_mode: u32 = 0o7000;
    // Define all file mode bits (including permissions).
    // setuid || setgid || sticky || permissions
    let all_modes: u32 = 0o7777;

    // Check if any special mode bits are set in the specified mode,
    // source file mode, or destination file mode.
    if b.specified_mode.unwrap_or(0) & extra_mode != 0
        || from_meta.mode() & extra_mode != 0
        || to_meta.mode() & extra_mode != 0
    {
        return Ok(true);
    }

    // Check if the mode of the destination file differs from the specified mode.
    if b.mode() != to_meta.mode() & all_modes {
        return Ok(true);
    }

    // Check if either the source or destination is not a file.
    if !from_meta.is_file() || !to_meta.is_file() {
        return Ok(true);
    }

    // Check if the file sizes differ.
    if from_meta.len() != to_meta.len() {
        return Ok(true);
    }

    // TODO: if -P (#1809) and from/to contexts mismatch, return true.

    // Check if the owner ID is specified and differs from the destination file's owner.
    if let Some(owner_id) = b.owner_id {
        if owner_id != to_meta.uid() {
            return Ok(true);
        }
    }

    // Check if the group ID is specified and differs from the destination file's group.
    if let Some(group_id) = b.group_id {
        if group_id != to_meta.gid() {
            return Ok(true);
        }
    } else {
        #[cfg(not(target_os = "windows"))]
        // Check if the destination file's owner or group
        // differs from the effective user/group ID of the process.
        if to_meta.uid() != geteuid() || to_meta.gid() != getegid() {
            return Ok(true);
        }
    }

    // Check if the contents of the source and destination files differ.
    if !diff(from.to_str().unwrap(), to.to_str().unwrap()) {
        return Ok(true);
    }

    Ok(false)
}
