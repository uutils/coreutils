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
extern crate remove_dir_all;
extern crate walkdir;

#[macro_use]
extern crate uucore;

use std::collections::VecDeque;
use std::fs;
use std::io::{stdin, stderr, BufRead, Write};
use std::ops::BitOr;
use std::path::Path;
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
        let options = Options {
            force: matches.opt_present("force"),
            interactive: {
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
                }
            },
            one_fs: matches.opt_present("one-file-system"),
            preserve_root: !matches.opt_present("no-preserve-root"),
            recursive: matches.opt_present("recursive"),
            dir: matches.opt_present("dir"),
            verbose: matches.opt_present("verbose")
        };
        if options.interactive == InteractiveMode::InteractiveOnce
                && (options.recursive || matches.free.len() > 3) {
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

        if remove(matches.free, options) {
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
