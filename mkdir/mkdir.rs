#[crate_id(name="mkdir", vers="1.0.0", author="Nicholas Juszczak")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Nicholas Juszczak <juszczakn@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[feature(macro_rules)];

extern crate extra;
extern crate getopts;

use std::os;
use std::io::fs;
use std::num::strconv;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "mkdir";
static VERSION: &'static str = "1.0.0";

/**
 * Handles option parsing
 */
fn main() {
    let args = os::args();
    
    let opts = ~[
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

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "Invalid options\n{}", f.to_err_msg());
        }
    };

    if args.len() == 1 || matches.opt_present("help") {
        print_help(opts);
        return;
    }
    if matches.opt_present("version") {
        println!("mkdir v{}", VERSION);
        return;
    }
    let verbose_flag = matches.opt_present("verbose");
    let mk_parents = matches.opt_present("parents");

    // Translate a ~str in octal form to u32, default to 755
    // Not tested on Windows
    let mode_match = matches.opts_str(&[~"mode"]);
    let mode: u32 = if mode_match.is_some() {
        let m = mode_match.unwrap();
        let res = strconv::from_str_common(m, 8, false, false, false,
                                           strconv::ExpNone,
                                           false, false);
        if res.is_some() {
            res.unwrap()
        } else {
            crash!(1, "no mode given");
        }
    } else {
        0o755
    };

    let dirs = matches.free;
    exec(dirs, mk_parents, mode, verbose_flag);
}

fn print_help(opts: &[getopts::OptGroup]) {
    println!("mkdir v{} - make a new directory with the given path", VERSION);
    println!("");
    println!("Usage:");
    print!("{}", getopts::usage("Create the given DIRECTORY(ies)" +
                               " if they do not exist", opts));
}

/**
 * Create the list of new directories
 */
fn exec(dirs: ~[~str], mk_parents: bool, mode: u32, verbose: bool) {
    let mut parent_dirs: ~[~str] = ~[];
    if mk_parents {
        for dir in dirs.iter() {
            let path = Path::new((*dir).clone());
            // Build list of parent dirs which need to be created
            let parent = path.dirname_str();
            match parent {
                Some(p) => {
                    if !Path::new(p).exists() {
                        parent_dirs.push(p.into_owned())
                    }
                },
                None => ()
            }
        }
    }
    // Recursively build parent dirs that are needed
    if !parent_dirs.is_empty() {
        exec(parent_dirs, mk_parents, mode, verbose);
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
        } else {
            let mut error_msg = ~"";
            if !parent_exists {
                error_msg.push_str("Error: parent directory '");
                error_msg.push_str(parent);
                error_msg.push_str("' does not exist");
            } else {
                error_msg.push_str("Error: directory '");
                error_msg.push_str(*dir);
                error_msg.push_str("' already exists");
            }
            show_error!(1, "{}", error_msg);
        }
    }
}

/**
 * Wrapper to catch errors, return false if failed
 */
fn mkdir(path: &Path, mode: u32) {
    match fs::mkdir(path, mode) {
        Ok(_) => {},
        Err(e) => {
            crash!(1, "test {}", e.to_str());
        }
    }
}
