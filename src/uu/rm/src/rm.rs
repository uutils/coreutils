//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (path) eacces

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, ArgAction, Command};
use remove_dir_all::remove_dir_all;
use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use std::io::{stderr, stdin, BufRead, Write};
use std::ops::BitOr;
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::format_usage;
use walkdir::{DirEntry, WalkDir};

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

static ABOUT: &str = "Remove (unlink) the FILE(s)";
const USAGE: &str = "{} [OPTION]... FILE...";
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

fn get_long_usage() -> String {
    String::from(
        "By default, rm does not remove directories.  Use the --recursive (-r or -R)
        option to remove each listed directory, too, along with all of its contents

        To remove a file whose name starts with a '-', for example '-foo',
        use one of these commands:
        rm -- -foo

        rm ./-foo

        Note that if you use rm to remove a file, it might be possible to recover
        some of its contents, given sufficient expertise and/or time.  For greater
        assurance that the contents are truly unrecoverable, consider using shred.",
    )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app()
        .after_help(get_long_usage())
        .try_get_matches_from(args)?;

    let files: Vec<String> = matches
        .get_many::<String>(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let force = matches.get_flag(OPT_FORCE);

    if files.is_empty() && !force {
        // Still check by hand and not use clap
        // Because "rm -f" is a thing
        return Err(UUsageError::new(1, "missing operand"));
    } else {
        let options = Options {
            force,
            interactive: {
                if matches.get_flag(OPT_PROMPT) {
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
                                format!("Invalid argument to interactive ({})", val),
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
            let msg = if options.recursive {
                "Remove all arguments recursively? "
            } else {
                "Remove all arguments? "
            };
            if !prompt(msg) {
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
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(OPT_PROMPT_MORE).short('I').help(
            "prompt once before removing more than three files, or when removing recursively. \
            Less intrusive than -i, while still giving some protection against most mistakes",
        ).action(ArgAction::SetTrue))
        .arg(
            Arg::new(OPT_INTERACTIVE)
                .long(OPT_INTERACTIVE)
                .help(
                    "prompt according to WHEN: never, once (-I), or always (-i). Without WHEN, \
                    prompts always",
                )
                .value_name("WHEN"),
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
                .num_args(1..)
                .value_hint(clap::ValueHint::AnyPath),
        )
}

// TODO: implement one-file-system (this may get partially implemented in walkdir)
fn remove(files: &[String], options: &Options) -> bool {
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
                // outputting the error, but there's no easy eay.
                if !options.force {
                    show_error!(
                        "cannot remove {}: No such file or directory",
                        filename.quote()
                    );
                    true
                } else {
                    false
                }
            }
        }
        .bitor(had_err);
    }

    had_err
}

fn handle_dir(path: &Path, options: &Options) -> bool {
    let mut had_err = false;

    let is_root = path.has_root() && path.parent().is_none();
    if options.recursive && (!is_root || !options.preserve_root) {
        if options.interactive != InteractiveMode::Always && !options.verbose {
            // we need the extra crate because apparently fs::remove_dir_all() does not function
            // correctly on Windows
            if let Err(e) = remove_dir_all(path) {
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
    } else if options.dir && (!is_root || !options.preserve_root) {
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
    let response = if options.interactive == InteractiveMode::Always {
        prompt_file(path, true)
    } else {
        true
    };
    if response {
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
    let response = if options.interactive == InteractiveMode::Always {
        prompt_file(path, false)
    } else {
        true
    };
    if response && prompt_write_protected(path, false, options) {
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

fn prompt_write_protected(path: &Path, is_dir: bool, options: &Options) -> bool {
    if options.interactive == InteractiveMode::Never {
        return true;
    }
    match File::open(path) {
        Ok(_) => true,
        Err(err) => {
            if err.kind() == ErrorKind::PermissionDenied {
                if is_dir {
                    prompt(&(format!("rm: remove write-protected directory {}? ", path.quote())))
                } else {
                    if fs::metadata(path).unwrap().len() == 0 {
                        return prompt(
                            &(format!(
                                "rm: remove write-protected regular empty file {}? ",
                                path.quote()
                            )),
                        );
                    }
                    prompt(&(format!("rm: remove write-protected regular file {}? ", path.quote())))
                }
            } else {
                true
            }
        }
    }
}

fn prompt_descend(path: &Path) -> bool {
    prompt(&(format!("rm: descend into directory {}? ", path.quote())))
}

fn prompt_file(path: &Path, is_dir: bool) -> bool {
    if is_dir {
        prompt(&(format!("rm: remove directory {}? ", path.quote())))
    } else {
        prompt(&(format!("rm: remove file {}? ", path.quote())))
    }
}

fn normalize(path: &Path) -> PathBuf {
    // copied from https://github.com/rust-lang/cargo/blob/2e4cfc2b7d43328b207879228a2ca7d427d188bb/src/cargo/util/paths.rs#L65-L90
    // both projects are MIT https://github.com/rust-lang/cargo/blob/master/LICENSE-MIT
    // for std impl progress see rfc https://github.com/rust-lang/rfcs/issues/2208
    // TODO: replace this once that lands
    uucore::fs::normalize_path(path)
}

fn prompt(msg: &str) -> bool {
    let _ = stderr().write_all(msg.as_bytes());
    let _ = stderr().flush();

    let mut buf = Vec::new();
    let stdin = stdin();
    let mut stdin = stdin.lock();
    let read = stdin.read_until(b'\n', &mut buf);
    match read {
        Ok(x) if x > 0 => matches!(buf[0], b'y' | b'Y'),
        _ => false,
    }
}

#[cfg(not(windows))]
fn is_symlink_dir(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(windows)]
use std::os::windows::prelude::MetadataExt;

#[cfg(windows)]
fn is_symlink_dir(metadata: &fs::Metadata) -> bool {
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;

    metadata.file_type().is_symlink()
        && ((metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY) != 0)
}
