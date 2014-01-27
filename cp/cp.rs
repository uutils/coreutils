#[crate_id(name="cp", vers="1.0.0", author="Jordy Dickinson")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::fs;

mod util;

pub enum Mode {
    Copy,
    Help,
    Version,
}

fn main() {
    use extra::getopts::groups::{
        getopts,
        optflag,
        usage,
    };

    let args = os::args();
    let opts = ~[
        optflag("h", "help", "display this help and exit"),
        optflag("", "version", "output version information and exit"),
    ];
    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(e) => {
            error!("error: {:s}", e.to_err_msg());
            fail!()
        },
    };

    let progname = args[0].clone();
    let usage = usage("Copy SOURCE to DEST, or multiple SOURCE(s) to \
                       DIRECTORY.", opts);
    let mode = if matches.opt_present("version") {
        Version
    } else if matches.opt_present("help") {
        Help
    } else {
        Copy
    };
    // For now we assume that the first free argument is SOURCE and the
    // second free argument is DEST.
    let sources = if matches.free.len() < 1 {
        error!("error: Missing SOURCE argument. Try --help.");
        fail!()
    } else {
        ~[~Path::new(matches.free[0].clone())]
    };
    let dest = if matches.free.len() < 2 {
        error!("error: Missing DEST argument. Try --help.");
        fail!()
    } else {
        ~Path::new(matches.free[1].clone())
    };
    // Any other free arguments are ignored for now.

    match mode {
        Copy    => copy(sources, dest),
        Help    => help(progname, usage),
        Version => version(),
    }
}

fn version() {
    println!("cp 1.0.0");
}

fn help(progname: &str, usage: &str) {
    println!("Usage: {:s} SOURCE DEST", progname);
    println!("");
    println!("{:s}", usage);
}

fn copy(sources: &[~Path], dest: &Path) {
    // We assume there is only one source for now.
    let source = sources[0].clone();

    if util::paths_refer_to_same_file(source, dest) {
        error!("error: \"{:s}\" and \"{:s}\" are the same file",
               source.display().to_str(),
               dest.display().to_str());
        fail!();
    }

    // In the case of only one source and one destination, it's a simple file
    // copy operation.
    fs::copy(source, dest);
}
