#![crate_name = "uu_rm"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::collections::VecDeque;
use std::fs;
use std::io::{stdin, stderr, BufRead, Write};
use std::ops::BitOr;
use std::path::{Path, PathBuf};

#[derive(Eq, PartialEq, Clone, Copy)]
enum InteractiveMode {
    InteractiveNone,
    InteractiveOnce,
    InteractiveAlways
}

static NAME: &'static str = "rm";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    // TODO: make getopts support -R in addition to -r
    let mut opts = getopts::Options::new();

    opts.optflag("f", "force", "ignore nonexistent files and arguments, never prompt");
    opts.optflag("i", "", "prompt before every removal");
    opts.optflag("I", "", "prompt once before removing more than three files, or when removing recursively.  Less intrusive than -i, while still giving some protection against most mistakes");
    opts.optflagopt("", "interactive", "prompt according to WHEN: never, once (-I), or always (-i).  Without WHEN, prompts always", "WHEN");
    opts.optflag("", "one-file-system", "when removing a hierarchy recursively, skip any directory that is on a file system different from that of the corresponding command line argument (NOT IMPLEMENTED)");
    opts.optflag("", "no-preserve-root", "do not treat '/' specially");
    opts.optflag("", "preserve-root", "do not remove '/' (default)");
    opts.optflag("r", "recursive", "remove directories and their contents recursively");
    opts.optflag("d", "dir", "remove empty directories");
    opts.optflag("v", "verbose", "explain what is being done");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };
    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION]... [FILE]...", NAME);
        println!("");
        println!("{}", opts.usage("Remove (unlink) the FILE(s)."));
        println!("By default, rm does not remove directories.  Use the --recursive (-r)");
        println!("option to remove each listed directory, too, along with all of its contents");
        println!("");
        println!("To remove a file whose name starts with a '-', for example '-foo',");
        println!("use one of these commands:");
        println!("rm -- -foo");
        println!("");
        println!("rm ./-foo");
        println!("");
        println!("Note that if you use rm to remove a file, it might be possible to recover");
        println!("some of its contents, given sufficient expertise and/or time.  For greater");
        println!("assurance that the contents are truly unrecoverable, consider using shred.");
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("for help, try '{0} --help'", NAME);
        return 1;
    } else {
        let force = matches.opt_present("force");
        let interactive =
            if matches.opt_present("i") {
                InteractiveMode::InteractiveAlways
            } else if matches.opt_present("I") {
                InteractiveMode::InteractiveOnce
            } else if matches.opt_present("interactive") {
                match &matches.opt_str("interactive").unwrap()[..] {
                    "none" => InteractiveMode::InteractiveNone,
                    "once" => InteractiveMode::InteractiveOnce,
                    "always" => InteractiveMode::InteractiveAlways,
                    val => {
                        crash!(1, "Invalid argument to interactive ({})", val)
                    }
                }
            } else {
                InteractiveMode::InteractiveNone
            };
        let one_fs = matches.opt_present("one-file-system");
        let preserve_root = !matches.opt_present("no-preserve-root");
        let recursive = matches.opt_present("recursive");
        let dir = matches.opt_present("dir");
        let verbose = matches.opt_present("verbose");
        if interactive == InteractiveMode::InteractiveOnce && (recursive || matches.free.len() > 3) {
            let msg =
                if recursive {
                    "Remove all arguments recursively? "
                } else {
                    "Remove all arguments? "
                };
            if !prompt(msg) {
                return 0;
            }
        }

        if remove(matches.free, force, interactive, one_fs, preserve_root, recursive, dir, verbose) {
            return 1;
        }
    }

    0
}

// TODO: implement one-file-system
#[allow(unused_variables)]
fn remove(files: Vec<String>, force: bool, interactive: InteractiveMode, one_fs: bool, preserve_root: bool, recursive: bool, dir: bool, verbose: bool) -> bool {
    let mut had_err = false;

    for filename in &files {
        let filename = &filename[..];
        let file = Path::new(filename);
        if file.exists() {
            let is_dir = match file.symlink_metadata() {
                Ok(metadata) => metadata.is_dir(),
                Err(e) => {
                    had_err = true;
                    show_error!("could not read metadata of '{}': {}", filename, e);
                    continue;
                }
            };
            if is_dir {
                if recursive && (filename != "/" || !preserve_root) {
                    if interactive != InteractiveMode::InteractiveAlways {
                        if let Err(e) = fs::remove_dir_all(file) {
                            had_err = true;
                            show_error!("could not remove '{}': {}", filename, e);
                        };
                    } else {
                        let mut dirs: VecDeque<PathBuf> = VecDeque::new();
                        let mut files: Vec<PathBuf> = Vec::new();
                        let mut rmdirstack: Vec<PathBuf> = Vec::new();
                        dirs.push_back(file.to_path_buf());

                        while !dirs.is_empty() {
                            let dir = dirs.pop_front().unwrap();
                            if !prompt(&(format!("rm: descend into directory '{}'? ", dir.display()))[..]) {
                                continue;
                            }

                            // iterate over items in this directory, adding to either file or
                            // directory queue
                            match fs::read_dir(dir.as_path()) {
                                Ok(rdir) => {
                                    for ent in rdir {
                                        match ent {
                                            Ok(ref f) => match f.file_type() {
                                                Ok(t) => {
                                                    if t.is_dir() {
                                                        dirs.push_back(f.path());
                                                    } else {
                                                        files.push(f.path());
                                                    }
                                                },
                                                Err(e) => {
                                                    had_err = true;
                                                    show_error!("reading '{}': {}", f.path().display(), e);
                                                },
                                            },
                                            Err(ref e) => {
                                                had_err = true;
                                                show_error!("recursing into '{}': {}", filename, e);
                                            },
                                        };
                                    }
                                },
                                Err(e) => {
                                    had_err = true;
                                    show_error!("could not recurse into '{}': {}", dir.display(), e);
                                    continue;
                                },
                            };

                            for f in &files {
                                had_err = remove_file(f.as_path(), interactive, verbose).bitor(had_err);
                            }

                            files.clear();
                            rmdirstack.push(dir);
                        }

                        for d in rmdirstack.iter().rev() {
                            had_err = remove_dir(d.as_path(), interactive, verbose).bitor(had_err);
                        }
                    }
                } else if dir && (filename != "/" || !preserve_root) {
                    had_err = remove_dir(&file, interactive, verbose).bitor(had_err);
                } else {
                    if recursive {
                        show_error!("could not remove directory '{}'", filename);
                        had_err = true;
                    } else {
                        show_error!("could not remove directory '{}' (did you mean to pass '-r'?)", filename);
                        had_err = true;
                    }
                }
            } else {
                had_err = remove_file(&file, interactive, verbose).bitor(had_err);
            }
        } else if !force {
            show_error!("no such file or directory '{}'", filename);
            had_err = true;
        }
    }

    had_err
}

fn remove_dir(path: &Path, interactive: InteractiveMode, verbose: bool) -> bool {
    let response =
        if interactive == InteractiveMode::InteractiveAlways {
            prompt_file(path, true)
        } else {
            true
        };
    if response {
        match fs::remove_dir(path) {
            Ok(_) => if verbose { println!("removed '{}'", path.display()); },
            Err(e) => {
                show_error!("removing '{}': {}", path.display(), e);
                return true;
            }
        }
    }

    false
}

fn remove_file(path: &Path, interactive: InteractiveMode, verbose: bool) -> bool {
    let response =
        if interactive == InteractiveMode::InteractiveAlways {
            prompt_file(path, false)
        } else {
            true
        };
    if response {
        match fs::remove_file(path) {
            Ok(_) => if verbose { println!("removed '{}'", path.display()); },
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
        prompt(&(format!("rm: remove directory '{}'? ", path.display()))[..])
    } else {
        prompt(&(format!("rm: remove file '{}'? ", path.display()))[..])
    }
}

fn prompt(msg: &str) -> bool {
    stderr().write_all(msg.as_bytes()).unwrap_or(());
    stderr().flush().unwrap_or(());
    let mut buf = Vec::new();
    let stdin = stdin();
    let mut stdin = stdin.lock();
    match stdin.read_until('\n' as u8, &mut buf) {
        Ok(x) if x > 0 => {
            match buf[0] {
                0x59 | 0x79 => true,
                _ => false,
            }
        }
        _ => false,
    }
}
