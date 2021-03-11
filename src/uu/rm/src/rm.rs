//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) bitor ulong

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use remove_dir_all::remove_dir_all;
use std::collections::VecDeque;
use std::fs;
use std::io::{stderr, stdin, BufRead, Write};
use std::ops::BitOr;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

#[derive(Eq, PartialEq, Clone, Copy)]
enum InteractiveMode {
    None,
    Once,
    Always,
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
static VERSION: &str = env!("CARGO_PKG_VERSION");
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

static ARG_FILES: &str = "files";

fn get_usage() -> String {
    format!("{0} [OPTION]... FILE...", executable!())
}

fn get_long_usage() -> String {
    String::from(
        "By default, rm does not remove directories.  Use the --recursive (-r)
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

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let long_usage = get_long_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(&long_usage[..])
    // TODO: make getopts support -R in addition to -r

        .arg(
            Arg::with_name(OPT_FORCE)
            .short("f")
            .long(OPT_FORCE)
            .multiple(true)
            .help("ignore nonexistent files and arguments, never prompt")
        )
        .arg(
            Arg::with_name(OPT_PROMPT)
            .short("i")
            .long("prompt before every removal")
        )
        .arg(
            Arg::with_name(OPT_PROMPT_MORE)
            .short("I")
            .help("prompt once before removing more than three files, or when removing recursively. Less intrusive than -i, while still giving some protection against most mistakes")
        )
        .arg(
            Arg::with_name(OPT_INTERACTIVE)
            .long(OPT_INTERACTIVE)
            .help("prompt according to WHEN: never, once (-I), or always (-i). Without WHEN, prompts always")
            .value_name("WHEN")
            .takes_value(true)
        )
        .arg(
            Arg::with_name(OPT_ONE_FILE_SYSTEM)
            .long(OPT_ONE_FILE_SYSTEM)
            .help("when removing a hierarchy recursively, skip any directory that is on a file system different from that of the corresponding command line argument (NOT IMPLEMENTED)")
        )
        .arg(
            Arg::with_name(OPT_NO_PRESERVE_ROOT)
            .long(OPT_NO_PRESERVE_ROOT)
            .help("do not treat '/' specially")
        )
        .arg(
            Arg::with_name(OPT_PRESERVE_ROOT)
            .long(OPT_PRESERVE_ROOT)
            .help("do not remove '/' (default)")
        )
        .arg(
            Arg::with_name(OPT_RECURSIVE).short("r")
            .long(OPT_RECURSIVE)
            .help("remove directories and their contents recursively")
        )
        .arg(
            Arg::with_name(OPT_DIR)
            .short("d")
            .long(OPT_DIR)
            .help("remove empty directories")
        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
            .short("v")
            .long(OPT_VERBOSE)
            .help("explain what is being done")
        )
        .arg(
            Arg::with_name(ARG_FILES)
            .multiple(true)
            .takes_value(true)
            .min_values(1)
        )
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let force = matches.is_present(OPT_FORCE);

    if files.is_empty() && !force {
        // Still check by hand and not use clap
        // Because "rm -f" is a thing
        show_error!("missing an argument");
        show_error!("for help, try '{0} --help'", executable!());
        return 1;
    } else {
        let options = Options {
            force,
            interactive: {
                if matches.is_present(OPT_PROMPT) {
                    InteractiveMode::Always
                } else if matches.is_present(OPT_PROMPT_MORE) {
                    InteractiveMode::Once
                } else if matches.is_present(OPT_INTERACTIVE) {
                    match &matches.value_of(OPT_INTERACTIVE).unwrap()[..] {
                        "none" => InteractiveMode::None,
                        "once" => InteractiveMode::Once,
                        "always" => InteractiveMode::Always,
                        val => crash!(1, "Invalid argument to interactive ({})", val),
                    }
                } else {
                    InteractiveMode::None
                }
            },
            one_fs: matches.is_present(OPT_ONE_FILE_SYSTEM),
            preserve_root: !matches.is_present(OPT_NO_PRESERVE_ROOT),
            recursive: matches.is_present(OPT_RECURSIVE),
            dir: matches.is_present(OPT_DIR),
            verbose: matches.is_present(OPT_VERBOSE),
        };
        if options.interactive == InteractiveMode::Once && (options.recursive || files.len() > 3) {
            let msg = if options.recursive {
                "Remove all arguments recursively? "
            } else {
                "Remove all arguments? "
            };
            if !prompt(msg) {
                return 0;
            }
        }

        if remove(files, options) {
            return 1;
        }
    }

    0
}

// TODO: implement one-file-system (this may get partially implemented in walkdir)
fn remove(files: Vec<String>, options: Options) -> bool {
    let mut had_err = false;

    for filename in &files {
        let file = Path::new(filename);
        had_err = match file.symlink_metadata() {
            Ok(metadata) => {
                if metadata.is_dir() {
                    handle_dir(file, &options)
                } else if is_symlink_dir(&metadata) {
                    remove_dir(file, &options)
                } else {
                    remove_file(file, &options)
                }
            }
            Err(_e) => {
                // TODO: actually print out the specific error
                // TODO: When the error is not about missing files
                // (e.g., permission), even rm -f should fail with
                // outputting the error, but there's no easy eay.
                if !options.force {
                    show_error!("no such file or directory '{}'", filename);
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
        if options.interactive != InteractiveMode::Always {
            // we need the extra crate because apparently fs::remove_dir_all() does not function
            // correctly on Windows
            if let Err(e) = remove_dir_all(path) {
                had_err = true;
                show_error!("could not remove '{}': {}", path.display(), e);
            }
        } else {
            let mut dirs: VecDeque<DirEntry> = VecDeque::new();

            for entry in WalkDir::new(path) {
                match entry {
                    Ok(entry) => {
                        let file_type = entry.file_type();
                        if file_type.is_dir() {
                            dirs.push_back(entry);
                        } else {
                            had_err = remove_file(entry.path(), options).bitor(had_err);
                        }
                    }
                    Err(e) => {
                        had_err = true;
                        show_error!("recursing in '{}': {}", path.display(), e);
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
        show_error!("could not remove directory '{}'", path.display());
        had_err = true;
    } else {
        show_error!(
            "could not remove directory '{}' (did you mean to pass '-r'?)",
            path.display()
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
        match fs::remove_dir(path) {
            Ok(_) => {
                if options.verbose {
                    println!("removed '{}'", path.display());
                }
            }
            Err(e) => {
                if e.to_string().starts_with("Directory not empty")
                    || (cfg!(windows) && e.to_string().starts_with("The directory is not empty"))
                {
                    let description = format!("cannot remove '{}'", path.display());
                    let error_message = "Directory not empty";

                    show_error_custom_description!(description, "{}", error_message);
                } else {
                    show_error!("removing '{}': {}", path.display(), e);
                }

                return true;
            }
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
    if response {
        match fs::remove_file(path) {
            Ok(_) => {
                if options.verbose {
                    println!("removed '{}'", path.display());
                }
            }
            Err(e) => {
                show_error!("removing '{}': {}", path.display(), e);
                return true;
            }
        }
    }

    false
}

fn prompt_file(path: &Path, is_dir: bool) -> bool {
    if is_dir {
        prompt(&(format!("rm: remove directory '{}'? ", path.display())))
    } else {
        prompt(&(format!("rm: remove file '{}'? ", path.display())))
    }
}

fn prompt(msg: &str) -> bool {
    let _ = stderr().write_all(msg.as_bytes());
    let _ = stderr().flush();

    let mut buf = Vec::new();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    #[allow(clippy::match_like_matches_macro)]
    // `matches!(...)` macro not stabilized until rust v1.42
    match stdin.read_until(b'\n', &mut buf) {
        Ok(x) if x > 0 => match buf[0] {
            b'y' | b'Y' => true,
            _ => false,
        },
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
    use std::os::raw::c_ulong;
    pub type DWORD = c_ulong;
    pub const FILE_ATTRIBUTE_DIRECTORY: DWORD = 0x10;

    metadata.file_type().is_symlink()
        && ((metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY) != 0)
}
