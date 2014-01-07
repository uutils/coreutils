#[crate_id(name="rm", vers="1.0.0", author="Arcterus")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::{stdin,stderr,stdio,fs,buffered,io_error};
use extra::getopts::groups;

enum InteractiveMode {
    InteractiveNone,
    InteractiveOnce,
    InteractiveAlways
}

impl Eq for InteractiveMode {
    fn eq(&self, other: &InteractiveMode) -> bool {
        match (*self, *other) {
            (InteractiveNone, InteractiveNone) |
            (InteractiveOnce, InteractiveOnce) |
            (InteractiveAlways, InteractiveAlways) => true,
            _ => false
        }
    }

    fn ne(&self, other: &InteractiveMode) -> bool {
        !self.eq(other)
    }
}

fn main() {
    let args = os::args();
    let program = args[0].clone();

    // TODO: make getopts support -R in addition to -r
    let opts = ~[
        groups::optflag("f", "force", "ignore nonexistent files and arguments, never prompt"),
        groups::optflag("i", "", "prompt before every removal"),
        groups::optflag("I", "", "prompt once before removing more than three files, or when removing recursively.  Less intrusive than -i, while still giving some protection against most mistakes"),
        groups::optflagopt("", "interactive", "prompt according to WHEN: never, once (-I), or always (-i).  Without WHEN, prompts always", "WHEN"),
        groups::optflag("", "one-file-system", "when removing a hierarchy recursively, skip any directory that is on a file system different from that of the corresponding command line argument (NOT IMPLEMENTED)"),
        groups::optflag("", "no-preserve-root", "do not treat '/' specially"),
        groups::optflag("", "preserve-root", "do not remove '/' (default)"),
        groups::optflag("r", "recursive", "remove directories and their contents recursively"),
        groups::optflag("d", "dir", "remove empty directories"),
        groups::optflag("v", "verbose", "explain what is being done"),
        groups::optflag("h", "help", "display this help and exit"),
        groups::optflag("V", "version", "output version information and exit")
    ];
    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut stderr() as &mut Writer,
                     "{}", f.to_err_msg());
            os::set_exit_status(1);
            return
        }
    };
    if matches.opt_present("help") {
        println("rm 1.0.0");
        println("");
        println("Usage:");
        println!("  {0:s} [OPTION]... [FILE]...", program);
        println("");
        print(groups::usage("Remove (unlink) the FILE(s).", opts));
        println("");
        println("By default, rm does not remove directories.  Use the --recursive (-r)");
        println("option to remove each listed directory, too, along with all of its contents");
        println("");
        println("To remove a file whose name starts with a '-', for example '-foo',");
        println("use one of these commands:");
        println("rm -- -foo");
        println("");
        println("rm ./-foo");
        println("");
        println("Note that if you use rm to remove a file, it might be possible to recover");
        println("some of its contents, given sufficient expertise and/or time.  For greater");
        println("assurance that the contents are truly unrecoverable, consider using shred.");
    } else if matches.opt_present("version") {
        println("rm 1.0.0");
    } else if matches.free.is_empty() {
        writeln!(&mut stderr() as &mut Writer, "Missing an argument");
        writeln!(&mut stderr() as &mut Writer,
                 "For help, try '{0:s} --help'", program);
        os::set_exit_status(1);
    } else {
        let force = matches.opt_present("force");
        let interactive =
            if matches.opt_present("i") {
                InteractiveAlways
            } else if matches.opt_present("I") {
                InteractiveOnce
            } else if matches.opt_present("interactive") {
                match matches.opt_str("interactive").unwrap() {
                    ~"none" => InteractiveNone,
                    ~"once" => InteractiveOnce,
                    ~"always" => InteractiveAlways,
                    val => {
                        writeln!(&mut stderr() as &mut Writer,
                                 "Invalid argument to interactive ({})", val);
                        os::set_exit_status(1);
                        return
                    }
                }
            } else {
                InteractiveNone
            };
        let one_fs = matches.opt_present("one-file-system");
        let preserve_root = !matches.opt_present("no-preserve-root");
        let recursive = matches.opt_present("recursive");
        let dir = matches.opt_present("dir");
        let verbose = matches.opt_present("verbose");
        if interactive == InteractiveOnce && (recursive || matches.free.len() > 3) {
            let msg =
                if recursive {
                    "Remove all arguments recursively? "
                } else {
                    "Remove all arguments? "
                };
            if !prompt(msg) {
                return;
            }
        }
        remove(matches.free, force, interactive, one_fs, preserve_root,
               recursive, dir, verbose);
    }
}

// TODO: implement one-file-system
fn remove(files: &[~str], force: bool, interactive: InteractiveMode, one_fs: bool, preserve_root: bool, recursive: bool, dir: bool, verbose: bool) {
    for filename in files.iter() {
        let file = Path::new(filename.to_owned());
        if file.exists() {
            if file.is_dir() {
                if recursive && (*filename != ~"/" || !preserve_root) {
                    remove(fs::walk_dir(&file).map(|x| x.as_str().unwrap().to_owned()).to_owned_vec(), force, interactive, one_fs, preserve_root, recursive, dir, verbose);
                    remove_dir(&file, *filename, interactive, verbose);
                } else if dir && (*filename != ~"/" || !preserve_root) {
                    remove_dir(&file, *filename, interactive, verbose);
                } else {
                    if recursive {
                        writeln!(&mut stderr() as &mut Writer,
                                 "Could not remove directory '{}'",
                                 *filename);
                    } else {
                        writeln!(&mut stderr() as &mut Writer,
                                 "Could not remove directory '{}' (did you mean to pass '-r'?)",
                                 *filename);
                    }
                    os::set_exit_status(1);
                }
            } else {
                remove_file(&file, *filename, interactive, verbose);
            }
        } else if !force {
            writeln!(&mut stderr() as &mut Writer,
                     "No such file or directory '{}'", *filename);
            os::set_exit_status(1);
        }
    }
}

fn remove_dir(path: &Path, name: &str, interactive: InteractiveMode, verbose: bool) {
    let response =
        if interactive == InteractiveAlways {
            prompt_file(path, name)
        } else {
            true
        };
    if response {
        io_error::cond.trap(|_| {
            writeln!(&mut stderr() as &mut Writer,
                     "Could not remove directory '{}'", name);
            os::set_exit_status(1);
        }).inside(|| {
            fs::rmdir(path);
            if verbose {
                println!("Removed '{}'", name);
            }
        });
    }
}

fn remove_file(path: &Path, name: &str, interactive: InteractiveMode, verbose: bool) {
    let response =
        if interactive == InteractiveAlways {
            prompt_file(path, name)
        } else {
            true
        };
    if response {
        io_error::cond.trap(|_| {
            writeln!(&mut stderr() as &mut Writer,
                     "Could not remove file '{}'", name);
            os::set_exit_status(1);
        }).inside(|| {
            fs::unlink(path);
            if verbose {
                println!("Removed '{}'", name);
            }
        });
    }
}

fn prompt_file(path: &Path, name: &str) -> bool {
    if path.is_dir() {
        prompt(format!("Remove directory '{}'? ", name))
    } else {
        prompt(format!("Remove file '{}'? ", name))
    }
}

fn prompt(msg: &str) -> bool {
    print(msg);
    read_prompt()
}

fn read_prompt() -> bool {
    stdio::flush();
    match buffered::BufferedReader::new(stdin()).read_line() {
        Some(line) => {
            match line.char_at(0) {
                'y' | 'Y' => true,
                'n' | 'N' => false,
                _ => {
                    print("Please enter either Y or N: ");
                    read_prompt()
                }
            }
        }
        None => true
    }
}

