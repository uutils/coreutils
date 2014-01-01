#[crate_id(name="mkdir", vers="1.0.0", author="Nicholas Juszczak")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Nicholas Juszczak <juszczakn@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::stderr;
use extra::getopts::groups;
use std::io::fs;

static VERSION: &'static str = "1.0.0";

fn main() {
    let args: ~[~str] = os::args();
    
    let opts: ~[groups::OptGroup] = ~[
        // Linux-specific options
        // groups::optflag("m", "mode", "set file mode"),
        // groups::optflag("Z", "context", "set SELinux secutiry context" +
        // " of each created directory to CTX"),
        groups::optflag("p", "parents", "make parent directories as needed"),
        groups::optflag("v", "verbose",
                        "print a message for each printed directory"),
        groups::optflag("", "help", "display this help"),
        groups::optflag("", "version", "display this version")
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

    if matches.opt_present("help") {
        print_help(opts);
        return;
    }
    if matches.opt_present("version") {
        println("mkdir v" + VERSION);
        return;
    }

    let mk_parents: bool = matches.opt_present("parents");
    let dirs: ~[~str] = matches.free;
    mkdir(dirs, mk_parents);
}

fn print_help(opts: &[groups::OptGroup]) {
    println!("mkdir v{} - make a new directory with the given path", VERSION);
    println("");
    println("Usage:");
    print(groups::usage("Create the given DIRECTORY(ies)" +
                        " if they do not exist", opts));
}

/* std::libc currently does not provide bindings for any bits
besides user bits, so might as well use octal for now.
See 'std::io::FilePermission',
'std::libc::consts::os::posix88' for future updates */
fn mkdir(dirs: ~[~str], mk_parents: bool) {
    let default: u32 = 0o755;

    for dir in dirs.iter() {
        let path = Path::new((*dir).clone());
        // Recursively create parent dirs as needed
        if mk_parents {
            match path.dirname_str() {
                Some(p) => if p != "." {
                    mkdir(~[p.into_owned()], mk_parents)
                },
                None => ()
            }
        }
        if !path.exists() {
            println(*dir);
            fs::mkdir(&path, default);
        }
    }
}
