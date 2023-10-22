// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (path) eacces inacc

use clap::{builder::ValueParser, crate_version, parser::ValueSource, Arg, ArgAction, Command};
use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, Metadata};
use std::io::ErrorKind;
use std::ops::BitOr;
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::{format_usage, help_about, help_section, help_usage, prompt_yes, show_error};
use walkdir::{DirEntry, WalkDir};

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

#[derive(PartialEq, Debug)]
pub enum PreserveRoot {
    Default,
    YesAll,
    No,
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
    /// `--one-file-system`
    pub one_fs: bool,
    /// `--preserve-root`/`--no-preserve-root`
    pub preserve_root: PreserveRoot,
    /// `-r`, `--recursive`
    pub recursive: bool,
    /// `-d`, `--dir`
    pub dir: bool,
    /// `-v`, `--verbose`
    pub verbose: bool,
}

const ABOUT: &str = help_about!("rm.md");
const USAGE: &str = help_usage!("rm.md");
const AFTER_HELP: &str = help_section!("after help", "rm.md");

static OPT_DIR: &str = "dir";
static OPT_INTERACTIVE: &str = "interactive";
static OPT_FORCE: &str = "force";
static OPT_NO_PRESERVE_ROOT: &str = "no-preserve-root";
static OPT_ONE_FILE_SYSTEM: &str = "one-file-system";
static OPT_PRESERVE_ROOT: &str = "preserve-root";
static OPT_PROMPT: &str = "prompt";
static OPT_PROMPT_MORE: &str = "prompt-more";
static OPT_RECURSIVE: &str = "recursive";
static OPT_VERBOSE: &str = "verbose";
static PRESUME_INPUT_TTY: &str = "-presume-input-tty";

static ARG_FILES: &str = "files";

#[uucore::main]
#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().after_help(AFTER_HELP).try_get_matches_from(args)?;

    let files: Vec<&OsStr> = matches
        .get_many::<OsString>(ARG_FILES)
        .map(|v| v.map(OsString::as_os_str).collect())
        .unwrap_or_default();

    let force_flag = matches.get_flag(OPT_FORCE);

    // If -f(--force) is before any -i (or variants) we want prompts else no prompts
    let force_prompt_never: bool = force_flag && {
        let force_index = matches.index_of(OPT_FORCE).unwrap_or(0);
        ![OPT_PROMPT, OPT_PROMPT_MORE, OPT_INTERACTIVE]
            .iter()
            .any(|flag| {
                matches.value_source(flag) == Some(ValueSource::CommandLine)
                    && matches.index_of(flag).unwrap_or(0) > force_index
            })
    };

    if files.is_empty() && !force_flag {
        // Still check by hand and not use clap
        // Because "rm -f" is a thing
        return Err(UUsageError::new(1, "missing operand"));
    } else {
        let options = Options {
            force: force_flag,
            interactive: {
                if force_prompt_never {
                    InteractiveMode::Never
                } else if matches.get_flag(OPT_PROMPT) {
                    InteractiveMode::Always
                } else if matches.get_flag(OPT_PROMPT_MORE) {
                    InteractiveMode::Once
                } else if matches.contains_id(OPT_INTERACTIVE) {
                    match matches.get_one::<String>(OPT_INTERACTIVE).unwrap().as_str() {
                        "never" => InteractiveMode::Never,
                        "once" => InteractiveMode::Once,
                        "always" => InteractiveMode::Always,
                        val => {
                            return Err(USimpleError::new(
                                1,
                                format!("Invalid argument to interactive ({val})"),
                            ))
                        }
                    }
                } else {
                    InteractiveMode::PromptProtected
                }
            },
            one_fs: matches.get_flag(OPT_ONE_FILE_SYSTEM),
            preserve_root: if matches.get_flag(OPT_NO_PRESERVE_ROOT) {
                PreserveRoot::No
            } else {
                match matches
                    .get_one::<String>(OPT_PRESERVE_ROOT)
                    .unwrap()
                    .as_str()
                {
                    "all" => PreserveRoot::YesAll,
                    _ => PreserveRoot::Default,
                }
            },
            recursive: matches.get_flag(OPT_RECURSIVE),
            dir: matches.get_flag(OPT_DIR),
            verbose: matches.get_flag(OPT_VERBOSE),
        };
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
            if !prompt_yes!("{}", msg) {
                return Ok(());
            }
        }

        if remove(&files, &options) {
            return Err(1.into());
        }
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(OPT_FORCE)
                .short('f')
                .long(OPT_FORCE)
                .help("ignore nonexistent files and arguments, never prompt")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PROMPT)
                .short('i')
                .help("prompt before every removal")
                .overrides_with_all([OPT_PROMPT_MORE, OPT_INTERACTIVE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PROMPT_MORE)
                .short('I')
                .help("prompt once before removing more than three files, or when removing recursively. \
                Less intrusive than -i, while still giving some protection against most mistakes")
                .overrides_with_all([OPT_PROMPT, OPT_INTERACTIVE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_INTERACTIVE)
                .long(OPT_INTERACTIVE)
                .help(
                    "prompt according to WHEN: never, once (-I), or always (-i). Without WHEN, \
                    prompts always",
                )
                .value_name("WHEN")
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value("always")
                .overrides_with_all([OPT_PROMPT, OPT_PROMPT_MORE]),
        )
        .arg(
            Arg::new(OPT_ONE_FILE_SYSTEM)
                .long(OPT_ONE_FILE_SYSTEM)
                .help(
                    "when removing a hierarchy recursively, skip any directory that is on a file \
                    system different from that of the corresponding command line argument",
                ).action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_NO_PRESERVE_ROOT)
                .long(OPT_NO_PRESERVE_ROOT)
                .help("do not treat '/' specially")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PRESERVE_ROOT)
                .long(OPT_PRESERVE_ROOT)
                .help("do not remove '/' (default)")
                .value_parser(["all"])
                .default_value("default")
                .default_missing_value("default")
                .hide_default_value(true)
        )
        .arg(
            Arg::new(OPT_RECURSIVE)
                .short('r')
                .visible_short_alias('R')
                .long(OPT_RECURSIVE)
                .help("remove directories and their contents recursively")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_DIR)
                .short('d')
                .long(OPT_DIR)
                .help("remove empty directories")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_VERBOSE)
                .short('v')
                .long(OPT_VERBOSE)
                .help("explain what is being done")
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

// TODO: implement one-file-system (this may get partially implemented in walkdir)
/// Remove (or unlink) the given files
///
/// Returns true if it has encountered an error.
///
/// Behavior is determined by the `options` parameter, see [`Options`] for
/// details.
pub fn remove(files: &[&OsStr], options: &Options) -> bool {
    let mut had_err = false;

    for filename in files {
        let file = Path::new(filename);
        had_err = match file.symlink_metadata() {
            Ok(metadata) => {
                if metadata.is_dir() {
                    handle_dir(file, options)
                } else if is_symlink_dir(&metadata) {
                    remove_dir(file, options)
                } else {
                    remove_file(file, options)
                }
            }
            Err(_e) => {
                // TODO: actually print out the specific error
                // TODO: When the error is not about missing files
                // (e.g., permission), even rm -f should fail with
                // outputting the error, but there's no easy way.
                if options.force {
                    false
                } else {
                    show_error!(
                        "cannot remove {}: No such file or directory",
                        filename.quote()
                    );
                    true
                }
            }
        }
        .bitor(had_err);
    }

    had_err
}

#[cfg(not(windows))]
fn get_device_id(p: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    p.symlink_metadata()
        .ok()
        .map(|metadata| MetadataExt::dev(&metadata))
}

#[cfg(windows)]
fn get_device_id(_p: &Path) -> Option<u64> {
    unimplemented!()
}

/// Checks if the given path is on the same device as its parent.
/// Returns false if the `one_fs` option is enabled and devices differ.
fn check_one_fs(path: &Path, options: &Options) -> bool {
    if !options.one_fs && options.preserve_root != PreserveRoot::YesAll {
        return true;
    }

    // as we can get relative path, we need to canonicalize
    // and manage potential errors
    let parent_device = fs::canonicalize(path)
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .as_deref()
        .and_then(get_device_id);
    let current_device = get_device_id(path);
    if parent_device != current_device {
        show_error!(
            "skipping {}, since it's on a different device",
            path.quote()
        );
        if options.preserve_root == PreserveRoot::YesAll {
            show_error!("and --preserve-root=all is in effect");
        }
        return false;
    }

    true
}

#[allow(clippy::cognitive_complexity)]
fn handle_dir(path: &Path, options: &Options) -> bool {
    let mut had_err = false;

    if !check_one_fs(path, options) {
        return true;
    }

    let is_root = path.has_root() && path.parent().is_none();
    if options.recursive && (!is_root || options.preserve_root == PreserveRoot::No) {
        if options.interactive != InteractiveMode::Always && !options.verbose {
            if let Err(e) = fs::remove_dir_all(path) {
                // GNU compatibility (rm/empty-inacc.sh)
                // remove_dir_all failed. maybe it is because of the permissions
                // but if the directory is empty, remove_dir might work.
                // So, let's try that before failing for real
                if let Err(_e) = fs::remove_dir(path) {
                    had_err = true;
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        // GNU compatibility (rm/fail-eacces.sh)
                        // here, GNU doesn't use some kind of remove_dir_all
                        // It will show directory+file
                        show_error!("cannot remove {}: {}", path.quote(), "Permission denied");
                    } else {
                        show_error!("cannot remove {}: {}", path.quote(), e);
                    }
                }
            }
        } else {
            let mut dirs: VecDeque<DirEntry> = VecDeque::new();
            // The Paths to not descend into. We need to this because WalkDir doesn't have a way, afaik, to not descend into a directory
            // So we have to just ignore paths as they come up if they start with a path we aren't descending into
            let mut not_descended: Vec<PathBuf> = Vec::new();

            'outer: for entry in WalkDir::new(path) {
                match entry {
                    Ok(entry) => {
                        if options.interactive == InteractiveMode::Always {
                            for not_descend in &not_descended {
                                if entry.path().starts_with(not_descend) {
                                    // We don't need to continue the rest of code in this loop if we are in a directory we don't want to descend into
                                    continue 'outer;
                                }
                            }
                        }
                        let file_type = entry.file_type();
                        if file_type.is_dir() {
                            // If we are in Interactive Mode Always and the directory isn't empty we ask if we should descend else we push this directory onto dirs vector
                            if options.interactive == InteractiveMode::Always
                                && fs::read_dir(entry.path()).unwrap().count() != 0
                            {
                                // If we don't descend we push this directory onto our not_descended vector else we push this directory onto dirs vector
                                if prompt_descend(entry.path()) {
                                    dirs.push_back(entry);
                                } else {
                                    not_descended.push(entry.path().to_path_buf());
                                }
                            } else {
                                dirs.push_back(entry);
                            }
                        } else {
                            had_err = remove_file(entry.path(), options).bitor(had_err);
                        }
                    }
                    Err(e) => {
                        had_err = true;
                        show_error!("recursing in {}: {}", path.quote(), e);
                    }
                }
            }

            for dir in dirs.iter().rev() {
                had_err = remove_dir(dir.path(), options).bitor(had_err);
            }
        }
    } else if options.dir && (!is_root || options.preserve_root == PreserveRoot::No) {
        had_err = remove_dir(path, options).bitor(had_err);
    } else if options.recursive {
        show_error!("could not remove directory {}", path.quote());
        had_err = true;
    } else {
        show_error!(
            "cannot remove {}: Is a directory", // GNU's rm error message does not include help
            path.quote()
        );
        had_err = true;
    }

    had_err
}

fn remove_dir(path: &Path, options: &Options) -> bool {
    if prompt_dir(path, options) {
        if let Ok(mut read_dir) = fs::read_dir(path) {
            if options.dir || options.recursive {
                if read_dir.next().is_none() {
                    match fs::remove_dir(path) {
                        Ok(_) => {
                            if options.verbose {
                                println!("removed directory {}", normalize(path).quote());
                            }
                        }
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::PermissionDenied {
                                // GNU compatibility (rm/fail-eacces.sh)
                                show_error!(
                                    "cannot remove {}: {}",
                                    path.quote(),
                                    "Permission denied"
                                );
                            } else {
                                show_error!("cannot remove {}: {}", path.quote(), e);
                            }
                            return true;
                        }
                    }
                } else {
                    // directory can be read but is not empty
                    show_error!("cannot remove {}: Directory not empty", path.quote());
                    return true;
                }
            } else {
                // called to remove a symlink_dir (windows) without "-r"/"-R" or "-d"
                show_error!("cannot remove {}: Is a directory", path.quote());
                return true;
            }
        } else {
            // GNU's rm shows this message if directory is empty but not readable
            show_error!("cannot remove {}: Directory not empty", path.quote());
            return true;
        }
    }

    false
}

fn remove_file(path: &Path, options: &Options) -> bool {
    if prompt_file(path, options) {
        match fs::remove_file(path) {
            Ok(_) => {
                if options.verbose {
                    println!("removed {}", normalize(path).quote());
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    // GNU compatibility (rm/fail-eacces.sh)
                    show_error!("cannot remove {}: {}", path.quote(), "Permission denied");
                } else {
                    show_error!("cannot remove {}: {}", path.quote(), e);
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
    // If interactive is Always we want to check if the file is symlink to prompt the right message
    if options.interactive == InteractiveMode::Always {
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if metadata.is_symlink() {
                return prompt_yes!("remove symbolic link {}?", path.quote());
            }
        }
    }
    // File::open(path) doesn't open the file in write mode so we need to use file options to open it in also write mode to check if it can written too
    match File::options().read(true).write(true).open(path) {
        Ok(file) => {
            let Ok(metadata) = file.metadata() else {
                return true;
            };

            if options.interactive == InteractiveMode::Always && !metadata.permissions().readonly()
            {
                return if metadata.len() == 0 {
                    prompt_yes!("remove regular empty file {}?", path.quote())
                } else {
                    prompt_yes!("remove file {}?", path.quote())
                };
            }
        }
        Err(err) => {
            if err.kind() != ErrorKind::PermissionDenied {
                return true;
            }
        }
    }
    prompt_file_permission_readonly(path)
}

fn prompt_file_permission_readonly(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) if !metadata.permissions().readonly() => true,
        Ok(metadata) if metadata.len() == 0 => prompt_yes!(
            "remove write-protected regular empty file {}?",
            path.quote()
        ),
        _ => prompt_yes!("remove write-protected regular file {}?", path.quote()),
    }
}

// For directories finding if they are writable or not is a hassle. In Unix we can use the built-in rust crate to to check mode bits. But other os don't have something similar afaik
// Most cases are covered by keep eye out for edge cases
#[cfg(unix)]
fn handle_writable_directory(path: &Path, options: &Options, metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let mode = metadata.permissions().mode();
    // Check if directory has user write permissions
    // Why is S_IWUSR showing up as a u16 on macos?
    #[allow(clippy::unnecessary_cast)]
    let user_writable = (mode & (libc::S_IWUSR as u32)) != 0;
    if !user_writable {
        prompt_yes!("remove write-protected directory {}?", path.quote())
    } else if options.interactive == InteractiveMode::Always {
        prompt_yes!("remove directory {}?", path.quote())
    } else {
        true
    }
}

// For windows we can use windows metadata trait and file attributes to see if a directory is readonly
#[cfg(windows)]
fn handle_writable_directory(path: &Path, options: &Options, metadata: &Metadata) -> bool {
    use std::os::windows::prelude::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_READONLY;
    let not_user_writable = (metadata.file_attributes() & FILE_ATTRIBUTE_READONLY) != 0;
    if not_user_writable {
        prompt_yes!("remove write-protected directory {}?", path.quote())
    } else if options.interactive == InteractiveMode::Always {
        prompt_yes!("remove directory {}?", path.quote())
    } else {
        true
    }
}

// I have this here for completeness but it will always return "remove directory {}" because metadata.permissions().readonly() only works for file not directories
#[cfg(not(windows))]
#[cfg(not(unix))]
fn handle_writable_directory(path: &Path, options: &Options, metadata: &Metadata) -> bool {
    if options.interactive == InteractiveMode::Always {
        prompt_yes!("remove directory {}?", path.quote())
    } else {
        true
    }
}

fn prompt_descend(path: &Path) -> bool {
    prompt_yes!("descend into directory {}?", path.quote())
}

fn normalize(path: &Path) -> PathBuf {
    // copied from https://github.com/rust-lang/cargo/blob/2e4cfc2b7d43328b207879228a2ca7d427d188bb/src/cargo/util/paths.rs#L65-L90
    // both projects are MIT https://github.com/rust-lang/cargo/blob/master/LICENSE-MIT
    // for std impl progress see rfc https://github.com/rust-lang/rfcs/issues/2208
    // TODO: replace this once that lands
    uucore::fs::normalize_path(path)
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
