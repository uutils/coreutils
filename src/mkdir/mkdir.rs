#![crate_name = "mkdir"]
#![feature(collections, core, io, libc, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Nicholas Juszczak <juszczakn@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use std::old_io::fs::{self, PathExtensions};
use std::old_io::FilePermission;
use std::num::from_str_radix;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "mkdir";
static VERSION: &'static str = "1.0.0";

/**
 * Handles option parsing
 */
pub fn uumain(args: Vec<String>) -> isize {

    let opts = [
        // Linux-specific options, not implemented
        // getopts::optflag("Z", "context", "set SELinux secutiry context" +
        // " of each created directory to CTX"),
        getopts::optopt("m", "mode", "set file mode", "755"),
        getopts::optflag("p", "parents", "make parent directories as needed"),
        getopts::optflag("v", "verbose",
                        "print a message for each printed directory"),
        getopts::optflag("h", "help", "display this help"),
        getopts::optflag("V", "version", "display this version")
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "Invalid options\n{}", f);
        }
    };

    if args.len() == 1 || matches.opt_present("help") {
        print_help(&opts);
        return 0;
    }
    if matches.opt_present("version") {
        println!("mkdir v{}", VERSION);
        return 0;
    }
    let verbose_flag = matches.opt_present("verbose");
    let mk_parents = matches.opt_present("parents");

    // Translate a ~str in octal form to u32, default to 755
    // Not tested on Windows
    let mode_match = matches.opts_str(&["mode".to_string()]);
    let mode: FilePermission = if mode_match.is_some() {
        let m = mode_match.unwrap();
        let res: Option<u32> = from_str_radix(m.as_slice(), 8).ok();
        if res.is_some() {
            unsafe { std::mem::transmute(res.unwrap()) }
        } else {
            crash!(1, "no mode given");
        }
    } else {
        unsafe { std::mem::transmute(0o755 as u32) }
    };

    let dirs = matches.free;
    if dirs.is_empty() {
        crash!(1, "missing operand");
    }
    match exec(dirs, mk_parents, mode, verbose_flag) {
        Ok(()) => 0,
        Err(e) => e
    }
}

fn print_help(opts: &[getopts::OptGroup]) {
    println!("mkdir v{} - make a new directory with the given path", VERSION);
    println!("");
    println!("Usage:");
    print!("{}", getopts::usage("Create the given DIRECTORY(ies) if they do not exist", opts));
}

/**
 * Create the list of new directories
 */
fn exec(dirs: Vec<String>, mk_parents: bool, mode: FilePermission, verbose: bool) -> Result<(), isize> {
    let mut result = Ok(());

    let mut parent_dirs = Vec::new();
    if mk_parents {
        for dir in dirs.iter() {
            let path = Path::new((*dir).clone());
            // Build list of parent dirs which need to be created
            let parent = path.dirname_str();
            match parent {
                Some(p) => {
                    if !Path::new(p).exists() {
                        parent_dirs.push(p.to_string())
                    }
                },
                None => ()
            }
        }
    }
    // Recursively build parent dirs that are needed
    if !parent_dirs.is_empty() {
        match exec(parent_dirs, mk_parents, mode, verbose) {
            Ok(()) => ( /* keep going */ ),
            Err(e) => result = Err(e)
        }
    }

    for dir in dirs.iter() {
        let path = Path::new((*dir).clone());
        // Determine if parent directory to the one to 
        // be created exists
        let parent = match path.dirname_str() {
            Some(p) => p,
            None => ""
        };
        let parent_exists = Path::new(parent).exists();
        if parent_exists && !path.exists() {
            mkdir(&path, mode);
            if verbose {println!("{}", *dir);}
        } else if !mk_parents {
            let error_msg =
                if !parent_exists {
                    format!("parent directory '{}' does not exist", parent)
                } else {
                    format!("directory '{}' already exists", *dir)
                };
            show_error!("{}", error_msg);
            result = Err(1)
        }
    }

    result
}

/**
 * Wrapper to catch errors, return false if failed
 */
fn mkdir(path: &Path, mode: FilePermission) {
    match fs::mkdir(path, mode) {
        Ok(_) => {},
        Err(e) => {
            crash!(1, "test {}", e.to_string());
        }
    }
}
