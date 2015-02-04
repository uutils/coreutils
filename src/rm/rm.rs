#![crate_name = "rm"]
#![feature(collections, core, io, libc, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use std::old_io::{print, stdin, stdio, fs, BufferedReader};
use std::old_io::fs::PathExtensions;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

#[derive(Eq, PartialEq)]
enum InteractiveMode {
    InteractiveNone,
    InteractiveOnce,
    InteractiveAlways
}

impl Copy for InteractiveMode {}

static NAME: &'static str = "rm";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();

    // TODO: make getopts support -R in addition to -r
    let opts = [
        getopts::optflag("f", "force", "ignore nonexistent files and arguments, never prompt"),
        getopts::optflag("i", "", "prompt before every removal"),
        getopts::optflag("I", "", "prompt once before removing more than three files, or when removing recursively.  Less intrusive than -i, while still giving some protection against most mistakes"),
        getopts::optflagopt("", "interactive", "prompt according to WHEN: never, once (-I), or always (-i).  Without WHEN, prompts always", "WHEN"),
        getopts::optflag("", "one-file-system", "when removing a hierarchy recursively, skip any directory that is on a file system different from that of the corresponding command line argument (NOT IMPLEMENTED)"),
        getopts::optflag("", "no-preserve-root", "do not treat '/' specially"),
        getopts::optflag("", "preserve-root", "do not remove '/' (default)"),
        getopts::optflag("r", "recursive", "remove directories and their contents recursively"),
        getopts::optflag("d", "dir", "remove empty directories"),
        getopts::optflag("v", "verbose", "explain what is being done"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "{}", f)
        }
    };
    if matches.opt_present("help") {
        println!("rm 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION]... [FILE]...", program);
        println!("");
        print(getopts::usage("Remove (unlink) the FILE(s).", &opts).as_slice());
        println!("");
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
        println!("rm 1.0.0");
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("for help, try '{0} --help'", program);
        return 1;
    } else {
        let force = matches.opt_present("force");
        let interactive =
            if matches.opt_present("i") {
                InteractiveMode::InteractiveAlways
            } else if matches.opt_present("I") {
                InteractiveMode::InteractiveOnce
            } else if matches.opt_present("interactive") {
                match matches.opt_str("interactive").unwrap().as_slice() {
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
        if let Err(e) = remove(matches.free, force, interactive, one_fs, preserve_root,
                               recursive, dir, verbose) {
            return e;
        }
    }

    0
}

// TODO: implement one-file-system
fn remove(files: Vec<String>, force: bool, interactive: InteractiveMode, one_fs: bool, preserve_root: bool, recursive: bool, dir: bool, verbose: bool) -> Result<(), isize> {
    let mut r = Ok(());

    for filename in files.iter() {
        let filename = filename.as_slice();
        let file = Path::new(filename);
        if file.exists() {
            if file.is_dir() {
                if recursive && (filename != "/" || !preserve_root) {
                    let walk_dir = match fs::walk_dir(&file) {
                        Ok(m) => m,
                        Err(f) => {
                            crash!(1, "{}", f.to_string());
                        }
                    };
                    r = remove(walk_dir.map(|x| x.as_str().unwrap().to_string()).collect(), force, interactive, one_fs, preserve_root, recursive, dir, verbose).and(r);
                    r = remove_dir(&file, filename, interactive, verbose).and(r);
                } else if dir && (filename != "/" || !preserve_root) {
                    r = remove_dir(&file, filename, interactive, verbose).and(r);
                } else {
                    if recursive {
                        show_error!("could not remove directory '{}'",
                                       filename);
                        r = Err(1);
                    } else {
                        show_error!("could not remove directory '{}' (did you mean to pass '-r'?)",
                                    filename);
                        r = Err(1);
                    }
                }
            } else {
                r = remove_file(&file, filename.as_slice(), interactive, verbose).and(r);
            }
        } else if !force {
            show_error!("no such file or directory '{}'", filename);
            r = Err(1);
        }
    }

    r
}

fn remove_dir(path: &Path, name: &str, interactive: InteractiveMode, verbose: bool) -> Result<(), isize> {
    let response =
        if interactive == InteractiveMode::InteractiveAlways {
            prompt_file(path, name)
        } else {
            true
        };
    if response {
        match fs::rmdir(path) {
            Ok(_) => if verbose { println!("Removed '{}'", name); },
            Err(f) => {
                show_error!("{}", f.to_string());
                return Err(1);
            }
        }
    }

    Ok(())
}

fn remove_file(path: &Path, name: &str, interactive: InteractiveMode, verbose: bool) -> Result<(), isize> {
    let response =
        if interactive == InteractiveMode::InteractiveAlways {
            prompt_file(path, name)
        } else {
            true
        };
    if response {
        match fs::unlink(path) {
            Ok(_) => if verbose { println!("Removed '{}'", name); },
            Err(f) => {
                show_error!("{}", f.to_string());
                return Err(1);
            }
        }
    }

    Ok(())
}

fn prompt_file(path: &Path, name: &str) -> bool {
    if path.is_dir() {
        prompt(format!("Remove directory '{}'? ", name).as_slice())
    } else {
        prompt(format!("Remove file '{}'? ", name).as_slice())
    }
}

fn prompt(msg: &str) -> bool {
    print(msg);
    read_prompt()
}

fn read_prompt() -> bool {
    stdio::flush();
    match BufferedReader::new(stdin()).read_line() {
        Ok(line) => {
            match line.as_slice().char_at(0) {
                'y' | 'Y' => true,
                'n' | 'N' => false,
                _ => {
                    print!("Please enter either Y or N: ");
                    read_prompt()
                }
            }
        }
        Err(_) => true
    }
}
