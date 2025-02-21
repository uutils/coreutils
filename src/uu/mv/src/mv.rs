// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) sourcepath targetpath nushell canonicalized

mod error;

use clap::builder::ValueParser;
use clap::{crate_version, error::ErrorKind, Arg, ArgAction, ArgMatches, Command};
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
use std::path::{absolute, Path, PathBuf};
use uucore::backup_control::{self, source_is_target_backup};
use uucore::display::Quotable;
use uucore::error::{set_exit_code, FromIo, UResult, USimpleError, UUsageError};
use uucore::fs::{
    are_hardlinks_or_one_way_symlink_to_same_file, are_hardlinks_to_same_file, canonicalize,
    path_ends_with_terminator, MissingHandling, ResolveMode,
};
#[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
use uucore::fsxattr;
use uucore::update_control;

// These are exposed for projects (e.g. nushell) that want to create an `Options` value, which
// requires these enums
pub use uucore::{backup_control::BackupMode, update_control::UpdateMode};
use uucore::{format_usage, help_about, help_section, help_usage, prompt_yes, show};

use fs_extra::dir::{
    get_size as dir_get_size, move_dir, move_dir_with_progress, CopyOptions as DirCopyOptions,
    TransitProcess, TransitProcessResult,
};

use crate::error::MvError;

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

const ABOUT: &str = help_about!("mv.md");
const USAGE: &str = help_usage!("mv.md");
const AFTER_HELP: &str = help_section!("after help", "mv.md");

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

    let target_is_dir = target.is_dir();
    let source_is_dir = source.is_dir();

    if path_ends_with_terminator(target)
        && (!target_is_dir && !source_is_dir)
        && !opts.no_target_dir
        && opts.update != UpdateMode::ReplaceIfOlder
    {
        return Err(MvError::FailedToAccessNotADirectory(target.quote().to_string()).into());
    }

    assert_not_same_file(source, target, target_is_dir, opts)?;

    if target_is_dir {
        if opts.no_target_dir {
            if source.is_dir() {
                rename(source, target, opts, None).map_err_context(|| {
                    format!("cannot move {} to {}", source.quote(), target.quote())
                })
            } else {
                Err(MvError::DirectoryToNonDirectory(target.quote().to_string()).into())
            }
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

fn assert_not_same_file(
    source: &Path,
    target: &Path,
    target_is_dir: bool,
    opts: &Options,
) -> UResult<()> {
    // we'll compare canonicalized_source and canonicalized_target for same file detection
    let canonicalized_source = match canonicalize(
        absolute(source)?,
        MissingHandling::Normal,
        ResolveMode::Logical,
    ) {
        Ok(source) if source.exists() => source,
        _ => absolute(source)?, // file or symlink target doesn't exist but its absolute path is still used for comparison
    };

    // special case if the target exists, is a directory, and the `-T` flag wasn't used
    let target_is_dir = target_is_dir && !opts.no_target_dir;
    let canonicalized_target = if target_is_dir {
        // `mv source_file target_dir` => target_dir/source_file
        // canonicalize the path that exists (target directory) and join the source file name
        canonicalize(
            absolute(target)?,
            MissingHandling::Normal,
            ResolveMode::Logical,
        )?
        .join(source.file_name().unwrap_or_default())
    } else {
        // `mv source target_dir/target` => target_dir/target
        // we canonicalize target_dir and join /target
        match absolute(target)?.parent() {
            Some(parent) if parent.to_str() != Some("") => {
                canonicalize(parent, MissingHandling::Normal, ResolveMode::Logical)?
                    .join(target.file_name().unwrap_or_default())
            }
            // path.parent() returns Some("") or None if there's no parent
            _ => absolute(target)?, // absolute paths should always have a parent, but we'll fall back just in case
        }
    };

    let same_file = (canonicalized_source.eq(&canonicalized_target)
        || are_hardlinks_to_same_file(source, target)
        || are_hardlinks_or_one_way_symlink_to_same_file(source, target))
        && opts.backup == BackupMode::NoBackup;

    // get the expected target path to show in errors
    // this is based on the argument and not canonicalized
    let target_display = match source.file_name() {
        Some(file_name) if target_is_dir => {
            // join target_dir/source_file in a platform-independent manner
            let mut path = target
                .display()
                .to_string()
                .trim_end_matches("/")
                .to_owned();

            path.push('/');
            path.push_str(&file_name.to_string_lossy());

            path.quote().to_string()
        }
        _ => target.quote().to_string(),
    };

    if same_file
        && (canonicalized_source.eq(&canonicalized_target)
            || source.eq(Path::new("."))
            || source.ends_with("/.")
            || source.is_file())
    {
        return Err(MvError::SameFile(source.quote().to_string(), target_display).into());
    } else if (same_file || canonicalized_target.starts_with(canonicalized_source))
        // don't error if we're moving a symlink of a directory into itself
        && !source.is_symlink()
    {
        return Err(
            MvError::SelfTargetSubdirectory(source.quote().to_string(), target_display).into(),
        );
    }
    Ok(())
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
        if let Err(e) = assert_not_same_file(sourcepath, target_dir, true, options) {
            show!(e);
            continue;
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

    if to.exists() {
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
            rename_with_fallback(to, backup_path, multi_progress)?;
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

    rename_with_fallback(from, to, multi_progress)?;

    if opts.verbose {
        let message = match backup_path {
            Some(path) => format!(
                "renamed {} -> {} (backup: {})",
                from.quote(),
                to.quote(),
                path.quote()
            ),
            None => format!("renamed {} -> {}", from.quote(), to.quote()),
        };

        match multi_progress {
            Some(pb) => pb.suspend(|| {
                println!("{message}");
            }),
            None => println!("{message}"),
        };
    }
    Ok(())
}

/// A wrapper around `fs::rename`, so that if it fails, we try falling back on
/// copying and removing.
fn rename_with_fallback(
    from: &Path,
    to: &Path,
    multi_progress: Option<&MultiProgress>,
) -> io::Result<()> {
    if let Err(err) = fs::rename(from, to) {
        #[cfg(windows)]
        const EXDEV: i32 = windows_sys::Win32::Foundation::ERROR_NOT_SAME_DEVICE as _;
        #[cfg(unix)]
        const EXDEV: i32 = libc::EXDEV as _;

        // We will only copy if:
        // 1. Files are on different devices (EXDEV error)
        // 2. On Windows, if the target file exists and source file is opened by another process
        //    (MoveFileExW fails with "Access Denied" even if the source file has FILE_SHARE_DELETE permission)
        let should_fallback = matches!(err.raw_os_error(), Some(EXDEV))
            || (from.is_file() && can_delete_file(from).unwrap_or(false));
        if !should_fallback {
            return Err(err);
        }

        // Get metadata without following symlinks
        let metadata = from.symlink_metadata()?;
        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            rename_symlink_fallback(from, to)?;
        } else if file_type.is_dir() {
            // We remove the destination directory if it exists to match the
            // behavior of `fs::rename`. As far as I can tell, `fs_extra`'s
            // `move_dir` would otherwise behave differently.
            if to.exists() {
                fs::remove_dir_all(to)?;
            }
            let options = DirCopyOptions {
                // From the `fs_extra` documentation:
                // "Recursively copy a directory with a new name or place it
                // inside the destination. (same behaviors like cp -r in Unix)"
                copy_inside: true,
                ..DirCopyOptions::new()
            };

            // Calculate total size of directory
            // Silently degrades:
            //    If finding the total size fails for whatever reason,
            //    the progress bar wont be shown for this file / dir.
            //    (Move will probably fail due to permission error later?)
            let total_size = dir_get_size(from).ok();

            let progress_bar =
                if let (Some(multi_progress), Some(total_size)) = (multi_progress, total_size) {
                    let bar = ProgressBar::new(total_size).with_style(
                        ProgressStyle::with_template(
                            "{msg}: [{elapsed_precise}] {wide_bar} {bytes:>7}/{total_bytes:7}",
                        )
                        .unwrap(),
                    );

                    Some(multi_progress.add(bar))
                } else {
                    None
                };

            #[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
            let xattrs =
                fsxattr::retrieve_xattrs(from).unwrap_or_else(|_| std::collections::HashMap::new());

            let result = if let Some(ref pb) = progress_bar {
                move_dir_with_progress(from, to, &options, |process_info: TransitProcess| {
                    pb.set_position(process_info.copied_bytes);
                    pb.set_message(process_info.file_name);
                    TransitProcessResult::ContinueOrAbort
                })
            } else {
                move_dir(from, to, &options)
            };

            #[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
            fsxattr::apply_xattrs(to, xattrs)?;

            if let Err(err) = result {
                return match err.kind {
                    fs_extra::error::ErrorKind::PermissionDenied => Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Permission denied",
                    )),
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
                .and_then(|_| fsxattr::copy_xattrs(&from, &to))
                .and_then(|_| fs::remove_file(from))?;
            #[cfg(any(target_os = "macos", target_os = "redox", not(unix)))]
            fs::copy(from, to).and_then(|_| fs::remove_file(from))?;
        }
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

/// Checks if a file can be deleted by attempting to open it with delete permissions.
#[cfg(windows)]
fn can_delete_file(path: &Path) -> Result<bool, io::Error> {
    use std::{
        os::windows::ffi::OsStrExt as _,
        ptr::{null, null_mut},
    };

    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, DELETE, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, OPEN_EXISTING,
        },
    };

    let wide_path = path
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect::<Vec<u16>>();

    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            DELETE,
            FILE_SHARE_DELETE | FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }

    unsafe { CloseHandle(handle) };

    Ok(true)
}

#[cfg(not(windows))]
fn can_delete_file(_: &Path) -> Result<bool, io::Error> {
    // On non-Windows platforms, always return false to indicate that we don't need
    // to try the copy+delete fallback. This is because on Unix-like systems,
    // rename() failing with errors other than EXDEV means the operation cannot
    // succeed even with a copy+delete approach (e.g. permission errors).
    Ok(false)
}
