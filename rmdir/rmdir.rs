#[crate_id(name="rmdir", vers="1.0.0", author="Arcterus")];

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
use std::io::{print, stderr, io_error, fs};
use extra::getopts::groups;

fn main() {
    let args = os::args();
    let program = args[0].clone();

    let opts = ~[
        groups::optflag("", "ignore-fail-on-non-empty", "ignore each failure that is solely because a directory is non-empty"),
        groups::optflag("p", "parents", "remove DIRECTORY and its ancestors; e.g., 'rmdir -p a/b/c' is similar to rmdir a/b/c a/b a"),
        groups::optflag("v", "verbose", "output a diagnostic for every directory processed"),
        groups::optflag("h", "help", "print this help and exit"),
        groups::optflag("V", "version", "output version information and exit")
    ];
    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut stderr() as &mut Writer, "{}", f.to_err_msg());
            os::set_exit_status(1);
            return
        }
    };

    if matches.opt_present("help") {
        println!("rmdir 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION]... DIRECTORY...", program);
        println!("");
        print(groups::usage("Remove the DIRECTORY(ies), if they are empty.", opts));
    } else if matches.opt_present("version") {
        println!("rmdir 1.0.0");
    } else if matches.free.is_empty() {
        writeln!(&mut stderr() as &mut Writer, "Missing an argument");
        writeln!(&mut stderr() as &mut Writer,
                 "For help, try '{0:s} --help'", program);
        os::set_exit_status(1);
    } else {
        let ignore = matches.opt_present("ignore-fail-on-non-empty");
        let parents = matches.opt_present("parents");
        let verbose = matches.opt_present("verbose");
        remove(matches.free, ignore, parents, verbose);
    }
}

fn remove(dirs: &[~str], ignore: bool, parents: bool, verbose: bool) {
    for dir in dirs.iter() {
        let path = Path::new(dir.to_owned());
        if path.exists() {
            if path.is_dir() {
                remove_dir(&path, dir, ignore, parents, verbose);
            } else {
                writeln!(&mut stderr() as &mut Writer,
                         "Failed to remove '{}' (file)", *dir);
                os::set_exit_status(1);
            }
        } else {
            writeln!(&mut stderr() as &mut Writer,
                     "No such file or directory '{}'", *dir);
            os::set_exit_status(1);
        }
    }
}

fn remove_dir(path: &Path, dir: &~str, ignore: bool, parents: bool, verbose: bool) {
    if fs::walk_dir(path).next() == None {
        io_error::cond.trap(|_| {
            writeln!(&mut stderr() as &mut Writer,
                     "Failed to remove directory '{}'", *dir);
            os::set_exit_status(1);
        }).inside(|| {
            fs::rmdir(path);
            if verbose {
                println!("Removed directory '{}'", *dir);
            }
            if parents {
                let dirname = path.dirname_str().unwrap();
                if dirname != "." {
                    remove_dir(&Path::new(dirname), &dirname.to_owned(), ignore, parents, verbose);
                }
            }
        });
    } else if !ignore {
        writeln!(&mut stderr() as &mut Writer,
                 "Failed to remove directory '{}' (non-empty)", *dir);
        os::set_exit_status(1);
    }
}

