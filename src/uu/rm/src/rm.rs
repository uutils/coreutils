// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (path) eacces inacc rm-r4 unlinkat fstatat rootlink

use clap::builder::{PossibleValue, ValueParser};
use clap::{Arg, ArgAction, Command, parser::ValueSource};
use indicatif::{ProgressBar, ProgressStyle};
use std::ffi::{OsStr, OsString};
use std::fs::{self, Metadata};
use std::io::{self, IsTerminal, stdin};
use std::ops::BitOr;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::MAIN_SEPARATOR;
use std::path::Path;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult};
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
use uucore::translate;
use uucore::{format_usage, os_str_as_bytes, prompt_yes, show_error};

mod platform;
#[cfg(all(unix, not(target_os = "redox")))]
use platform::{safe_remove_dir_recursive, safe_remove_empty_dir, safe_remove_file};

#[derive(Debug, Error)]
enum RmError {
    #[error("{}", translate!("rm-error-missing-operand", "util_name" => uucore::execution_phrase()))]
    MissingOperand,
    #[error("{}", translate!("rm-error-cannot-remove-no-such-file", "file" => _0.quote()))]
    CannotRemoveNoSuchFile(OsString),
    #[error("{}", translate!("rm-error-cannot-remove-permission-denied", "file" => _0.quote()))]
    CannotRemovePermissionDenied(OsString),
    #[error("{}", translate!("rm-error-cannot-remove-is-directory", "file" => _0.quote()))]
    CannotRemoveIsDirectory(OsString),
    #[error("{}", translate!("rm-error-dangerous-recursive-operation"))]
    DangerousRecursiveOperation,
    #[error("{}", translate!("rm-error-use-no-preserve-root"))]
    UseNoPreserveRoot,
    #[error("{}", translate!("rm-error-refusing-to-remove-directory", "path" => _0.quote()))]
    RefusingToRemoveDirectory(OsString),
    #[error("{}", translate!("rm-error-may-not-abbreviate-no-preserve-root"))]
    MayNotAbbreviateNoPreserveRoot,
}

impl UError for RmError {}

/// Helper function to print verbose message for removed file
fn verbose_removed_file(path: &Path, options: &Options) {
    if options.verbose {
        println!(
            "{}",
            translate!("rm-verbose-removed", "file" => uucore::fs::normalize_path(path).quote())
        );
    }
}

/// Helper function to print verbose message for removed directory
fn verbose_removed_directory(path: &Path, options: &Options) {
    if options.verbose {
        println!(
            "{}",
            translate!("rm-verbose-removed-directory", "file" => uucore::fs::normalize_path(path).quote())
        );
    }
}

/// Helper function to show error with context and return error status
fn show_removal_error(error: io::Error, path: &Path) -> bool {
    if error.kind() == io::ErrorKind::PermissionDenied {
        show_error!("cannot remove {}: Permission denied", path.quote());
    } else {
        let e =
            error.map_err_context(|| translate!("rm-error-cannot-remove", "file" => path.quote()));
        show_error!("{e}");
    }
    true
}

/// Helper function for permission denied errors
fn show_permission_denied_error(path: &Path) -> bool {
    show_error!("cannot remove {}: Permission denied", path.quote());
    true
}

/// Helper function to remove a directory and handle results
fn remove_dir_with_feedback(path: &Path, options: &Options) -> bool {
    match fs::remove_dir(path) {
        Ok(_) => {
            verbose_removed_directory(path, options);
            false
        }
        Err(e) => show_removal_error(e, path),
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
/// Enum, determining when the `rm` will prompt the user about the file deletion
pub enum InteractiveMode {
    /// Never prompt
    Never,
    /// Prompt once before removing more than three files, or when removing
    /// recursively.
    Once,
    /// Prompt before every removal
    Always,
    /// Prompt only on write-protected files
    PromptProtected,
}

// We implement `From` instead of `TryFrom` because clap guarantees that we only receive valid values.
//
// The `PromptProtected` variant is not supposed to be created from a string.
impl From<&str> for InteractiveMode {
    fn from(s: &str) -> Self {
        match s {
            "never" => Self::Never,
            "once" => Self::Once,
            "always" => Self::Always,
            _ => unreachable!("should be prevented by clap"),
        }
    }
}

/// Options for the `rm` command
///
/// All options are public so that the options can be programmatically
/// constructed by other crates, such as Nushell. That means that this struct
/// is part of our public API. It should therefore not be changed without good
/// reason.
///
/// The fields are documented with the arguments that determine their value.
pub struct Options {
    /// `-f`, `--force`
    pub force: bool,
    /// Iterative mode, determines when the command will prompt.
    ///
    /// Set by the following arguments:
    /// - `-i`: [`InteractiveMode::Always`]
    /// - `-I`: [`InteractiveMode::Once`]
    /// - `--interactive`: sets one of the above or [`InteractiveMode::Never`]
    /// - `-f`: implicitly sets [`InteractiveMode::Never`]
    ///
    /// If no other option sets this mode, [`InteractiveMode::PromptProtected`]
    /// is used
    pub interactive: InteractiveMode,
    #[allow(dead_code)]
    /// `--one-file-system`
    pub one_fs: bool,
    /// `--preserve-root`/`--no-preserve-root`
    pub preserve_root: bool,
    /// `-r`, `--recursive`
    pub recursive: bool,
    /// `-d`, `--dir`
    pub dir: bool,
    /// `-v`, `--verbose`
    pub verbose: bool,
    /// `-g`, `--progress`
    pub progress: bool,
    #[doc(hidden)]
    /// `---presume-input-tty`
    /// Always use `None`; GNU flag for testing use only
    pub __presume_input_tty: Option<bool>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            force: false,
            interactive: InteractiveMode::PromptProtected,
            one_fs: false,
            preserve_root: true,
            recursive: false,
            dir: false,
            verbose: false,
            progress: false,
            __presume_input_tty: None,
        }
    }
}

static OPT_DIR: &str = "dir";
static OPT_INTERACTIVE: &str = "interactive";
static OPT_FORCE: &str = "force";
static OPT_NO_PRESERVE_ROOT: &str = "no-preserve-root";
static OPT_ONE_FILE_SYSTEM: &str = "one-file-system";
static OPT_PRESERVE_ROOT: &str = "preserve-root";
static OPT_PROMPT_ALWAYS: &str = "prompt-always";
static OPT_PROMPT_ONCE: &str = "prompt-once";
static OPT_RECURSIVE: &str = "recursive";
static OPT_VERBOSE: &str = "verbose";
static OPT_PROGRESS: &str = "progress";
static PRESUME_INPUT_TTY: &str = "-presume-input-tty";

static ARG_FILES: &str = "files";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<OsString> = args.collect();
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args.iter())?;

    let files: Vec<_> = matches
        .get_many::<OsString>(ARG_FILES)
        .map(|v| v.map(OsString::as_os_str).collect())
        .unwrap_or_default();

    let force_flag = matches.get_flag(OPT_FORCE);

    if files.is_empty() && !force_flag {
        // Still check by hand and not use clap
        // Because "rm -f" is a thing
        return Err(RmError::MissingOperand.into());
    }

    // If -f(--force) is before any -i (or variants) we want prompts else no prompts
    let force_prompt_never = force_flag && {
        let force_index = matches.index_of(OPT_FORCE).unwrap_or(0);
        ![OPT_PROMPT_ALWAYS, OPT_PROMPT_ONCE, OPT_INTERACTIVE]
            .iter()
            .any(|flag| {
                matches.value_source(flag) == Some(ValueSource::CommandLine)
                    && matches.index_of(flag).unwrap_or(0) > force_index
            })
    };

    let preserve_root = !matches.get_flag(OPT_NO_PRESERVE_ROOT);
    let recursive = matches.get_flag(OPT_RECURSIVE);

    let options = Options {
        force: force_flag,
        interactive: {
            if force_prompt_never {
                InteractiveMode::Never
            } else if matches.get_flag(OPT_PROMPT_ALWAYS) {
                InteractiveMode::Always
            } else if matches.get_flag(OPT_PROMPT_ONCE) {
                InteractiveMode::Once
            } else if matches.contains_id(OPT_INTERACTIVE) {
                InteractiveMode::from(matches.get_one::<String>(OPT_INTERACTIVE).unwrap().as_str())
            } else {
                InteractiveMode::PromptProtected
            }
        },
        one_fs: matches.get_flag(OPT_ONE_FILE_SYSTEM),
        preserve_root,
        recursive,
        dir: matches.get_flag(OPT_DIR),
        verbose: matches.get_flag(OPT_VERBOSE),
        progress: matches.get_flag(OPT_PROGRESS),
        __presume_input_tty: if matches.get_flag(PRESUME_INPUT_TTY) {
            Some(true)
        } else {
            None
        },
    };

    // manually parse all args to verify --no-preserve-root did not get abbreviated (clap does
    // allow this)
    if !options.preserve_root && !args.iter().any(|arg| arg == "--no-preserve-root") {
        return Err(RmError::MayNotAbbreviateNoPreserveRoot.into());
    }

    if options.interactive == InteractiveMode::Once && (options.recursive || files.len() > 3) {
        let msg: String = format!(
            "remove {} {}{}",
            files.len(),
            if files.len() > 1 {
                "arguments"
            } else {
                "argument"
            },
            if options.recursive {
                " recursively?"
            } else {
                "?"
            }
        );
        if !prompt_yes!("{msg}") {
            return Ok(());
        }
    }

    if remove(&files, &options) {
        return Err(1.into());
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("rm-about"))
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("rm-usage")))
        .after_help(translate!("rm-after-help"))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(OPT_FORCE)
                .short('f')
                .long(OPT_FORCE)
                .help(translate!("rm-help-force"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PROMPT_ALWAYS)
                .short('i')
                .help(translate!("rm-help-prompt-always"))
                .overrides_with_all([OPT_PROMPT_ONCE, OPT_INTERACTIVE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PROMPT_ONCE)
                .short('I')
                .help(translate!("rm-help-prompt-once"))
                .overrides_with_all([OPT_PROMPT_ALWAYS, OPT_INTERACTIVE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_INTERACTIVE)
                .long(OPT_INTERACTIVE)
                .help(translate!("rm-help-interactive"))
                .value_name("WHEN")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("always").alias("yes"),
                    PossibleValue::new("once"),
                    PossibleValue::new("never").alias("no").alias("none"),
                ]))
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value("always")
                .overrides_with_all([OPT_PROMPT_ALWAYS, OPT_PROMPT_ONCE]),
        )
        .arg(
            Arg::new(OPT_ONE_FILE_SYSTEM)
                .long(OPT_ONE_FILE_SYSTEM)
                .help(translate!("rm-help-one-file-system"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_NO_PRESERVE_ROOT)
                .long(OPT_NO_PRESERVE_ROOT)
                .help(translate!("rm-help-no-preserve-root"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PRESERVE_ROOT)
                .long(OPT_PRESERVE_ROOT)
                .help(translate!("rm-help-preserve-root"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_RECURSIVE)
                .short('r')
                .visible_short_alias('R')
                .long(OPT_RECURSIVE)
                .help(translate!("rm-help-recursive"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_DIR)
                .short('d')
                .long(OPT_DIR)
                .help(translate!("rm-help-dir"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_VERBOSE)
                .short('v')
                .long(OPT_VERBOSE)
                .help(translate!("rm-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PROGRESS)
                .short('g')
                .long(OPT_PROGRESS)
                .help(translate!("rm-help-progress"))
                .action(ArgAction::SetTrue),
        )
        // From the GNU source code:
        // This is solely for testing.
        // Do not document.
        // It is relatively difficult to ensure that there is a tty on stdin.
        // Since rm acts differently depending on that, without this option,
        // it'd be harder to test the parts of rm that depend on that setting.
        // In contrast with Arg::long, Arg::alias does not strip leading
        // hyphens. Therefore it supports 3 leading hyphens.
        .arg(
            Arg::new(PRESUME_INPUT_TTY)
                .long("presume-input-tty")
                .alias(PRESUME_INPUT_TTY)
                .hide(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .num_args(1..)
                .value_hint(clap::ValueHint::AnyPath),
        )
}

/// Creates a progress bar for rm operations if conditions are met.
/// Returns Some(ProgressBar) if `total_files` > 0, None otherwise.
fn create_progress_bar(files: &[&OsStr], recursive: bool) -> Option<ProgressBar> {
    let total_files = count_files(files, recursive);
    if total_files == 0 {
        return None;
    }

    Some(
        ProgressBar::new(total_files)
            .with_style(
                ProgressStyle::with_template(
                    "{msg}: [{elapsed_precise}] {wide_bar} {pos:>7}/{len:7} files",
                )
                .unwrap(),
            )
            .with_message(translate!("rm-progress-removing")),
    )
}

/// Count the total number of files and directories to be deleted.
/// This function recursively counts all files and directories that will be processed.
/// Files are not deduplicated when appearing in multiple sources. If `recursive` is set to `false`, the
/// directories in `paths` will be ignored.
fn count_files(paths: &[&OsStr], recursive: bool) -> u64 {
    let mut total = 0;
    for p in paths {
        let path = Path::new(p);
        if let Ok(md) = fs::symlink_metadata(path) {
            if md.is_dir() && !is_symlink_dir(&md) {
                if recursive {
                    total += count_files_in_directory(path);
                }
            } else {
                total += 1;
            }
        }
        // If we can't access the file, skip it for counting
        // This matches the behavior where -f suppresses errors for missing files
    }
    total
}

/// A helper for `count_files` specialized for directories.
fn count_files_in_directory(p: &Path) -> u64 {
    let entries_count = fs::read_dir(p).map_or(0, |entries| {
        entries
            .flatten()
            .map(|entry| match entry.file_type() {
                Ok(ft) if ft.is_dir() => count_files_in_directory(&entry.path()),
                Ok(_) => 1,
                Err(_) => 0,
            })
            .sum()
    });

    1 + entries_count
}

// TODO: implement one-file-system (this may get partially implemented in walkdir)
/// Remove (or unlink) the given files
///
/// Returns true if it has encountered an error.
///
/// Behavior is determined by the `options` parameter, see [`Options`] for
/// details.
pub fn remove(files: &[&OsStr], options: &Options) -> bool {
    let mut had_err = false;

    // Check if any files actually exist before creating progress bar
    let mut progress_bar: Option<ProgressBar> = None;
    let mut any_files_processed = false;

    for filename in files {
        let file = Path::new(filename);

        // Check if the path (potentially with trailing slash) resolves to root
        // This needs to happen before symlink_metadata to catch cases like "rootlink/"
        // where rootlink is a symlink to root.
        if uucore::fs::path_ends_with_terminator(file)
            && options.recursive
            && options.preserve_root
            && is_root_path(file)
        {
            show_preserve_root_error(file);
            had_err = true;
            continue;
        }

        had_err = match file.symlink_metadata() {
            Ok(metadata) => {
                // Create progress bar on first successful file metadata read
                if options.progress && progress_bar.is_none() {
                    progress_bar = create_progress_bar(files, options.recursive);
                }

                any_files_processed = true;
                if metadata.is_dir() {
                    handle_dir(file, options, progress_bar.as_ref())
                } else if is_symlink_dir(&metadata) {
                    remove_dir(file, options, progress_bar.as_ref())
                } else {
                    remove_file(file, options, progress_bar.as_ref())
                }
            }

            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if options.force {
                    false
                } else {
                    show_error!(
                        "{}",
                        RmError::CannotRemoveNoSuchFile(filename.to_os_string())
                    );
                    true
                }
            }

            Err(_e) => {
                // TODO: report the specific error (e.g. "Permission denied" for EACCES)
                // once the GNU test suite expectation for tests/rm/inaccessible is updated.
                if options.force {
                    false
                } else {
                    show_error!(
                        "{}",
                        RmError::CannotRemoveNoSuchFile(filename.to_os_string())
                    );
                    true
                }
            }
        }
        .bitor(had_err);
    }

    // Only finish progress bar if it was created and files were processed
    if let Some(pb) = progress_bar {
        if any_files_processed {
            pb.finish();
        }
    }

    had_err
}

/// Whether the given directory is empty.
///
/// `path` must be a directory. If there is an error reading the
/// contents of the directory, this returns `false`.
fn is_dir_empty(path: &Path) -> bool {
    fs::read_dir(path).is_ok_and(|mut iter| iter.next().is_none())
}

#[cfg(unix)]
fn is_readable_metadata(metadata: &Metadata) -> bool {
    let mode = metadata.permissions().mode();
    (mode & 0o400) > 0
}

/// Whether the given file or directory is readable.
#[cfg(any(not(unix), target_os = "redox"))]
fn is_readable(_path: &Path) -> bool {
    true
}

#[cfg(unix)]
fn is_writable_metadata(metadata: &Metadata) -> bool {
    let mode = metadata.permissions().mode();
    (mode & 0o200) > 0
}

#[cfg(not(unix))]
fn is_writable_metadata(_metadata: &Metadata) -> bool {
    true
}

/// Recursively remove the directory tree rooted at the given path.
///
/// If `path` is a file or a symbolic link, just remove it. If it is a
/// directory, remove all of its entries recursively and then remove the
/// directory itself. In case of an error, print the error message to
/// `stderr` and return `true`. If there were no errors, return `false`.
fn remove_dir_recursive(
    path: &Path,
    options: &Options,
    progress_bar: Option<&ProgressBar>,
) -> bool {
    // Base case 1: this is a file or a symbolic link.
    //
    // The symbolic link case is important because it could be a link to
    // a directory and we don't want to recurse. In particular, this
    // avoids an infinite recursion in the case of a link to the current
    // directory, like `ln -s . link`.
    if !path.is_dir() || path.is_symlink() {
        return remove_file(path, options, progress_bar);
    }

    // Base case 2: this is a non-empty directory, but the user
    // doesn't want to descend into it.
    if options.interactive == InteractiveMode::Always
        && !is_dir_empty(path)
        && !prompt_descend(path)
    {
        return false;
    }

    // Use secure traversal on Unix (except Redox) for all recursive directory removals
    #[cfg(all(unix, not(target_os = "redox")))]
    {
        safe_remove_dir_recursive(path, options, progress_bar)
    }

    // Fallback for non-Unix, Redox, or use fs::remove_dir_all for very long paths
    #[cfg(any(not(unix), target_os = "redox"))]
    {
        if let Some(s) = path.to_str() {
            if s.len() > 1000 {
                match fs::remove_dir_all(path) {
                    Ok(_) => return false,
                    Err(e) => {
                        let e = e.map_err_context(
                            || translate!("rm-error-cannot-remove", "file" => path.quote()),
                        );
                        show_error!("{e}");
                        return true;
                    }
                }
            }
        }

        // Recursive case: this is a directory.
        let mut error = false;
        match fs::read_dir(path) {
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                // This is not considered an error.
            }
            Err(_) => error = true,
            Ok(iter) => {
                for entry in iter {
                    match entry {
                        Err(_) => error = true,
                        Ok(entry) => {
                            let child_error =
                                remove_dir_recursive(&entry.path(), options, progress_bar);
                            error = error || child_error;
                        }
                    }
                }
            }
        }

        // Ask the user whether to remove the current directory.
        if options.interactive == InteractiveMode::Always && !prompt_dir(path, options) {
            return false;
        }

        // Try removing the directory itself.
        match fs::remove_dir(path) {
            Err(_) if !error && !is_readable(path) => {
                // For compatibility with GNU test case
                // `tests/rm/unread2.sh`, show "Permission denied" in this
                // case instead of "Directory not empty".
                show_permission_denied_error(path);
                error = true;
            }
            Err(e) if !error => {
                let e = e.map_err_context(
                    || translate!("rm-error-cannot-remove", "file" => path.quote()),
                );
                show_error!("{e}");
                error = true;
            }
            Err(_) => {
                // If there has already been at least one error when
                // trying to remove the children, then there is no need to
                // show another error message as we return from each level
                // of the recursion.
            }
            Ok(_) => verbose_removed_directory(path, options),
        }

        error
    }
}

/// Check if a path resolves to the root directory.
/// Returns true if the path is root, false otherwise.
fn is_root_path(path: &Path) -> bool {
    // Check simple case: literal "/" path
    if path.has_root() && path.parent().is_none() {
        return true;
    }

    // Check if path resolves to "/" after following symlinks
    if let Ok(canonical) = path.canonicalize() {
        canonical.has_root() && canonical.parent().is_none()
    } else {
        false
    }
}

/// Show error message for attempting to remove root.
fn show_preserve_root_error(path: &Path) {
    let path_looks_like_root = path.has_root() && path.parent().is_none();

    if path_looks_like_root {
        // Path is literally "/"
        show_error!("{}", RmError::DangerousRecursiveOperation);
    } else {
        // Path resolves to root but isn't literally "/" (e.g., symlink to /)
        show_error!(
            "{}",
            translate!("rm-error-dangerous-recursive-operation-same-as-root",
            "path" => path.display())
        );
    }
    show_error!("{}", RmError::UseNoPreserveRoot);
}

fn handle_dir(path: &Path, options: &Options, progress_bar: Option<&ProgressBar>) -> bool {
    let mut had_err = false;

    let path = clean_trailing_slashes(path);
    if path_is_current_or_parent_directory(path) {
        show_error!(
            "{}",
            RmError::RefusingToRemoveDirectory(path.as_os_str().to_os_string())
        );
        return true;
    }

    let is_root = is_root_path(path);
    if options.recursive && (!is_root || !options.preserve_root) {
        had_err = remove_dir_recursive(path, options, progress_bar);
    } else if options.dir && (!is_root || !options.preserve_root) {
        had_err = remove_dir(path, options, progress_bar).bitor(had_err);
    } else if options.recursive {
        show_preserve_root_error(path);
        had_err = true;
    } else {
        show_error!(
            "{}",
            RmError::CannotRemoveIsDirectory(path.as_os_str().to_os_string())
        );
        had_err = true;
    }

    had_err
}

/// Remove the given directory, asking the user for permission if necessary.
///
/// Returns true if it has encountered an error.
fn remove_dir(path: &Path, options: &Options, progress_bar: Option<&ProgressBar>) -> bool {
    // Ask the user for permission.
    if !prompt_dir(path, options) {
        return false;
    }

    // Called to remove a symlink_dir (windows) without "-r"/"-R" or "-d".
    if !options.dir && !options.recursive {
        show_error!(
            "{}",
            RmError::CannotRemoveIsDirectory(path.as_os_str().to_os_string())
        );
        return true;
    }

    // Use safe traversal on Unix (except Redox) for empty directory removal
    #[cfg(all(unix, not(target_os = "redox")))]
    {
        if let Some(result) = safe_remove_empty_dir(path, options, progress_bar) {
            return result;
        }
    }

    // Update progress bar for directory removal
    if let Some(pb) = progress_bar {
        pb.inc(1);
    }

    // Fallback method for non-Linux or when safe traversal is unavailable
    remove_dir_with_feedback(path, options)
}

fn remove_file(path: &Path, options: &Options, progress_bar: Option<&ProgressBar>) -> bool {
    if prompt_file(path, options) {
        // Update progress bar before removing the file
        if let Some(pb) = progress_bar {
            pb.inc(1);
        }

        // Use safe traversal on Unix (except Redox) for individual file removal
        #[cfg(all(unix, not(target_os = "redox")))]
        {
            if let Some(result) = safe_remove_file(path, options, progress_bar) {
                return result;
            }
        }

        // Fallback method for non-Unix, Redox, or when safe traversal is unavailable
        match fs::remove_file(path) {
            Ok(_) => {
                verbose_removed_file(path, options);
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::PermissionDenied {
                    // GNU compatibility (rm/fail-eacces.sh)
                    show_error!(
                        "{}",
                        RmError::CannotRemovePermissionDenied(path.as_os_str().to_os_string())
                    );
                } else {
                    return show_removal_error(e, path);
                }
                return true;
            }
        }
    }

    false
}

fn prompt_dir(path: &Path, options: &Options) -> bool {
    // If interactive is Never we never want to send prompts
    if options.interactive == InteractiveMode::Never {
        return true;
    }

    // We can't use metadata.permissions.readonly for directories because it only works on files
    // So we have to handle whether a directory is writable manually
    if let Ok(metadata) = fs::metadata(path) {
        handle_writable_directory(path, options, &metadata)
    } else {
        true
    }
}

fn prompt_file(path: &Path, options: &Options) -> bool {
    // If interactive is Never we never want to send prompts
    if options.interactive == InteractiveMode::Never {
        return true;
    }

    let Ok(metadata) = fs::symlink_metadata(path) else {
        return true;
    };

    if metadata.is_symlink() {
        return options.interactive != InteractiveMode::Always
            || prompt_yes!("remove symbolic link {}?", path.quote());
    }

    if options.interactive == InteractiveMode::Always && is_writable_metadata(&metadata) {
        return if metadata.len() == 0 {
            prompt_yes!("remove regular empty file {}?", path.quote())
        } else {
            prompt_yes!("remove file {}?", path.quote())
        };
    }

    prompt_file_permission_readonly(path, options, &metadata)
}

fn prompt_file_permission_readonly(path: &Path, options: &Options, metadata: &Metadata) -> bool {
    let stdin_ok = options.__presume_input_tty.unwrap_or(false) || stdin().is_terminal();
    match (stdin_ok, options.interactive) {
        (false, InteractiveMode::PromptProtected) => true,
        _ if is_writable_metadata(metadata) => true,
        _ if metadata.len() == 0 => prompt_yes!(
            "remove write-protected regular empty file {}?",
            path.quote()
        ),
        _ => prompt_yes!("remove write-protected regular file {}?", path.quote()),
    }
}

/// Checks if the path is referring to current or parent directory , if it is referring to current or any parent directory in the file tree e.g  '/../..' , '../..'
fn path_is_current_or_parent_directory(path: &Path) -> bool {
    let path_str = os_str_as_bytes(path.as_os_str());
    let dir_separator = MAIN_SEPARATOR as u8;
    if let Ok(path_bytes) = path_str {
        return path_bytes == ([b'.'])
            || path_bytes == ([b'.', dir_separator])
            || path_bytes == ([b'.', b'.'])
            || path_bytes == ([b'.', b'.', dir_separator])
            || path_bytes.ends_with(&[dir_separator, b'.'])
            || path_bytes.ends_with(&[dir_separator, b'.', b'.'])
            || path_bytes.ends_with(&[dir_separator, b'.', dir_separator])
            || path_bytes.ends_with(&[dir_separator, b'.', b'.', dir_separator]);
    }
    false
}

// For directories finding if they are writable or not is a hassle. In Unix we can use the built-in rust crate to check mode bits. But other os don't have something similar afaik
// Most cases are covered by keep eye out for edge cases
#[cfg(unix)]
fn handle_writable_directory(path: &Path, options: &Options, metadata: &Metadata) -> bool {
    let stdin_ok = options.__presume_input_tty.unwrap_or(false) || stdin().is_terminal();
    match (
        stdin_ok,
        is_readable_metadata(metadata),
        is_writable_metadata(metadata),
        options.interactive,
    ) {
        (false, _, _, InteractiveMode::PromptProtected) => true,
        (false, false, false, InteractiveMode::Never) => true, // Don't prompt when interactive is never
        (_, false, false, _) => prompt_yes!(
            "attempt removal of inaccessible directory {}?",
            path.quote()
        ),
        (_, false, true, InteractiveMode::Always) => prompt_yes!(
            "attempt removal of inaccessible directory {}?",
            path.quote()
        ),
        (_, true, false, _) => prompt_yes!("remove write-protected directory {}?", path.quote()),
        (_, _, _, InteractiveMode::Always) => prompt_yes!("remove directory {}?", path.quote()),
        (_, _, _, _) => true,
    }
}

// For windows we can use windows metadata trait and file attributes to see if a directory is readonly
#[cfg(windows)]
fn handle_writable_directory(path: &Path, options: &Options, metadata: &Metadata) -> bool {
    use std::os::windows::prelude::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_READONLY;
    let not_user_writable = (metadata.file_attributes() & FILE_ATTRIBUTE_READONLY) != 0;
    let stdin_ok = options.__presume_input_tty.unwrap_or(false) || stdin().is_terminal();
    match (stdin_ok, not_user_writable, options.interactive) {
        (false, _, InteractiveMode::PromptProtected) => true,
        (_, true, _) => prompt_yes!("remove write-protected directory {}?", path.quote()),
        (_, _, InteractiveMode::Always) => prompt_yes!("remove directory {}?", path.quote()),
        (_, _, _) => true,
    }
}

// I have this here for completeness but it will always return "remove directory {}" because metadata.permissions().readonly() only works for file not directories
#[cfg(not(windows))]
#[cfg(not(unix))]
fn handle_writable_directory(path: &Path, options: &Options, _metadata: &Metadata) -> bool {
    if options.interactive == InteractiveMode::Always {
        prompt_yes!("remove directory {}?", path.quote())
    } else {
        true
    }
}

/// Removes trailing slashes, for example 'd/../////' yield 'd/../' required to fix rm-r4 GNU test
fn clean_trailing_slashes(path: &Path) -> &Path {
    let path_str = os_str_as_bytes(path.as_os_str());
    let dir_separator = MAIN_SEPARATOR as u8;

    if let Ok(path_bytes) = path_str {
        let mut idx = if path_bytes.len() > 1 {
            path_bytes.len() - 1
        } else {
            return path;
        };
        // Checks if element at the end is a '/'
        if path_bytes[idx] == dir_separator {
            for i in (1..path_bytes.len()).rev() {
                // Will break at the start of the continuous sequence of '/', eg: "abc//////" , will break at
                // "abc/", this will clean ////// to the root '/', so we have to be careful to not
                // delete the root.
                if path_bytes[i - 1] != dir_separator {
                    idx = i;
                    break;
                }
            }
            #[cfg(unix)]
            return Path::new(OsStr::from_bytes(&path_bytes[0..=idx]));

            #[cfg(not(unix))]
            // Unwrapping is fine here as os_str_as_bytes() would return an error on non unix
            // systems with non utf-8 characters and thus bypass the if let Ok branch
            return Path::new(std::str::from_utf8(&path_bytes[0..=idx]).unwrap());
        }
    }
    path
}

fn prompt_descend(path: &Path) -> bool {
    prompt_yes!("descend into directory {}?", path.quote())
}

#[cfg(not(windows))]
fn is_symlink_dir(_metadata: &Metadata) -> bool {
    false
}

#[cfg(windows)]
fn is_symlink_dir(metadata: &Metadata) -> bool {
    use std::os::windows::prelude::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;

    metadata.file_type().is_symlink()
        && ((metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY) != 0)
}

mod tests {

    #[test]
    // Testing whether path the `/////` collapses to `/`
    fn test_collapsible_slash_path() {
        use std::path::Path;

        use crate::clean_trailing_slashes;
        let path = Path::new("/////");

        assert_eq!(Path::new("/"), clean_trailing_slashes(path));
    }
}
