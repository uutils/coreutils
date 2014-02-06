#[crate_id(name="mkdir", vers="1.0.0", author="Nicholas Juszczak")];

/**
 * This file is part of the uutils coreutils package.
 *
 * (c) Nicholas Juszczak <juszczakn@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::{fs, stderr};
use std::num::strconv;
use extra::getopts::groups;

static VERSION: &'static str = "1.0.0";

/**
 * Handles option parsing
 */
fn main() {
    let args = os::args();
    
    let opts = ~[
        // Linux-specific options, not implemented
        // groups::optflag("Z", "context", "set SELinux secutiry context" +
        // " of each created directory to CTX"),
        groups::optopt("m", "mode", "set file mode", "755"),
        groups::optflag("p", "parents", "make parent directories as needed"),
        groups::optflag("v", "verbose",
                        "print a message for each printed directory"),
        groups::optflag("h", "help", "display this help"),
        groups::optflag("V", "version", "display this version")
    ];

    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut stderr() as &mut Writer,
                     "Invalid options\n{}", f.to_err_msg());
            os::set_exit_status(1);
            return;
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
            writeln!(&mut stderr() as &mut Writer,
                     "Error: no mode given");
            os::set_exit_status(1);
            return;
        }
    } else {
        0o755
    };

    let dirs = matches.free;
    exec(dirs, mk_parents, mode, verbose_flag);
}

fn print_help(opts: &[groups::OptGroup]) {
    println!("mkdir v{} - make a new directory with the given path", VERSION);
    println!("");
    println!("Usage:");
    print!("{}", groups::usage("Create the given DIRECTORY(ies)" +
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
            // if mkdir failed return
            if !mkdir(&path, mode) {return;}
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
            writeln!(&mut stderr() as &mut Writer,
                     "{}", error_msg);
            os::set_exit_status(1);
        }
    }
}

/**
 * Wrapper to catch errors, return false if failed
 */
fn mkdir(path: &Path, mode: u32) -> bool {
    match fs::mkdir(path, mode) {
        Ok(_) => true,
        Err(e) => {
            writeln!(&mut stderr() as &mut Writer,
                     "mkdir: test {}", e.to_str());
            os::set_exit_status(1);
            false
        }
    }
}
