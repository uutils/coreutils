// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) sourcepath targetpath nushell canonicalized lred

mod error;

use clap::builder::ValueParser;
use clap::{crate_version, error::ErrorKind, Arg, ArgAction, ArgMatches, Command};
use filetime::set_symlink_file_times;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix;
#[cfg(windows)]
use std::os::windows;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
#[cfg(unix)]
use unix::fs::{FileTypeExt, MetadataExt};
use uucore::backup_control::{self, source_is_target_backup};
use uucore::display::Quotable;
use uucore::error::{set_exit_code, FromIo, UResult, USimpleError, UUsageError};
use uucore::fs::{
    are_hardlinks_or_one_way_symlink_to_same_file, are_hardlinks_to_same_file, disk_usage,
    path_ends_with_terminator,
};
#[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
use uucore::fsxattr;
#[cfg(unix)]
use uucore::perms::{wrap_chown, Verbosity, VerbosityLevel};
use uucore::{show_error, update_control};
use walkdir::WalkDir;

// These are exposed for projects (e.g. nushell) that want to create an `Options` value, which
// requires these enums
pub use uucore::{backup_control::BackupMode, update_control::UpdateMode};
use uucore::{format_usage, help_about, help_section, help_usage, prompt_yes, show};

use fs_extra::{
    error::{ErrorKind as FsXErrorKind, Result as FsXResult},
    file::{self, CopyOptions},
};

use crate::error::MvError;

type MvResult<T> = Result<T, MvError>;

/// Options contains all the possible behaviors and flags for mv.
///
/// All options are public so that the options can be programmatically
/// constructed by other crates, such as nushell. That means that this struct is
/// part of our public API. It should therefore not be changed without good reason.
///
/// The fields are documented with the arguments that determine their value.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Options {
    /// specifies overwrite behavior
    /// '-n' '--no-clobber'
    /// '-i' '--interactive'
    /// '-f' '--force'
    pub overwrite: OverwriteMode,

    /// `--backup[=CONTROL]`, `-b`
    pub backup: BackupMode,

    /// '-S' --suffix' backup suffix
    pub suffix: String,

    /// Available update mode "--update-mode=all|none|older"
    pub update: UpdateMode,

    /// Specifies target directory
    /// '-t, --target-directory=DIRECTORY'
    pub target_dir: Option<OsString>,

    /// Treat destination as a normal file
    /// '-T, --no-target-directory
    pub no_target_dir: bool,

    /// '-v, --verbose'
    pub verbose: bool,

    /// '--strip-trailing-slashes'
    pub strip_slashes: bool,

    /// '-g, --progress'
    pub progress_bar: bool,

    /// `--debug`
    pub debug: bool,
}

/// specifies behavior of the overwrite flag
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OverwriteMode {
    /// '-n' '--no-clobber'   do not overwrite
    NoClobber,
    /// '-i' '--interactive'  prompt before overwrite
    Interactive,
    ///'-f' '--force'         overwrite without prompt
    Force,
}

/// a context for handling verbose output during file operations.
struct VerboseContext<'a> {
    backup: Option<&'a Path>,
    pb: Option<&'a MultiProgress>,
}

impl<'a> VerboseContext<'a> {
    fn new(backup: Option<&'a Path>, pb: Option<&'a MultiProgress>) -> Self {
        VerboseContext { backup, pb }
    }

    fn hide_pb_and_print(&self, msg: &str) {
        match self.pb {
            Some(pb) => pb.suspend(|| {
                println!("{msg}");
            }),
            None => println!("{msg}"),
        };
    }

    fn print_move_file(&self, from: &Path, to: &Path) {
        let message = match self.backup.as_ref() {
            Some(path) => format!(
                "renamed {} -> {} (backup: {})",
                from.quote(),
                to.quote(),
                path.quote()
            ),
            None => format!("renamed {} -> {}", from.quote(), to.quote()),
        };
        self.hide_pb_and_print(&message);
    }

    fn print_copy_file(&self, from: &Path, to: &Path, with_backup_message: bool) {
        let message = match self.backup.as_ref() {
            Some(path) if with_backup_message => format!(
                "copied {} -> {} (backup: {})",
                from.quote(),
                to.quote(),
                path.quote()
            ),
            _ => format!("copied {} -> {}", from.quote(), to.quote()),
        };
        self.hide_pb_and_print(&message);
    }

    fn create_directory(&self, path: &Path) {
        let message = format!(
            "created directory {}",
            path.to_string_lossy()
                .trim_end_matches(MAIN_SEPARATOR)
                .quote()
        );
        self.hide_pb_and_print(&message);
    }

    fn remove_file(&self, from: &Path) {
        let message = format!("removed {}", from.quote());
        self.hide_pb_and_print(&message);
    }

    fn remove_directory(&self, from: &Path) {
        let message = format!("removed directory {}", from.quote());
        self.hide_pb_and_print(&message);
    }
}

const ABOUT: &str = help_about!("mv.md");
const USAGE: &str = help_usage!("mv.md");
const AFTER_HELP: &str = help_section!("after help", "mv.md");
// os error code for when rename operation crosses devices.
#[cfg(unix)]
const CROSSES_DEVICES_ERROR_CODE: i32 = 18;
#[cfg(target_os = "windows")]
const CROSSES_DEVICES_ERROR_CODE: i32 = 17;

static OPT_FORCE: &str = "force";
static OPT_INTERACTIVE: &str = "interactive";
static OPT_NO_CLOBBER: &str = "no-clobber";
static OPT_STRIP_TRAILING_SLASHES: &str = "strip-trailing-slashes";
static OPT_TARGET_DIRECTORY: &str = "target-directory";
static OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
static OPT_VERBOSE: &str = "verbose";
static OPT_PROGRESS: &str = "progress";
static ARG_FILES: &str = "files";
static OPT_DEBUG: &str = "debug";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut app = uu_app();
    let matches = app.try_get_matches_from_mut(args)?;

    let files: Vec<OsString> = matches
        .get_many::<OsString>(ARG_FILES)
        .unwrap_or_default()
        .cloned()
        .collect();

    if files.len() == 1 && !matches.contains_id(OPT_TARGET_DIRECTORY) {
        app.error(
            ErrorKind::TooFewValues,
            format!(
                "The argument '<{ARG_FILES}>...' requires at least 2 values, but only 1 was provided"
            ),
        )
        .exit();
    }

    let overwrite_mode = determine_overwrite_mode(&matches);
    let backup_mode = backup_control::determine_backup_mode(&matches)?;
    let update_mode = update_control::determine_update_mode(&matches);

    if backup_mode != BackupMode::NoBackup
        && (overwrite_mode == OverwriteMode::NoClobber
            || update_mode == UpdateMode::ReplaceNone
            || update_mode == UpdateMode::ReplaceNoneFail)
    {
        return Err(UUsageError::new(
            1,
            "cannot combine --backup with -n/--no-clobber or --update=none-fail",
        ));
    }

    let backup_suffix = backup_control::determine_backup_suffix(&matches);

    let target_dir = matches
        .get_one::<OsString>(OPT_TARGET_DIRECTORY)
        .map(OsString::from);

    if let Some(ref maybe_dir) = target_dir {
        if !Path::new(&maybe_dir).is_dir() {
            return Err(MvError::TargetNotADirectory(maybe_dir.quote().to_string()).into());
        }
    }

    let opts = Options {
        overwrite: overwrite_mode,
        backup: backup_mode,
        suffix: backup_suffix,
        update: update_mode,
        target_dir,
        no_target_dir: matches.get_flag(OPT_NO_TARGET_DIRECTORY),
        verbose: matches.get_flag(OPT_VERBOSE) || matches.get_flag(OPT_DEBUG),
        strip_slashes: matches.get_flag(OPT_STRIP_TRAILING_SLASHES),
        progress_bar: matches.get_flag(OPT_PROGRESS),
        debug: matches.get_flag(OPT_DEBUG),
    };

    mv(&files[..], &opts)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(format!(
            "{AFTER_HELP}\n\n{}",
            backup_control::BACKUP_CONTROL_LONG_HELP
        ))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_FORCE)
                .short('f')
                .long(OPT_FORCE)
                .help("do not prompt before overwriting")
                .overrides_with_all([OPT_INTERACTIVE, OPT_NO_CLOBBER])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_INTERACTIVE)
                .short('i')
                .long(OPT_INTERACTIVE)
                .help("prompt before override")
                .overrides_with_all([OPT_FORCE, OPT_NO_CLOBBER])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_NO_CLOBBER)
                .short('n')
                .long(OPT_NO_CLOBBER)
                .help("do not overwrite an existing file")
                .overrides_with_all([OPT_FORCE, OPT_INTERACTIVE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_STRIP_TRAILING_SLASHES)
                .long(OPT_STRIP_TRAILING_SLASHES)
                .help("remove any trailing slashes from each SOURCE argument")
                .action(ArgAction::SetTrue),
        )
        .arg(backup_control::arguments::backup())
        .arg(backup_control::arguments::backup_no_args())
        .arg(backup_control::arguments::suffix())
        .arg(update_control::arguments::update())
        .arg(update_control::arguments::update_no_args())
        .arg(
            Arg::new(OPT_TARGET_DIRECTORY)
                .short('t')
                .long(OPT_TARGET_DIRECTORY)
                .help("move all SOURCE arguments into DIRECTORY")
                .value_name("DIRECTORY")
                .value_hint(clap::ValueHint::DirPath)
                .conflicts_with(OPT_NO_TARGET_DIRECTORY)
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(OPT_NO_TARGET_DIRECTORY)
                .short('T')
                .long(OPT_NO_TARGET_DIRECTORY)
                .help("treat DEST as a normal file")
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
            Arg::new(OPT_PROGRESS)
                .short('g')
                .long(OPT_PROGRESS)
                .help(
                    "Display a progress bar. \n\
                Note: this feature is not supported by GNU coreutils.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .num_args(1..)
                .required(true)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::AnyPath),
        )
        .arg(
            Arg::new(OPT_DEBUG)
                .long(OPT_DEBUG)
                .help("explain how a file is copied. Implies -v")
                .action(ArgAction::SetTrue),
        )
}

fn determine_overwrite_mode(matches: &ArgMatches) -> OverwriteMode {
    // This does not exactly match the GNU implementation:
    // The GNU mv defaults to Force, but if more than one of the
    // overwrite options are supplied, only the last takes effect.
    // To default to no-clobber in that situation seems safer:
    //
    if matches.get_flag(OPT_NO_CLOBBER) {
        OverwriteMode::NoClobber
    } else if matches.get_flag(OPT_INTERACTIVE) {
        OverwriteMode::Interactive
    } else {
        OverwriteMode::Force
    }
}

fn parse_paths(files: &[OsString], opts: &Options) -> Vec<PathBuf> {
    let paths = files.iter().map(Path::new);

    if opts.strip_slashes {
        paths
            .map(|p| p.components().as_path().to_owned())
            .collect::<Vec<PathBuf>>()
    } else {
        paths.map(|p| p.to_owned()).collect::<Vec<PathBuf>>()
    }
}

fn handle_two_paths(source: &Path, target: &Path, opts: &Options) -> UResult<()> {
    if opts.backup == BackupMode::SimpleBackup
        && source_is_target_backup(source, target, &opts.suffix)
    {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "backing up {} might destroy source;  {} not moved",
                target.quote(),
                source.quote()
            ),
        )
        .into());
    }
    if source.symlink_metadata().is_err() {
        return Err(if path_ends_with_terminator(source) {
            MvError::CannotStatNotADirectory(source.quote().to_string()).into()
        } else {
            MvError::NoSuchFile(source.quote().to_string()).into()
        });
    }

    if (source.eq(target)
        || are_hardlinks_to_same_file(source, target)
        || are_hardlinks_or_one_way_symlink_to_same_file(source, target))
        && opts.backup == BackupMode::NoBackup
    {
        if source.eq(Path::new(".")) || source.ends_with("/.") || source.is_file() {
            return Err(
                MvError::SameFile(source.quote().to_string(), target.quote().to_string()).into(),
            );
        } else {
            return Err(MvError::SelfSubdirectory(source.display().to_string()).into());
        }
    }

    let target_is_dir = target.is_dir();
    let source_is_dir = source.is_dir();

    if path_ends_with_terminator(target)
        && (!target_is_dir && !source_is_dir)
        && !opts.no_target_dir
        && opts.update != UpdateMode::ReplaceIfOlder
    {
        return Err(MvError::FailedToAccessNotADirectory(target.quote().to_string()).into());
    }

    if target_is_dir {
        if opts.no_target_dir {
            if source.is_dir() {
                rename(source, target, opts, None).map_err_context(|| {
                    format!("cannot move {} to {}", source.quote(), target.quote())
                })
            } else {
                Err(MvError::DirectoryToNonDirectory(target.quote().to_string()).into())
            }
        // Check that source & target do not contain same subdir/dir when both exist
        // mkdir dir1/dir2; mv dir1 dir1/dir2
        } else if target.starts_with(source) {
            Err(MvError::SelfTargetSubdirectory(
                source.display().to_string(),
                target.display().to_string(),
            )
            .into())
        } else {
            move_files_into_dir(&[source.to_path_buf()], target, opts)
        }
    } else if target.exists() && source.is_dir() {
        match opts.overwrite {
            OverwriteMode::NoClobber => return Ok(()),
            OverwriteMode::Interactive => {
                if !prompt_yes!("overwrite {}? ", target.quote()) {
                    return Err(io::Error::new(io::ErrorKind::Other, "").into());
                }
            }
            OverwriteMode::Force => {}
        };
        Err(MvError::NonDirectoryToDirectory(
            source.quote().to_string(),
            target.quote().to_string(),
        )
        .into())
    } else {
        rename(source, target, opts, None).map_err(|e| USimpleError::new(1, format!("{e}")))
    }
}

fn handle_multiple_paths(paths: &[PathBuf], opts: &Options) -> UResult<()> {
    if opts.no_target_dir {
        return Err(UUsageError::new(
            1,
            format!("mv: extra operand {}", paths[2].quote()),
        ));
    }
    let target_dir = paths.last().unwrap();
    let sources = &paths[..paths.len() - 1];

    move_files_into_dir(sources, target_dir, opts)
}

/// Execute the mv command. This moves 'source' to 'target', where
/// 'target' is a directory. If 'target' does not exist, and source is a single
/// file or directory, then 'source' will be renamed to 'target'.
pub fn mv(files: &[OsString], opts: &Options) -> UResult<()> {
    let paths = parse_paths(files, opts);

    if let Some(ref name) = opts.target_dir {
        return move_files_into_dir(&paths, &PathBuf::from(name), opts);
    }

    match paths.len() {
        2 => handle_two_paths(&paths[0], &paths[1], opts),
        _ => handle_multiple_paths(&paths, opts),
    }
}

#[allow(clippy::cognitive_complexity)]
fn move_files_into_dir(files: &[PathBuf], target_dir: &Path, options: &Options) -> UResult<()> {
    // remember the moved destinations for further usage
    let mut moved_destinations: HashSet<PathBuf> = HashSet::with_capacity(files.len());

    if !target_dir.is_dir() {
        return Err(MvError::NotADirectory(target_dir.quote().to_string()).into());
    }

    let canonicalized_target_dir = target_dir
        .canonicalize()
        .unwrap_or_else(|_| target_dir.to_path_buf());

    let multi_progress = options.progress_bar.then(MultiProgress::new);

    let count_progress = if let Some(ref multi_progress) = multi_progress {
        if files.len() > 1 {
            Some(multi_progress.add(
                ProgressBar::new(files.len().try_into().unwrap()).with_style(
                    ProgressStyle::with_template("moving {msg} {wide_bar} {pos}/{len}").unwrap(),
                ),
            ))
        } else {
            None
        }
    } else {
        None
    };

    for sourcepath in files {
        if !sourcepath.exists() {
            show!(MvError::NoSuchFile(sourcepath.quote().to_string()));
            continue;
        }

        if let Some(ref pb) = count_progress {
            pb.set_message(sourcepath.to_string_lossy().to_string());
        }

        let targetpath = match sourcepath.file_name() {
            Some(name) => target_dir.join(name),
            None => {
                show!(MvError::NoSuchFile(sourcepath.quote().to_string()));
                continue;
            }
        };

        if moved_destinations.contains(&targetpath) && options.backup != BackupMode::NumberedBackup
        {
            // If the target file was already created in this mv call, do not overwrite
            show!(USimpleError::new(
                1,
                format!(
                    "will not overwrite just-created '{}' with '{}'",
                    targetpath.display(),
                    sourcepath.display()
                ),
            ));
            continue;
        }

        // Check if we have mv dir1 dir2 dir2
        // And generate an error if this is the case
        if let Ok(canonicalized_source) = sourcepath.canonicalize() {
            if canonicalized_source == canonicalized_target_dir {
                // User tried to move directory to itself, warning is shown
                // and process of moving files is continued.
                show!(USimpleError::new(
                    1,
                    format!(
                        "cannot move '{}' to a subdirectory of itself, '{}/{}'",
                        sourcepath.display(),
                        uucore::fs::normalize_path(target_dir).display(),
                        canonicalized_target_dir.components().last().map_or_else(
                            || target_dir.display().to_string(),
                            |dir| { PathBuf::from(dir.as_os_str()).display().to_string() }
                        )
                    )
                ));
                continue;
            }
        }

        match rename(sourcepath, &targetpath, options, multi_progress.as_ref()) {
            Err(e) if e.to_string().is_empty() => set_exit_code(1),
            Err(e) => {
                let e = e.map_err_context(|| {
                    format!(
                        "cannot move {} to {}",
                        sourcepath.quote(),
                        targetpath.quote()
                    )
                });
                match multi_progress {
                    Some(ref pb) => pb.suspend(|| show!(e)),
                    None => show!(e),
                };
            }
            Ok(()) => (),
        }
        if let Some(ref pb) = count_progress {
            pb.inc(1);
        }
        moved_destinations.insert(targetpath.clone());
    }
    Ok(())
}

fn rename(
    from: &Path,
    to: &Path,
    opts: &Options,
    multi_progress: Option<&MultiProgress>,
) -> io::Result<()> {
    let mut backup_path = None;

    // If `no-target-directory` is specified, we treat the destination as a file.
    // In that case, if there is a trailing forward slash, we remove it.
    let to = if path_ends_with_terminator(to) && opts.no_target_dir {
        let to_str = to.to_string_lossy();
        let trimmed_to = to_str.trim_end_matches(MAIN_SEPARATOR);
        Path::new(trimmed_to).to_path_buf()
    } else {
        to.to_path_buf()
    };

    let to = &to;

    if to.exists() {
        if opts.update == UpdateMode::ReplaceIfOlder && opts.overwrite == OverwriteMode::Interactive
        {
            // `mv -i --update old new` when `new` exists doesn't move anything
            // and exit with 0
            return Ok(());
        }

        if opts.update == UpdateMode::ReplaceNone {
            if opts.debug {
                println!("skipped {}", to.quote());
            }
            return Ok(());
        }

        if (opts.update == UpdateMode::ReplaceIfOlder)
            && fs::metadata(from)?.modified()? <= fs::metadata(to)?.modified()?
        {
            return Ok(());
        }

        if opts.update == UpdateMode::ReplaceNoneFail {
            let err_msg = format!("not replacing {}", to.quote());
            return Err(io::Error::new(io::ErrorKind::Other, err_msg));
        }

        match opts.overwrite {
            OverwriteMode::NoClobber => {
                if opts.debug {
                    println!("skipped {}", to.quote());
                }
                return Ok(());
            }
            OverwriteMode::Interactive => {
                if !prompt_yes!("overwrite {}?", to.quote()) {
                    return Err(io::Error::new(io::ErrorKind::Other, ""));
                }
            }
            OverwriteMode::Force => {}
        };

        backup_path = backup_control::get_backup_path(opts.backup, to, &opts.suffix);
        if let Some(ref backup_path) = backup_path {
            rename_with_fallback(to, backup_path, multi_progress, None)?;
        }
    }

    // "to" may no longer exist if it was backed up
    if to.exists() && to.is_dir() {
        // normalize behavior between *nix and windows
        if from.is_dir() {
            if is_empty_dir(to) {
                fs::remove_dir(to)?;
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "Directory not empty"));
            }
        }
    }

    let verbose_context = if opts.verbose {
        Some(VerboseContext::new(backup_path.as_deref(), multi_progress))
    } else {
        None
    };

    rename_with_fallback(from, to, multi_progress, verbose_context.as_ref())
}

/// A wrapper around `fs::rename`, so that if it fails, we try falling back on
/// copying and removing.
fn rename_with_fallback(
    from: &Path,
    to: &Path,
    multi_progress: Option<&MultiProgress>,
    verbose_context: Option<&VerboseContext<'_>>,
) -> io::Result<()> {
    if let Err(err) = fs::rename(from, to) {
        // Get metadata without following symlinks
        let metadata = from.symlink_metadata()?;
        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            rename_symlink_fallback(from, to)?;
            if let Some(vc) = verbose_context {
                vc.print_move_file(from, to);
            }
        } else if !matches!(err.raw_os_error(),Some(err_code)if err_code ==  CROSSES_DEVICES_ERROR_CODE)
        {
            // only try to copy if os reports an crosses devices error.
            return Err(err);
        } else if file_type.is_dir() {
            // We remove the destination directory if it exists to match the
            // behavior of `fs::rename`. As far as I can tell, `fs_extra`'s
            // `move_dir` would otherwise behave differently.
            if to.exists() {
                fs::remove_dir_all(to)?;
            }

            // Calculate total size of directory
            // Silently degrades:
            //    If finding the total size fails for whatever reason,
            //    the progress bar wont be shown for this file / dir.
            //    (Move will probably fail due to permission error later?)
            let mut progress_bar = None;
            if let Some(multi_progress) = multi_progress {
                if let Ok(total_size) = disk_usage(&[from], true) {
                    let bar = ProgressBar::new(total_size).with_style(
                        ProgressStyle::with_template(
                            "{msg}: [{elapsed_precise}] {wide_bar} {bytes:>7}/{total_bytes:7}",
                        )
                        .unwrap(),
                    );
                    progress_bar = Some(multi_progress.add(bar));
                }
            }

            let result = move_dir(from, to, progress_bar.as_ref(), verbose_context);

            if let Err(err) = result {
                return match err {
                    MvError::FsXError(fs_extra::error::Error {
                        kind: fs_extra::error::ErrorKind::PermissionDenied,
                        ..
                    }) => Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Permission denied",
                    )),
                    MvError::NotAllFilesMoved => {
                        Err(io::Error::new(io::ErrorKind::Other, String::new()))
                    }
                    _ => Err(io::Error::new(io::ErrorKind::Other, format!("{err:?}"))),
                };
            }
        } else {
            if to.is_symlink() {
                fs::remove_file(to).map_err(|err| {
                    let to = to.to_string_lossy();
                    let from = from.to_string_lossy();
                    io::Error::new(
                        err.kind(),
                        format!(
                            "inter-device move failed: '{from}' to '{to}'\
                            ; unable to remove target: {err}"
                        ),
                    )
                })?;
            }
            #[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
            fs::copy(from, to)
                .inspect(|_| {
                    if let Some(vc) = verbose_context {
                        vc.print_copy_file(from, to, true);
                    }
                })
                .and_then(|_| fsxattr::copy_xattrs(&from, &to))?;
            #[cfg(any(target_os = "macos", target_os = "redox", not(unix)))]
            fs::copy(from, to).inspect(|_| {
                if let Some(vc) = verbose_context {
                    vc.print_copy_file(from, to, true);
                }
            })?;
            fs::remove_file(from)?;
            if let Some(vc) = verbose_context {
                vc.remove_file(from);
            }
        }
    } else if let Some(vb) = verbose_context {
        vb.print_move_file(from, to);
    }
    Ok(())
}

/// Move the given symlink to the given destination. On Windows, dangling
/// symlinks return an error.
#[inline]
fn rename_symlink_fallback(from: &Path, to: &Path) -> io::Result<()> {
    let path_symlink_points_to = fs::read_link(from)?;
    #[cfg(unix)]
    {
        unix::fs::symlink(path_symlink_points_to, to).and_then(|_| fs::remove_file(from))?;
    }
    #[cfg(windows)]
    {
        if path_symlink_points_to.exists() {
            if path_symlink_points_to.is_dir() {
                windows::fs::symlink_dir(&path_symlink_points_to, to)?;
            } else {
                windows::fs::symlink_file(&path_symlink_points_to, to)?;
            }
            fs::remove_file(from)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "can't determine symlink type, since it is dangling",
            ));
        }
    }
    #[cfg(not(any(windows, unix)))]
    {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "your operating system does not support symlinks",
        ));
    }
    Ok(())
}

fn is_empty_dir(path: &Path) -> bool {
    match fs::read_dir(path) {
        Ok(contents) => contents.peekable().peek().is_none(),
        Err(_e) => false,
    }
}

/// Moves a directory from one location to another with progress tracking.
/// This function assumes that `from` is a directory and `to` does not exist.
///
/// Returns:
/// - `Result<u64>`: The total number of bytes moved if successful.
fn move_dir(
    from: &Path,
    to: &Path,
    progress_bar: Option<&ProgressBar>,
    verbose_context: Option<&VerboseContext<'_>>,
) -> MvResult<u64> {
    // The return value that represents the number of bytes copied.
    let mut result: u64 = 0;
    let mut error_occurred = false;
    let mut moved_entries: Vec<(PathBuf, fs::FileType, PathBuf, Option<fs::Metadata>, usize)> =
        vec![];
    for dir_entry_result in WalkDir::new(from) {
        match dir_entry_result {
            Ok(dir_entry) => {
                let file_type = dir_entry.file_type();
                let dir_entry_md = dir_entry.metadata().ok();
                let depth = dir_entry.depth();
                let dir_entry_path = dir_entry.into_path();
                let tmp_to = dir_entry_path.strip_prefix(from).unwrap();
                let dir_entry_to = to.join(tmp_to);
                if file_type.is_dir() {
                    let res = fs_extra::dir::create(&dir_entry_to, false);
                    if let Err(err) = res {
                        if let FsXErrorKind::NotFound = err.kind {
                            // This error would be thrown in the first iteration
                            // if the destination parent directory doesn't
                            // exist.
                            return Err(err.into());
                        }
                        error_occurred = true;
                        show_error!("{:?}", err);
                        continue;
                    }
                    if let Some(vc) = verbose_context {
                        vc.create_directory(&dir_entry_to);
                    }
                } else {
                    let res = copy_file(&dir_entry_path, &dir_entry_to, progress_bar, result);
                    match res {
                        Ok(copied_bytes) => {
                            result += copied_bytes;
                            if let Some(vc) = verbose_context {
                                vc.print_copy_file(&dir_entry_path, &dir_entry_to, false);
                            }
                        }
                        Err(err) => {
                            let err_msg = match err.kind {
                                FsXErrorKind::Io(error) => {
                                    format!("error writing {}: {}", dir_entry_to.quote(), error)
                                }
                                _ => {
                                    format!("{:?}", err)
                                }
                            };
                            show_error!("{}", err_msg);
                            error_occurred = true;
                            continue;
                        }
                    }
                }
                moved_entries.push((dir_entry_path, file_type, dir_entry_to, dir_entry_md, depth));
            }
            Err(err) => {
                let err_msg = match (err.io_error(), err.path()) {
                    (Some(io_error), Some(path)) => {
                        format!("cannot access {}: {io_error}", path.quote())
                    }
                    _ => err.to_string(),
                };
                show_error!("{err_msg}");
                error_occurred = true;
            }
        }
    }
    // if no error occurred try to remove source and copy metadata
    if !error_occurred {
        // GNU's `mv` only reports an error when it fails to remove a directory
        // entry. It doesn't print anything if it fails to remove the parent
        // directory of that entry.
        // in order to mimic that behavior, we need to remember where the last error occurred.
        let mut last_rem_err_depth: Option<usize> = None;
        while let Some((src_path, file_type, dest_path, src_md, depth)) = moved_entries.pop() {
            if let Some(src_metadata) = src_md {
                copy_metadata(&src_path, &dest_path, &src_metadata);
            }
            if matches!(last_rem_err_depth,Some(lred)if lred > depth) {
                // This means current dir entry is parent directory of a child
                // dir entry that couldn't be removed.

                // We mark current depth as the depth last error was occurred, this
                // would ensure that we won't ignore sibling dir entries of the
                // parent directory.
                last_rem_err_depth = Some(depth);
                // there's no point trying to remove a non empty directory.
                continue;
            }
            let res = if src_path.is_dir() {
                fs::remove_dir(&src_path)
            } else {
                fs::remove_file(&src_path)
            };
            if let Err(err) = res {
                error_occurred = true;
                show_error!("cannot remove {}: {}", src_path.quote(), err);
                last_rem_err_depth = Some(depth);
            } else if let Some(vc) = verbose_context {
                if file_type.is_dir() {
                    vc.remove_directory(&src_path);
                } else {
                    vc.remove_file(&src_path);
                }
            }
        }
    }
    if error_occurred {
        return Err(MvError::NotAllFilesMoved);
    }
    Ok(result)
}

/// Copies a file from one path to another, updating the progress bar if provided.
fn copy_file(
    from: &Path,
    to: &Path,
    progress_bar: Option<&ProgressBar>,
    progress_bar_start_val: u64,
) -> FsXResult<u64> {
    let copy_options: CopyOptions = CopyOptions {
        // We are overwriting here based on the assumption that the update and
        // override options are handled by a parent function call.
        overwrite: true,
        ..Default::default()
    };
    let progress_handler = if let Some(progress_bar) = progress_bar {
        let display_file_name = from
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .map(|file_name| file_name.to_string())
            .unwrap_or_default();
        let progress_handler = |info: file::TransitProcess| {
            let copied_bytes = progress_bar_start_val + info.copied_bytes;
            progress_bar.set_position(copied_bytes);
        };
        progress_bar.set_message(display_file_name);
        Some(progress_handler)
    } else {
        None
    };

    #[cfg(all(unix, not(target_os = "redox")))]
    {
        let md = from.metadata()?;
        if FileTypeExt::is_fifo(&md.file_type()) {
            let file_size = md.len();
            uucore::fs::create_fifo(to)?;
            if let Some(progress_bar) = progress_bar {
                progress_bar.set_position(file_size + progress_bar_start_val);
            }
            return Ok(file_size);
        }
    }
    if let Some(progress_handler) = progress_handler {
        file::copy_with_progress(from, to, &copy_options, progress_handler)
    } else {
        file::copy(from, to, &copy_options)
    }
}

#[allow(unused_variables)]
fn copy_metadata(src: &Path, dest: &Path, src_metadata: &fs::Metadata) {
    // Copy file permissions
    let permissions = src_metadata.permissions();
    fs::set_permissions(dest, permissions).ok();

    // Copy ownership (if on Unix-like system)
    #[cfg(unix)]
    {
        let uid = MetadataExt::uid(src_metadata);
        let gid = MetadataExt::gid(src_metadata);
        if let Ok(dest_md) = fs::symlink_metadata(dest).as_ref() {
            wrap_chown(
                dest,
                dest_md,
                Some(uid),
                Some(gid),
                false,
                Verbosity {
                    groups_only: false,
                    level: VerbosityLevel::Silent,
                },
            )
            .ok();
        }
    }

    // Copy the modified and accessed timestamps
    let modified_time = src_metadata.modified();
    let accessed_time = src_metadata.accessed();
    if let (Ok(modified_time), Ok(accessed_time)) = (modified_time, accessed_time) {
        set_symlink_file_times(dest, accessed_time.into(), modified_time.into()).ok();
    }

    // Copy xattrs.
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
    if let Err(err) = fsxattr::copy_xattrs(src, dest) {
        show_error!("preserving permissions for {}: {}", dest.quote(), err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copy_file;
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
    use fsxattr::{apply_xattrs, retrieve_xattrs};
    use indicatif::ProgressBar;
    use std::fs::{self, create_dir_all, File};
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};
    use std::thread::sleep;
    use std::time::Duration;
    use tempfile::tempdir;
    #[cfg(unix)]
    use uucore::fs::create_fifo;
    use uucore::fs::disk_usage;

    #[test]
    fn move_all_files_and_directories() {
        let tempdir = tempdir().expect("couldn't create tempdir");
        let tempdir_path = tempdir.path();
        let mut from = tempdir_path.to_path_buf();
        from.push("test_src");
        let mut to = tempdir_path.to_path_buf();
        to.push("test_dest");

        // Setup source directory with files and subdirectories
        create_dir_all(from.join("subdir")).expect("couldn't create subdir");
        let mut file = File::create(from.join("file1.txt")).expect("couldn't create file1.txt");
        writeln!(file, "Hello, world!").expect("couldn't write to file1.txt");
        let mut file =
            File::create(from.join("subdir/file2.txt")).expect("couldn't create subdir/file2.txt");
        writeln!(file, "Hello, subdir!").expect("couldn't write to subdir/file2.txt");

        // Call the function
        let result: MvResult<u64> = move_dir(&from, &to, None, None);

        // Assert the result
        assert!(result.is_ok());
        assert!(to.join("file1.txt").exists());
        assert!(to.join("subdir/file2.txt").exists());
        assert!(!from.join("file1.txt").exists());
        assert!(!from.join("subdir/file2.txt").exists());
        assert!(!from.exists());
    }

    #[test]
    fn move_dir_tracks_progress() {
        // Create a temporary directory for testing
        let tempdir = tempdir().expect("couldn't create tempdir");
        let tempdir_path = tempdir.path();
        let mut from = tempdir_path.to_path_buf();
        from.push("test_src");
        let mut to = tempdir_path.to_path_buf();
        to.push("test_dest");

        // Setup source directory with files and subdirectories
        create_dir_all(from.join("subdir")).expect("couldn't create subdir");
        {
            let mut file = File::create(from.join("file1.txt")).expect("couldn't create file1.txt");
            writeln!(file, "Hello, world!").expect("couldn't write to file1.txt");
            file.sync_all().unwrap();
        }
        {
            let mut file = File::create(from.join("subdir/file2.txt"))
                .expect("couldn't create subdir/file2.txt");
            writeln!(file, "Hello, subdir!").expect("couldn't write to subdir/file2.txt");
            file.sync_all().unwrap();
        }

        let len = disk_usage(&[&from], true).expect("couldn't get the size of source dir");
        let pb = ProgressBar::new(len);

        // Call the function
        let result: MvResult<u64> = move_dir(&from, &to, Some(&pb), None);

        // Assert the result
        assert!(result.is_ok());
        assert!(to.join("file1.txt").exists());
        assert!(to.join("subdir/file2.txt").exists());
        assert!(!from.join("file1.txt").exists());
        assert!(!from.join("subdir/file2.txt").exists());
        assert!(!from.exists());
        assert_eq!(pb.position(), len)
    }

    #[cfg(unix)]
    #[test]
    fn move_all_files_and_directories_without_src_permission() {
        let tempdir = tempdir().expect("couldn't create tempdir");
        let tempdir_path = tempdir.path();
        let mut from = tempdir_path.to_path_buf();
        from.push("test_src");
        let mut to = tempdir_path.to_path_buf();
        to.push("test_dest");

        // Setup source directory with files and subdirectories
        create_dir_all(from.join("subdir")).expect("couldn't create subdir");

        let mut file = File::create(from.join("file1.txt")).expect("couldn't create file1.txt");
        writeln!(file, "Hello, world!").expect("couldn't write to file1.txt");
        let mut file =
            File::create(from.join("subdir/file2.txt")).expect("couldn't create subdir/file2.txt");
        writeln!(file, "Hello, subdir!").expect("couldn't write to subdir/file2.txt");

        let metadata = fs::metadata(&from).expect("failed to get metadata");
        let mut permissions = metadata.permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o222);
        fs::set_permissions(&from, permissions).expect("failed to set permissions");

        // Call the function
        let result: MvResult<u64> = move_dir(&from, &to, None, None);
        assert!(matches!(result, Err(MvError::NotAllFilesMoved)));
        assert!(from.exists());
    }

    #[test]
    fn test_copy_file() {
        let temp_dir = tempdir().expect("couldn't create tempdir");
        let from = temp_dir.path().join("test_source.txt");
        let to = temp_dir.path().join("test_destination.txt");

        // Create a test source file
        let mut file = File::create(&from).expect("couldn't create file1.txt");
        write!(file, "Hello, world!").expect("couldn't write to file1.txt");

        // Call the function
        let result = copy_file(&from, &to, None, 0);

        // Assert the result is Ok and the file was copied
        assert!(result.is_ok());
        assert!(to.exists());
        assert_eq!(
            fs::read_to_string(to).expect("couldn't read from to"),
            "Hello, world!"
        );
    }
    #[test]
    fn test_copy_file_with_progress() {
        let temp_dir = tempdir().expect("couldn't create tempdir");
        let from = temp_dir.path().join("test_source.txt");
        let to = temp_dir.path().join("test_destination.txt");

        // Create a test source file
        let mut file = File::create(&from).expect("couldn't create file1.txt");
        write!(file, "Hello, world!").expect("couldn't write to file1.txt");

        let len = file
            .metadata()
            .expect("couldn't get source file metadata")
            .len();
        let pb = ProgressBar::new(len);

        // Call the function
        let result = copy_file(&from, &to, Some(&pb), 0);

        // Assert the result is Ok and the file was copied
        assert_eq!(pb.position(), len);
        assert!(result.is_ok());
        assert!(to.exists());
        assert_eq!(
            fs::read_to_string(to).expect("couldn't read from to"),
            "Hello, world!"
        );
    }

    #[cfg(all(unix, not(target_os = "redox")))]
    #[test]
    fn test_copy_file_with_fifo() {
        let temp_dir = tempdir().expect("couldn't create tempdir");
        let from = temp_dir.path().join("test_source.txt");
        let to = temp_dir.path().join("test_destination.txt");

        // Create a test source file
        create_fifo(&from).expect("couldn't create fifo");

        // Call the function
        let result = copy_file(&from, &to, None, 0);

        // Assert the result is Ok and the fifo was copied
        assert!(result.is_ok());
        assert!(to.exists());
        assert!(to
            .metadata()
            .expect("couldn't get metadata")
            .file_type()
            .is_fifo())
    }

    #[cfg(unix)]
    #[test]
    fn test_copy_metadata_copies_permissions() {
        let temp_dir = tempdir().unwrap();
        let src_path = temp_dir.path().join("src_file");
        let dest_path = temp_dir.path().join("dest_file");

        // Create source and destination files
        File::create(&src_path).unwrap();
        File::create(&dest_path).unwrap();

        // Set permissions for the source file
        let src_md = fs::metadata(&src_path).unwrap();
        let mut permissions = src_md.permissions();
        permissions.set_mode(0o100000);
        fs::set_permissions(&src_path, permissions.clone()).unwrap();
        let src_md = fs::metadata(&src_path).unwrap();

        // Call the function under test
        copy_metadata(&src_path, &dest_path, &src_md);

        // Verify that the permissions were copied
        let dest_permissions = fs::metadata(&dest_path).unwrap().permissions();
        assert_eq!(permissions.mode(), dest_permissions.mode());
    }

    #[test]
    fn test_copy_metadata_copies_file_times() {
        let temp_dir = tempdir().expect("couldn't create tempdir");
        let src_path = temp_dir.path().join("src_file");
        let dest_path = temp_dir.path().join("dest_file");

        // Create source and destination files
        File::create(&src_path).expect("couldn't create source file");
        // Wait for a second so that file times are different
        sleep(Duration::from_secs(1));
        File::create(&dest_path).expect("couldn't create dest file");

        // Get file times for the source file
        let src_metadata = fs::metadata(&src_path).expect("couldn't get metadata for source file");
        let modified_time = src_metadata
            .modified()
            .expect("couldn't get modified time for src file");
        let accessed_time = src_metadata
            .accessed()
            .expect("couldn't get accessed time for src file");

        //Try to copy metadata
        copy_metadata(&src_path, &dest_path, &src_metadata);

        // Get file times for the dest file
        let dest_metadata = fs::metadata(&dest_path).expect("couldn't get metadata for dest file");
        let dest_modified_time = dest_metadata
            .modified()
            .expect("couldn't get modified time for src file");
        let dest_accessed_time = dest_metadata
            .accessed()
            .expect("couldn't get accessed time for src file");

        // Verify that the file times were copied
        assert_eq!(modified_time, dest_modified_time);
        assert_eq!(dest_accessed_time, accessed_time);
    }

    #[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
    #[test]
    fn test_copy_metadata_copies_xattr() {
        let temp_dir = tempdir().expect("couldn't create tempdir");
        let src_path = temp_dir.path().join("src_file");
        let dest_path = temp_dir.path().join("dest_file");

        // Create source and destination files
        File::create(&src_path).expect("couldn't create source file");
        File::create(&dest_path).expect("couldn't create dest file");

        let src_metadata = fs::metadata(&src_path).unwrap();

        // Set xattrs for the source file
        let mut test_xattrs = std::collections::HashMap::new();
        let test_attr = "user.test_attr";
        let test_value = b"test value";
        test_xattrs.insert(OsString::from(test_attr), test_value.to_vec());
        apply_xattrs(&src_path, test_xattrs).expect("couldn't apply xattr to the destination file");

        //Try to copy metadata
        copy_metadata(&src_path, &dest_path, &src_metadata);

        // Verify that the xattrs were copied
        let retrieved_xattrs = retrieve_xattrs(&dest_path).unwrap();
        assert!(retrieved_xattrs.contains_key(OsString::from(test_attr).as_os_str()));
        assert_eq!(
            retrieved_xattrs
                .get(OsString::from(test_attr).as_os_str())
                .expect("couldn't find xattr with name user.test_attr"),
            test_value
        );
    }
}
