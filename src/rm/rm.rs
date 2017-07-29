#![crate_name = "uu_rm"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_use]
extern crate clap;
extern crate remove_dir_all;
extern crate walkdir;

#[macro_use]
extern crate uucore;

use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{stdin, stderr, BufRead, Write};
use std::ops::BitOr;
use std::path::Path;
use clap::{App, Arg};
use remove_dir_all::remove_dir_all;
use walkdir::{DirEntry, WalkDir};

#[derive(Eq, PartialEq, Clone, Copy)]
enum InteractiveMode {
    InteractiveNone,
    InteractiveOnce,
    InteractiveAlways
}

struct Options {
    force: bool,
    interactive: InteractiveMode,
    #[allow(dead_code)]
    one_fs: bool,
    preserve_root: bool,
    recursive: bool,
    dir: bool,
    verbose: bool
}

const AFTER_HELP: &'static str = "
By default, rm does not remove directories.  Use the --recursive (-r)
option to remove each listed directory, too, along with all of its contents

To remove a file whose name starts with a '-', for example '-foo',
use one of these commands:
rm -- -foo

rm ./-foo

Note that if you use rm to remove a file, it might be possible to recover
some of its contents, given sufficient expertise and/or time.  For greater
assurance that the contents are truly unrecoverable, consider using shred.
";

pub fn uumain(args: Vec<OsString>) -> i32 {
    let matches = App::new(executable!(args))
                          .version(crate_version!())
                          .author("uutils developers (https://github.com/uutils)")
                          .about("Deletes (aka \"remove\" or \"unlink\") files and directories.")
                          .after_help(AFTER_HELP)
                          .arg(Arg::with_name("force")
                               .short("f")
                               .long("force")
                               .help("Ignore nonexistent files and arguments, never prompt"))
                          .arg(Arg::with_name("int_always")
                               .short("i")
                               .help("Prompt before every removal"))
                          .arg(Arg::with_name("int_once")
                               .short("I")
                               .help("Prompt once before removing more than three files, or when removing recursively.  Less intrusive than -i, while still giving some protection against most mistakes"))
                          .arg(Arg::with_name("interactive")
                               .long("interactive")
                               .value_name("WHEN")
                               .help("Prompt according to WHEN: never, once (-I), or always (-i).  Without WHEN, prompts always")
                               .takes_value(true))
                          .arg(Arg::with_name("one-fs")
                               .long("one-file-system")
                               .help("When removing a hierarchy recursively, skip any directory that is on a file system different from that of the corresponding command line argument (NOT IMPLEMENTED)"))
                          .arg(Arg::with_name("no-preserve-root")
                               .long("no-preserve-root")
                               .help("Do not treat '/' specially"))
                          .arg(Arg::with_name("recursive")
                               .short("r")
                               .visible_alias("R")
                               .long("recursive")
                               .help("Remove directories and their contents recursively"))
                          .arg(Arg::with_name("dir")
                               .short("d")
                               .long("dir")
                               .help("Remove empty directories"))
                          .arg(Arg::with_name("verbose")
                               .short("v")
                               .long("verbose")
                               .help("Explain what is being done"))
                          .arg(Arg::with_name("FILES")
                               .help("Which files and/or directories are to be removed")
                               .required(true)
                               .index(1)
                               .multiple(true))
                          .get_matches_from(args);

    let files: Vec<&OsStr> = matches.values_of_os("FILES").unwrap().collect();
    let options = Options {
        force: matches.is_present("force"),
        interactive: {
            if matches.is_present("int_always") {
                InteractiveMode::InteractiveAlways
            } else if matches.is_present("int_once") {
                InteractiveMode::InteractiveOnce
            } else if matches.is_present("interactive") {
                match &matches.value_of("interactive").unwrap()[..] {
                    "none" => InteractiveMode::InteractiveNone,
                    "once" => InteractiveMode::InteractiveOnce,
                    "always" => InteractiveMode::InteractiveAlways,
                    val => {
                        crash!(1, "Invalid argument to interactive ({})", val)
                    }
                }
            } else {
                InteractiveMode::InteractiveNone
            }
        },
        one_fs: matches.is_present("one-fs"),
        preserve_root: !matches.is_present("no-preserve-root"),
        recursive: matches.is_present("recursive"),
        dir: matches.is_present("dir"),
        verbose: matches.is_present("verbose")
    };
    if options.interactive == InteractiveMode::InteractiveOnce
            && (options.recursive || files.len() > 3) {
        let msg =
            if options.recursive {
                "Remove all arguments recursively? "
            } else {
                "Remove all arguments? "
            };
        if !prompt(msg) {
            return 0;
        }
    }

    if remove(files, options) {
        1
    } else {
        0
    }
}

// TODO: implement one-file-system (this may get partially implemented in walkdir)
fn remove(files: Vec<&OsStr>, options: Options) -> bool {
    let mut had_err = false;

    for filename in &files {
        let file = Path::new(filename);
        had_err = match file.symlink_metadata() {
            Ok(metadata) => {
                if metadata.is_dir() {
                    handle_dir(file, &options)
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
                    show_error!("no such file or directory '{}'", file.display());
                    true
                } else {
                    false
                }
            }
        }.bitor(had_err);
    }

    had_err
}

fn handle_dir(path: &Path, options: &Options) -> bool {
    let mut had_err = false;

    let is_root = path.has_root() && path.parent().is_none();
    if options.recursive && (!is_root || !options.preserve_root) {
        if options.interactive != InteractiveMode::InteractiveAlways {
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
    } else {
        if options.recursive {
            show_error!("could not remove directory '{}'", path.display());
            had_err = true;
        } else {
            show_error!("could not remove directory '{}' (did you mean to pass '-r'?)",
                        path.display());
            had_err = true;
        }
    }

    had_err
}

fn remove_dir(path: &Path, options: &Options) -> bool {
    let response =
        if options.interactive == InteractiveMode::InteractiveAlways {
            prompt_file(path, true)
        } else {
            true
        };
    if response {
        match fs::remove_dir(path) {
            Ok(_) => if options.verbose { println!("removed '{}'", path.display()); },
            Err(e) => {
                show_error!("removing '{}': {}", path.display(), e);
                return true;
            }
        }
    }

    false
}

fn remove_file(path: &Path, options: &Options) -> bool {
    let response =
        if options.interactive == InteractiveMode::InteractiveAlways {
            prompt_file(path, false)
        } else {
            true
        };
    if response {
        match fs::remove_file(path) {
            Ok(_) => if options.verbose { println!("removed '{}'", path.display()); },
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

    match stdin.read_until('\n' as u8, &mut buf) {
        Ok(x) if x > 0 => {
            match buf[0] {
                b'y' | b'Y' => true,
                _ => false,
            }
        }
        _ => false,
    }
}
