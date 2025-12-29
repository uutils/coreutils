// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) sourcepath targetpath nushell canonicalized unwriteable

mod error;
#[cfg(unix)]
mod hardlink;

use clap::builder::ValueParser;
use clap::error::ErrorKind;
use clap::{Arg, ArgAction, ArgMatches, Command};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

#[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, IsTerminal};
#[cfg(unix)]
use std::os::unix;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
#[cfg(windows)]
use std::os::windows;
use std::path::{Path, PathBuf, absolute};

#[cfg(unix)]
use crate::hardlink::{
    HardlinkGroupScanner, HardlinkOptions, HardlinkTracker, create_hardlink_context,
    with_optional_hardlink_context,
};
use uucore::backup_control::{self, source_is_target_backup};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError, set_exit_code};
#[cfg(unix)]
use uucore::fs::display_permissions_unix;
#[cfg(unix)]
use uucore::fs::make_fifo;
use uucore::fs::{
    MissingHandling, ResolveMode, are_hardlinks_or_one_way_symlink_to_same_file,
    are_hardlinks_to_same_file, canonicalize, path_ends_with_terminator,
};
#[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
use uucore::fsxattr;
#[cfg(feature = "selinux")]
use uucore::selinux::set_selinux_security_context;
use uucore::translate;
use uucore::update_control;

// These are exposed for projects (e.g. nushell) that want to create an `Options` value, which
// requires these enums
pub use uucore::{backup_control::BackupMode, update_control::UpdateMode};
use uucore::{format_usage, prompt_yes, show};

use fs_extra::dir::get_size as dir_get_size;

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

    /// `-Z, --context`
    pub context: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            overwrite: OverwriteMode::default(),
            backup: BackupMode::default(),
            suffix: backup_control::DEFAULT_BACKUP_SUFFIX.to_owned(),
            update: UpdateMode::default(),
            target_dir: None,
            no_target_dir: false,
            verbose: false,
            strip_slashes: false,
            progress_bar: false,
            debug: false,
            context: None,
        }
    }
}

/// specifies behavior of the overwrite flag
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub enum OverwriteMode {
    /// No flag specified - prompt for unwriteable files when stdin is TTY
    #[default]
    Default,
    /// '-n' '--no-clobber'   do not overwrite
    NoClobber,
    /// '-i' '--interactive'  prompt before overwrite
    Interactive,
    ///'-f' '--force'         overwrite without prompt
    Force,
}

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
static OPT_CONTEXT: &str = "context";
static OPT_SELINUX: &str = "selinux";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let files: Vec<OsString> = matches
        .get_many::<OsString>(ARG_FILES)
        .unwrap_or_default()
        .cloned()
        .collect();

    if files.len() == 1 && !matches.contains_id(OPT_TARGET_DIRECTORY) {
        let err = uu_app().error(
            ErrorKind::TooFewValues,
            translate!("mv-error-insufficient-arguments", "arg_files" => ARG_FILES),
        );
        uucore::clap_localization::handle_clap_error_with_exit_code(err, 1);
    }

    let overwrite_mode = determine_overwrite_mode(&matches);
    let backup_mode = backup_control::determine_backup_mode(&matches)?;
    let update_mode = update_control::determine_update_mode(&matches);

    if backup_mode != BackupMode::None
        && (overwrite_mode == OverwriteMode::NoClobber
            || update_mode == UpdateMode::None
            || update_mode == UpdateMode::NoneFail)
    {
        return Err(UUsageError::new(
            1,
            translate!("mv-error-backup-with-no-clobber"),
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

    // Handle -Z and --context options
    // If -Z is used, use the default context (empty string)
    // If --context=value is used, use that specific value
    let context = if matches.get_flag(OPT_SELINUX) {
        Some(String::new())
    } else {
        matches.get_one::<String>(OPT_CONTEXT).cloned()
    };

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
        context,
    };

    mv(&files[..], &opts)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("mv-about"))
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("mv-usage")))
        .after_help(format!(
            "{}\n\n{}",
            translate!("mv-after-help"),
            backup_control::BACKUP_CONTROL_LONG_HELP
        ))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_FORCE)
                .short('f')
                .long(OPT_FORCE)
                .help(translate!("mv-help-force"))
                .overrides_with_all([OPT_INTERACTIVE, OPT_NO_CLOBBER])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_INTERACTIVE)
                .short('i')
                .long(OPT_INTERACTIVE)
                .help(translate!("mv-help-interactive"))
                .overrides_with_all([OPT_FORCE, OPT_NO_CLOBBER])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_NO_CLOBBER)
                .short('n')
                .long(OPT_NO_CLOBBER)
                .help(translate!("mv-help-no-clobber"))
                .overrides_with_all([OPT_FORCE, OPT_INTERACTIVE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_STRIP_TRAILING_SLASHES)
                .long(OPT_STRIP_TRAILING_SLASHES)
                .help(translate!("mv-help-strip-trailing-slashes"))
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
                .help(translate!("mv-help-target-directory"))
                .value_name("DIRECTORY")
                .value_hint(clap::ValueHint::DirPath)
                .conflicts_with(OPT_NO_TARGET_DIRECTORY)
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(OPT_NO_TARGET_DIRECTORY)
                .short('T')
                .long(OPT_NO_TARGET_DIRECTORY)
                .help(translate!("mv-help-no-target-directory"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_VERBOSE)
                .short('v')
                .long(OPT_VERBOSE)
                .help(translate!("mv-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PROGRESS)
                .short('g')
                .long(OPT_PROGRESS)
                .help(translate!("mv-help-progress"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_SELINUX)
                .short('Z')
                .help(translate!("mv-help-selinux"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_CONTEXT)
                .long(OPT_CONTEXT)
                .value_name("CTX")
                .value_parser(clap::value_parser!(String))
                .help(translate!("mv-help-context"))
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value(""),
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
                .help(translate!("mv-help-debug"))
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
    } else if matches.get_flag(OPT_FORCE) {
        OverwriteMode::Force
    } else {
        OverwriteMode::Default
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
    if opts.backup == BackupMode::Simple && source_is_target_backup(source, target, &opts.suffix) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            translate!("mv-error-backup-might-destroy-source", "target" => target.quote(), "source" => source.quote()),
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

    let source_is_dir = source.is_dir() && !source.is_symlink();
    let target_is_dir = if target.is_symlink() {
        fs::canonicalize(target).is_ok_and(|p| p.is_dir())
    } else {
        target.is_dir()
    };

    if path_ends_with_terminator(target)
        && (!target_is_dir && !source_is_dir)
        && !opts.no_target_dir
        && opts.update != UpdateMode::IfOlder
    {
        return Err(MvError::FailedToAccessNotADirectory(target.quote().to_string()).into());
    }

    assert_not_same_file(source, target, target_is_dir, opts)?;

    if target_is_dir {
        if opts.no_target_dir {
            if source.is_dir() {
                #[cfg(unix)]
                let (mut hardlink_tracker, hardlink_scanner) = create_hardlink_context();
                #[cfg(unix)]
                let hardlink_params = (Some(&mut hardlink_tracker), Some(&hardlink_scanner));
                #[cfg(not(unix))]
                let hardlink_params = (None, None);

                rename(
                    source,
                    target,
                    opts,
                    None,
                    hardlink_params.0,
                    hardlink_params.1,
                )
                .map_err_context(|| {
                    translate!("mv-error-cannot-move", "source" => source.quote(), "target" => target.quote())
                })
            } else {
                Err(MvError::DirectoryToNonDirectory(target.quote().to_string()).into())
            }
        } else {
            move_files_into_dir(&[source.to_path_buf()], target, opts)
        }
    } else if target.exists() && source_is_dir {
        match opts.overwrite {
            OverwriteMode::NoClobber => return Ok(()),
            OverwriteMode::Interactive => prompt_overwrite(target, None)?,
            OverwriteMode::Force => {}
            OverwriteMode::Default => {
                let (writable, mode) = is_writable(target);
                if !writable && std::io::stdin().is_terminal() {
                    prompt_overwrite(target, mode)?;
                }
            }
        }
        Err(MvError::NonDirectoryToDirectory(
            source.quote().to_string(),
            target.quote().to_string(),
        )
        .into())
    } else {
        #[cfg(unix)]
        let (mut hardlink_tracker, hardlink_scanner) = create_hardlink_context();
        #[cfg(unix)]
        let hardlink_params = (Some(&mut hardlink_tracker), Some(&hardlink_scanner));
        #[cfg(not(unix))]
        let hardlink_params = (None, None);

        rename(
            source,
            target,
            opts,
            None,
            hardlink_params.0,
            hardlink_params.1,
        )
        .map_err(|e| USimpleError::new(1, format!("{e}")))
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
        && opts.backup == BackupMode::None;

    // get the expected target path to show in errors
    // this is based on the argument and not canonicalized
    let target_display = match source.file_name() {
        Some(file_name) if target_is_dir => {
            // join target_dir/source_file in a platform-independent manner
            let mut path = target
                .display()
                .to_string()
                .trim_end_matches('/')
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
            translate!("mv-error-extra-operand", "operand" => paths.last().unwrap().quote()),
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
    // Create hardlink tracking context
    #[cfg(unix)]
    let (mut hardlink_tracker, hardlink_scanner) = {
        let (tracker, mut scanner) = create_hardlink_context();

        // Use hardlink options
        let hardlink_options = HardlinkOptions {
            verbose: options.verbose || options.debug,
        };

        // Pre-scan files if needed
        if let Err(e) = scanner.scan_files(files, &hardlink_options) {
            if hardlink_options.verbose {
                eprintln!("mv: warning: failed to scan files for hardlinks: {e}");
                eprintln!("mv: continuing without hardlink preservation");
            } else {
                // Show warning in non-verbose mode for serious errors
                eprintln!(
                    "mv: warning: hardlink scanning failed, continuing without hardlink preservation"
                );
            }
            // Continue without hardlink tracking on scan failure
            // This provides graceful degradation rather than failing completely
        }

        (tracker, scanner)
    };

    if !target_dir.is_dir() {
        return Err(MvError::NotADirectory(target_dir.quote().to_string()).into());
    }

    let display_manager = options.progress_bar.then(MultiProgress::new);

    let count_progress = if let Some(ref display_manager) = display_manager {
        if files.len() > 1 {
            Some(
                display_manager.add(
                    ProgressBar::new(files.len().try_into().unwrap()).with_style(
                        ProgressStyle::with_template(&format!(
                            "{} {{msg}} {{wide_bar}} {{pos}}/{{len}}",
                            translate!("mv-progress-moving")
                        ))
                        .unwrap(),
                    ),
                ),
            )
        } else {
            None
        }
    } else {
        None
    };

    for sourcepath in files {
        if sourcepath.symlink_metadata().is_err() {
            show!(MvError::NoSuchFile(sourcepath.quote().to_string()));
            continue;
        }

        if let Some(ref pb) = count_progress {
            let msg = format!("{} (scanning hardlinks)", sourcepath.to_string_lossy());
            pb.set_message(msg);
        }

        let targetpath = match sourcepath.file_name() {
            Some(name) => target_dir.join(name),
            None => {
                show!(MvError::NoSuchFile(sourcepath.quote().to_string()));
                continue;
            }
        };

        if moved_destinations.contains(&targetpath) && options.backup != BackupMode::Numbered {
            // If the target file was already created in this mv call, do not overwrite
            show!(USimpleError::new(
                1,
                translate!("mv-error-will-not-overwrite-just-created", "target" => targetpath.quote(), "source" => sourcepath.quote()),
            ));
            continue;
        }

        // Check if we have mv dir1 dir2 dir2
        // And generate an error if this is the case
        if let Err(e) = assert_not_same_file(sourcepath, target_dir, true, options) {
            show!(e);
            continue;
        }

        #[cfg(unix)]
        let hardlink_params = (Some(&mut hardlink_tracker), Some(&hardlink_scanner));
        #[cfg(not(unix))]
        let hardlink_params = (None, None);

        match rename(
            sourcepath,
            &targetpath,
            options,
            display_manager.as_ref(),
            hardlink_params.0,
            hardlink_params.1,
        ) {
            Err(e) if e.to_string().is_empty() => set_exit_code(1),
            Err(e) => {
                let e = e.map_err_context(|| {
                    translate!("mv-error-cannot-move", "source" => sourcepath.quote(), "target" => targetpath.quote())
                });
                match display_manager {
                    Some(ref pb) => pb.suspend(|| show!(e)),
                    None => show!(e),
                }
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
    display_manager: Option<&MultiProgress>,
    #[cfg(unix)] hardlink_tracker: Option<&mut HardlinkTracker>,
    #[cfg(unix)] hardlink_scanner: Option<&HardlinkGroupScanner>,
    #[cfg(not(unix))] _hardlink_tracker: Option<()>,
    #[cfg(not(unix))] _hardlink_scanner: Option<()>,
) -> io::Result<()> {
    let mut backup_path = None;

    if to.exists() {
        if opts.update == UpdateMode::None {
            if opts.debug {
                println!("{}", translate!("mv-debug-skipped", "target" => to.quote()));
            }
            return Ok(());
        }

        if (opts.update == UpdateMode::IfOlder)
            && fs::metadata(from)?.modified()? <= fs::metadata(to)?.modified()?
        {
            return Ok(());
        }

        if opts.update == UpdateMode::NoneFail {
            let err_msg = translate!("mv-error-not-replacing", "target" => to.quote());
            return Err(io::Error::other(err_msg));
        }

        match opts.overwrite {
            OverwriteMode::NoClobber => {
                if opts.debug {
                    println!("{}", translate!("mv-debug-skipped", "target" => to.quote()));
                }
                return Ok(());
            }
            OverwriteMode::Interactive => prompt_overwrite(to, None)?,
            OverwriteMode::Force => {}
            OverwriteMode::Default => {
                // GNU mv prompts when stdin is a TTY and target is not writable
                let (writable, mode) = is_writable(to);
                if !writable && std::io::stdin().is_terminal() {
                    prompt_overwrite(to, mode)?;
                }
            }
        }

        backup_path = backup_control::get_backup_path(opts.backup, to, &opts.suffix);
        if let Some(ref backup_path) = backup_path {
            // For backup renames, we don't need to track hardlinks as we're just moving the existing file
            rename_with_fallback(to, backup_path, display_manager, false, None, None)?;
        }
    }

    // "to" may no longer exist if it was backed up
    if to.exists() && to.is_dir() && !to.is_symlink() {
        // normalize behavior between *nix and windows
        if from.is_dir() {
            if is_empty_dir(to) {
                fs::remove_dir(to)?;
            } else {
                return Err(io::Error::other(translate!("mv-error-directory-not-empty")));
            }
        }
    }

    #[cfg(unix)]
    {
        rename_with_fallback(
            from,
            to,
            display_manager,
            opts.verbose,
            hardlink_tracker,
            hardlink_scanner,
        )?;
    }
    #[cfg(not(unix))]
    {
        rename_with_fallback(from, to, display_manager, opts.verbose, None, None)?;
    }

    #[cfg(feature = "selinux")]
    if let Some(ref context) = opts.context {
        set_selinux_security_context(to, Some(context))
            .map_err(|e| io::Error::other(e.to_string()))?;
    }

    if opts.verbose {
        let message = match backup_path {
            Some(path) => {
                translate!("mv-verbose-renamed-with-backup", "from" => from.quote(), "to" => to.quote(), "backup" => path.quote())
            }
            None => translate!("mv-verbose-renamed", "from" => from.quote(), "to" => to.quote()),
        };

        match display_manager {
            Some(pb) => pb.suspend(|| {
                println!("{message}");
            }),
            None => println!("{message}"),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn is_fifo(filetype: fs::FileType) -> bool {
    filetype.is_fifo()
}

#[cfg(not(unix))]
fn is_fifo(_filetype: fs::FileType) -> bool {
    false
}

/// A wrapper around `fs::rename`, so that if it fails, we try falling back on
/// copying and removing.
fn rename_with_fallback(
    from: &Path,
    to: &Path,
    display_manager: Option<&MultiProgress>,
    verbose: bool,
    #[cfg(unix)] hardlink_tracker: Option<&mut HardlinkTracker>,
    #[cfg(unix)] hardlink_scanner: Option<&HardlinkGroupScanner>,
    #[cfg(not(unix))] _hardlink_tracker: Option<()>,
    #[cfg(not(unix))] _hardlink_scanner: Option<()>,
) -> io::Result<()> {
    fs::rename(from, to).or_else(|err| {
        #[cfg(windows)]
        const EXDEV: i32 = windows_sys::Win32::Foundation::ERROR_NOT_SAME_DEVICE as _;
        #[cfg(unix)]
        const EXDEV: i32 = libc::EXDEV as _;

        // We will only copy if:
        // 1. Files are on different devices (EXDEV error)
        // 2. On Windows, if the target file exists and source file is opened by another process
        //    (MoveFileExW fails with "Access Denied" even if the source file has FILE_SHARE_DELETE permission)
        let should_fallback =
            matches!(err.raw_os_error(), Some(EXDEV)) || (from.is_file() && can_delete_file(from));
        if !should_fallback {
            return Err(err);
        }
        // Get metadata without following symlinks
        let metadata = from.symlink_metadata()?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            rename_symlink_fallback(from, to)
        } else if file_type.is_dir() {
            #[cfg(unix)]
            {
                with_optional_hardlink_context(
                    hardlink_tracker,
                    hardlink_scanner,
                    |tracker, scanner| {
                        rename_dir_fallback(
                            from,
                            to,
                            display_manager,
                            verbose,
                            Some(tracker),
                            Some(scanner),
                        )
                    },
                )
            }
            #[cfg(not(unix))]
            {
                rename_dir_fallback(from, to, display_manager, verbose)
            }
        } else if is_fifo(file_type) {
            rename_fifo_fallback(from, to)
        } else {
            #[cfg(unix)]
            {
                with_optional_hardlink_context(
                    hardlink_tracker,
                    hardlink_scanner,
                    |tracker, scanner| rename_file_fallback(from, to, Some(tracker), Some(scanner)),
                )
            }
            #[cfg(not(unix))]
            {
                rename_file_fallback(from, to)
            }
        }
    })
}

/// Replace the destination with a new pipe with the same name as the source.
#[cfg(unix)]
fn rename_fifo_fallback(from: &Path, to: &Path) -> io::Result<()> {
    if to.try_exists()? {
        fs::remove_file(to)?;
    }
    make_fifo(to).and_then(|_| fs::remove_file(from))
}

#[cfg(not(unix))]
fn rename_fifo_fallback(_from: &Path, _to: &Path) -> io::Result<()> {
    Ok(())
}

/// Move the given symlink to the given destination. On Windows, dangling
/// symlinks return an error.
#[cfg(unix)]
fn rename_symlink_fallback(from: &Path, to: &Path) -> io::Result<()> {
    let path_symlink_points_to = fs::read_link(from)?;
    unix::fs::symlink(path_symlink_points_to, to).and_then(|_| fs::remove_file(from))
}

#[cfg(windows)]
fn rename_symlink_fallback(from: &Path, to: &Path) -> io::Result<()> {
    let path_symlink_points_to = fs::read_link(from)?;
    if path_symlink_points_to.exists() {
        if path_symlink_points_to.is_dir() {
            windows::fs::symlink_dir(&path_symlink_points_to, to)?;
        } else {
            windows::fs::symlink_file(&path_symlink_points_to, to)?;
        }
        fs::remove_file(from)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            translate!("mv-error-dangling-symlink"),
        ))
    }
}

#[cfg(not(any(windows, unix)))]
fn rename_symlink_fallback(from: &Path, to: &Path) -> io::Result<()> {
    let path_symlink_points_to = fs::read_link(from)?;
    Err(io::Error::new(
        io::ErrorKind::Other,
        translate!("mv-error-no-symlink-support"),
    ))
}

fn rename_dir_fallback(
    from: &Path,
    to: &Path,
    display_manager: Option<&MultiProgress>,
    verbose: bool,
    #[cfg(unix)] hardlink_tracker: Option<&mut HardlinkTracker>,
    #[cfg(unix)] hardlink_scanner: Option<&HardlinkGroupScanner>,
) -> io::Result<()> {
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
    let total_size = dir_get_size(from).ok();

    let progress_bar = match (display_manager, total_size) {
        (Some(display_manager), Some(total_size)) => {
            let template = "{msg}: [{elapsed_precise}] {wide_bar} {bytes:>7}/{total_bytes:7}";
            let style = ProgressStyle::with_template(template).unwrap();
            let bar = ProgressBar::new(total_size).with_style(style);
            Some(display_manager.add(bar))
        }
        (_, _) => None,
    };

    #[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
    let xattrs = fsxattr::retrieve_xattrs(from).unwrap_or_else(|_| HashMap::new());

    // Use directory copying (with or without hardlink support)
    let result = copy_dir_contents(
        from,
        to,
        #[cfg(unix)]
        hardlink_tracker,
        #[cfg(unix)]
        hardlink_scanner,
        verbose,
        progress_bar.as_ref(),
        display_manager,
    );

    #[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
    fsxattr::apply_xattrs(to, xattrs)?;

    result?;

    // Remove the source directory after successful copy
    fs::remove_dir_all(from)?;

    Ok(())
}

/// Copy directory recursively, optionally preserving hardlinks
fn copy_dir_contents(
    from: &Path,
    to: &Path,
    #[cfg(unix)] hardlink_tracker: Option<&mut HardlinkTracker>,
    #[cfg(unix)] hardlink_scanner: Option<&HardlinkGroupScanner>,
    verbose: bool,
    progress_bar: Option<&ProgressBar>,
    display_manager: Option<&MultiProgress>,
) -> io::Result<()> {
    // Create the destination directory
    fs::create_dir_all(to)?;

    // Recursively copy contents
    #[cfg(unix)]
    {
        if let (Some(tracker), Some(scanner)) = (hardlink_tracker, hardlink_scanner) {
            copy_dir_contents_recursive(
                from,
                to,
                tracker,
                scanner,
                verbose,
                progress_bar,
                display_manager,
            )?;
        }
    }
    #[cfg(not(unix))]
    {
        copy_dir_contents_recursive(from, to, None, None, verbose, progress_bar, display_manager)?;
    }

    Ok(())
}

fn copy_dir_contents_recursive(
    from_dir: &Path,
    to_dir: &Path,
    #[cfg(unix)] hardlink_tracker: &mut HardlinkTracker,
    #[cfg(unix)] hardlink_scanner: &HardlinkGroupScanner,
    #[cfg(not(unix))] _hardlink_tracker: Option<()>,
    #[cfg(not(unix))] _hardlink_scanner: Option<()>,
    verbose: bool,
    progress_bar: Option<&ProgressBar>,
    display_manager: Option<&MultiProgress>,
) -> io::Result<()> {
    let entries = fs::read_dir(from_dir)?;

    for entry in entries {
        let entry = entry?;
        let from_path = entry.path();
        let file_name = from_path.file_name().unwrap();
        let to_path = to_dir.join(file_name);

        if let Some(pb) = progress_bar {
            pb.set_message(from_path.to_string_lossy().to_string());
        }

        if from_path.is_dir() {
            // Recursively copy subdirectory
            fs::create_dir_all(&to_path)?;

            // Print verbose message for directory
            if verbose {
                let message = translate!("mv-verbose-renamed", "from" => from_path.quote(), "to" => to_path.quote());
                match display_manager {
                    Some(pb) => pb.suspend(|| {
                        println!("{message}");
                    }),
                    None => println!("{message}"),
                }
            }

            copy_dir_contents_recursive(
                &from_path,
                &to_path,
                #[cfg(unix)]
                hardlink_tracker,
                #[cfg(unix)]
                hardlink_scanner,
                #[cfg(not(unix))]
                _hardlink_tracker,
                #[cfg(not(unix))]
                _hardlink_scanner,
                verbose,
                progress_bar,
                display_manager,
            )?;
        } else {
            // Copy file with or without hardlink support based on platform
            #[cfg(unix)]
            {
                copy_file_with_hardlinks_helper(
                    &from_path,
                    &to_path,
                    hardlink_tracker,
                    hardlink_scanner,
                )?;
            }
            #[cfg(not(unix))]
            {
                fs::copy(&from_path, &to_path)?;
            }

            // Print verbose message for file
            if verbose {
                let message = translate!("mv-verbose-renamed", "from" => from_path.quote(), "to" => to_path.quote());
                match display_manager {
                    Some(pb) => pb.suspend(|| {
                        println!("{message}");
                    }),
                    None => println!("{message}"),
                }
            }
        }

        if let Some(pb) = progress_bar {
            if let Ok(metadata) = from_path.metadata() {
                pb.inc(metadata.len());
            }
        }
    }

    Ok(())
}

#[cfg(unix)]
fn copy_file_with_hardlinks_helper(
    from: &Path,
    to: &Path,
    hardlink_tracker: &mut HardlinkTracker,
    hardlink_scanner: &HardlinkGroupScanner,
) -> io::Result<()> {
    // Check if this file should be a hardlink to an already-copied file
    use crate::hardlink::HardlinkOptions;
    let hardlink_options = HardlinkOptions::default();
    // Create a hardlink instead of copying
    if let Some(existing_target) =
        hardlink_tracker.check_hardlink(from, to, hardlink_scanner, &hardlink_options)?
    {
        fs::hard_link(&existing_target, to)?;
        return Ok(());
    }

    // Regular file copy
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
    {
        fs::copy(from, to).and_then(|_| fsxattr::copy_xattrs(&from, &to))?;
    }
    #[cfg(any(target_os = "macos", target_os = "redox"))]
    {
        fs::copy(from, to)?;
    }

    Ok(())
}

fn rename_file_fallback(
    from: &Path,
    to: &Path,
    #[cfg(unix)] hardlink_tracker: Option<&mut HardlinkTracker>,
    #[cfg(unix)] hardlink_scanner: Option<&HardlinkGroupScanner>,
) -> io::Result<()> {
    // Remove existing target file if it exists
    if to.is_symlink() {
        fs::remove_file(to).map_err(|err| {
            let inter_device_msg = translate!("mv-error-inter-device-move-failed", "from" => from.quote(), "to" => to.quote(), "err" => err);
            io::Error::new(err.kind(), inter_device_msg)
        })?;
    } else if to.exists() {
        // For non-symlinks, just remove the file without special error handling
        fs::remove_file(to)?;
    }

    // Check if this file is part of a hardlink group and if so, create a hardlink instead of copying
    #[cfg(unix)]
    {
        if let (Some(tracker), Some(scanner)) = (hardlink_tracker, hardlink_scanner) {
            use crate::hardlink::HardlinkOptions;
            let hardlink_options = HardlinkOptions::default();
            if let Some(existing_target) =
                tracker.check_hardlink(from, to, scanner, &hardlink_options)?
            {
                // Create a hardlink to the first moved file instead of copying
                fs::hard_link(&existing_target, to)?;
                fs::remove_file(from)?;
                return Ok(());
            }
        }
    }

    // Regular file copy
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "redox"))))]
    fs::copy(from, to)
        .and_then(|_| fsxattr::copy_xattrs(&from, &to))
        .and_then(|_| fs::remove_file(from))
        .map_err(|err| io::Error::new(err.kind(), translate!("mv-error-permission-denied")))?;
    #[cfg(any(target_os = "macos", target_os = "redox", not(unix)))]
    fs::copy(from, to)
        .and_then(|_| fs::remove_file(from))
        .map_err(|err| io::Error::new(err.kind(), translate!("mv-error-permission-denied")))?;
    Ok(())
}

fn is_empty_dir(path: &Path) -> bool {
    fs::read_dir(path).is_ok_and(|mut contents| contents.next().is_none())
}

/// Check if file is writable, returning the mode for potential reuse.
#[cfg(unix)]
fn is_writable(path: &Path) -> (bool, Option<u32>) {
    if let Ok(metadata) = path.metadata() {
        let mode = metadata.permissions().mode();
        // Check if user write bit is set
        ((mode & 0o200) != 0, Some(mode))
    } else {
        (false, None) // If we can't get metadata, prompt user to be safe
    }
}

/// Check if file is writable.
#[cfg(not(unix))]
fn is_writable(path: &Path) -> (bool, Option<u32>) {
    if let Ok(metadata) = path.metadata() {
        (!metadata.permissions().readonly(), None)
    } else {
        (false, None) // If we can't get metadata, prompt user to be safe
    }
}

#[cfg(unix)]
fn get_interactive_prompt(to: &Path, cached_mode: Option<u32>) -> String {
    use libc::mode_t;
    // Use cached mode if available, otherwise fetch it
    let mode = cached_mode.or_else(|| to.metadata().ok().map(|m| m.permissions().mode()));
    if let Some(mode) = mode {
        let file_mode = mode & 0o777;
        // Check if file is not writable by user
        if (mode & 0o200) == 0 {
            let perms = display_permissions_unix(mode as mode_t, false);
            let mode_info = format!("{file_mode:04o} ({perms})");
            return translate!("mv-prompt-overwrite-mode", "target" => to.quote(), "mode_info" => mode_info);
        }
    }
    translate!("mv-prompt-overwrite", "target" => to.quote())
}

#[cfg(not(unix))]
fn get_interactive_prompt(to: &Path, _cached_mode: Option<u32>) -> String {
    translate!("mv-prompt-overwrite", "target" => to.quote())
}

/// Prompts the user for confirmation and returns an error if declined.
fn prompt_overwrite(to: &Path, cached_mode: Option<u32>) -> io::Result<()> {
    if !prompt_yes!("{}", get_interactive_prompt(to, cached_mode)) {
        return Err(io::Error::other(""));
    }
    Ok(())
}

/// Checks if a file can be deleted by attempting to open it with delete permissions.
#[cfg(windows)]
fn can_delete_file(path: &Path) -> bool {
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
        return false;
    }

    unsafe { CloseHandle(handle) };

    true
}

#[cfg(not(windows))]
fn can_delete_file(_: &Path) -> bool {
    // On non-Windows platforms, always return false to indicate that we don't need
    // to try the copy+delete fallback. This is because on Unix-like systems,
    // rename() failing with errors other than EXDEV means the operation cannot
    // succeed even with a copy+delete approach (e.g. permission errors).
    false
}
