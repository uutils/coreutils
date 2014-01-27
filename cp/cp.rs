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
use std::io;
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
        // All but the last argument:
        matches.free.slice(0, matches.free.len() - 2)
            .map(|arg| ~Path::new(arg.clone()))
    };
    let dest = if matches.free.len() < 2 {
        error!("error: Missing DEST argument. Try --help.");
        fail!()
    } else {
        // Only the last argument:
        ~Path::new(matches.free[matches.free.len() - 1].clone())
    };

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
    assert!(sources.len() >= 1);

    if sources.len() == 1 {
        let source = sources[0].clone();

        if util::paths_refer_to_same_file(source, dest) {
            error!("error: \"{:s}\" and \"{:s}\" are the same file",
                source.display().to_str(),
                dest.display().to_str());
            fail!();
        }

        fs::copy(source, dest);
    } else {
        if fs::stat(dest).kind != io::TypeDirectory {
            error!("error: TARGET must be a directory");
            fail!();
        }

        for source in sources.iter() {
            if fs::stat(*source).kind != io::TypeFile {
                error!("error: \"{:s}\" is not a file", source.display().to_str());
                continue;
            }

            let mut full_dest = dest.clone();
            
            full_dest.push(source.filename_str().unwrap());

            println!("{:s}", full_dest.display().to_str());

            fs::copy(*source, &full_dest);
        }
    }
}
