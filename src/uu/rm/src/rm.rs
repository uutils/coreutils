//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (path) eacces

use clap::{builder::ValueParser, crate_version, parser::ValueSource, Arg, ArgAction, Command};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, Metadata, Permissions};
use std::io::ErrorKind;
use std::ops::BitOr;
#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::{format_usage, help_about, help_section, help_usage, prompt_yes, show_error};
use walkdir::WalkDir;

#[derive(Eq, PartialEq, Clone, Copy)]
enum InteractiveMode {
    Never,
    Once,
    Always,
    PromptProtected,
}

struct Options {
    force: bool,
    interactive: InteractiveMode,
    #[allow(dead_code)]
    one_fs: bool,
    preserve_root: bool,
    recursive: bool,
    dir: bool,
    verbose: bool,
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
            preserve_root: !matches.get_flag(OPT_NO_PRESERVE_ROOT),
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
                    system different from that of the corresponding command line argument (NOT \
                    IMPLEMENTED)",
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
                .action(ArgAction::SetTrue),
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
fn remove(files: &[&OsStr], options: &Options) -> bool {
    let mut had_err = false;

    for filename in files {
        let file = Path::new(filename);
        had_err = match file.symlink_metadata() {
            Ok(metadata) => {
                if metadata.is_dir() || is_symlink_dir(&metadata) {
                    handle_dir(file, options)
                } else {
                    remove_file(file, options)
                }
            }
            Err(_e) => {
                // TODO: actually print out the specific error
                // TODO: When the error is not about missing files
                // (e.g., permission), even rm -f should fail with
                // outputting the error, but there's no easy eay.
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

fn handle_dir(path: &Path, options: &Options) -> bool {
    // either `--recursive` or `--dir` has to be present
    if !options.recursive && !options.dir {
        show_error!(
            "cannot remove {}: Is a directory", // GNU's rm error message does not include help
            path.quote()
        );
        return true;
    }

    let is_root = path.has_root() && path.parent().is_none();

    if options.dir && (!is_root || !options.preserve_root) {
        return remove_dir(path, options);
    }

    if options.recursive && (!is_root || !options.preserve_root) {
        return remove_dir_recursively(path, options);
    }

    // this is when trying to remove root with `--preserve-root`
    show_error!("could not remove directory {}", path.quote());
    false
}

fn remove_dir_recursively(path: &Path, options: &Options) -> bool {
    match fs::read_dir(path) {
        Ok(read_dir) => {
            if options.interactive == InteractiveMode::Always && read_dir.count() != 0 {
                // terminate recursive call
                if !prompt_descend(path) {
                    return false;
                }
            }

            let mut had_err: bool = false;

            // traverse only the direct children
            for entry in WalkDir::new(path).min_depth(1).max_depth(1) {
                match entry {
                    Ok(entry) => {
                        let file_type = entry.file_type();
                        if file_type.is_dir() {
                            had_err = remove_dir_recursively(entry.path(), options).bitor(had_err);
                        } else {
                            had_err = remove_file(entry.path(), options).bitor(had_err);
                        }
                    }
                    Err(e) => {
                        if e.io_error().is_some()
                            && e.io_error().unwrap().kind() == std::io::ErrorKind::PermissionDenied
                            && options.force
                        {
                            // try to see if dir to be removed is empty
                            had_err = remove_empty_dir_no_perm(path, options).bitor(had_err);
                        }
                    }
                }
            }

            if had_err {
                return had_err;
            }

            return remove_dir(path, options);
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied && options.force {
                // try to see if dir to be removed is empty
                return remove_empty_dir_no_perm(path, options);
            } else {
                show_error!("cannot remove {}: {}", path.quote(), e);
                return true;
            }
        }
    }
}

fn remove_dir(path: &Path, options: &Options) -> bool {
    if prompt_file(path, options, true) {
        match fs::read_dir(path) {
            Ok(mut read_dir) => {
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
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied && options.dir {
                    // try to see if dir to be removed is empty
                    return remove_empty_dir_no_perm(path, options);
                } else {
                    show_error!("cannot remove {}: {}", path.quote(), e);
                    return true;
                }
            }
        }
    }

    false
}

fn remove_empty_dir_no_perm(path: &Path, options: &Options) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => {
            let original_perm = metadata.permissions();
            #[cfg(unix)]
            fs::set_permissions(path, Permissions::from_mode(0o777)).unwrap();
            if fs::read_dir(path).unwrap().next().is_none() {
                match fs::remove_dir(path) {
                    Ok(_) => {
                        if options.verbose {
                            println!("removed directory {}", normalize(path).quote());
                        }
                    }
                    Err(e) => {
                        show_error!("cannot remove {}: {}", path.quote(), e);
                        return true;
                    }
                }
            } else {
                // dir is not empty
                // match GNU's error message
                show_error!("cannot remove {}: {}", path.quote(), "Permission denied");
                fs::set_permissions(path, original_perm).unwrap();
                return true;
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                show_error!("cannot remove {}: {}", path.quote(), "Permission denied");
            } else {
                show_error!("cannot remove {}: {}", path.quote(), e);
            }
            return true;
        }
    }

    false
}

fn remove_file(path: &Path, options: &Options) -> bool {
    if prompt_file(path, options, false) {
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

#[allow(clippy::cognitive_complexity)]
fn prompt_file(path: &Path, options: &Options, is_dir: bool) -> bool {
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
    if is_dir {
        // We can't use metadata.permissions.readonly for directories because it only works on files
        // So we have to handle whether a directory is writable on not manually
        if let Ok(metadata) = fs::metadata(path) {
            handle_writable_directory(path, options, &metadata)
        } else {
            true
        }
    } else {
        // File::open(path) doesn't open the file in write mode so we need to use file options to open it in also write mode to check if it can written too
        match File::options().read(true).write(true).open(path) {
            Ok(file) => {
                if let Ok(metadata) = file.metadata() {
                    if metadata.permissions().readonly() {
                        if metadata.len() == 0 {
                            prompt_yes!(
                                "remove write-protected regular empty file {}?",
                                path.quote()
                            )
                        } else {
                            prompt_yes!("remove write-protected regular file {}?", path.quote())
                        }
                    } else if options.interactive == InteractiveMode::Always {
                        if metadata.len() == 0 {
                            prompt_yes!("remove regular empty file {}?", path.quote())
                        } else {
                            prompt_yes!("remove file {}?", path.quote())
                        }
                    } else {
                        true
                    }
                } else {
                    true
                }
            }
            Err(err) => {
                if err.kind() == ErrorKind::PermissionDenied {
                    if let Ok(metadata) = fs::metadata(path) {
                        if metadata.len() == 0 {
                            prompt_yes!(
                                "remove write-protected regular empty file {}?",
                                path.quote()
                            )
                        } else {
                            prompt_yes!("remove write-protected regular file {}?", path.quote())
                        }
                    } else {
                        prompt_yes!("remove write-protected regular file {}?", path.quote())
                    }
                } else {
                    true
                }
            }
        }
    }
}

// For directories finding if they are writable or not is a hassle. In Unix we can use the built-in rust crate to to check mode bits. But other os don't have something similar afaik
// Most cases are covered by keep eye out for edge cases
#[cfg(unix)]
fn handle_writable_directory(path: &Path, options: &Options, metadata: &Metadata) -> bool {
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
