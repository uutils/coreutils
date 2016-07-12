#![crate_name = "uu_install"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Ben Eills <ben@beneills.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use std::io::{Write};
use std::path::{Path, PathBuf};

static NAME: &'static str = "install";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflagopt("",  "backup", "make a backup of each existing destination file", "CONTROL");
    opts.optflag("b", "", "like --backup but does not accept an argument");
    opts.optflag("C", "compare", "compare each pair of source and destination files, and in\n \
                                  some cases, do not modify the destination at all");
    opts.optflag("d", "directory", "treat all arguments as directory names; create all\n \
                                    components of the specified directories");

    opts.optflag("D", "", "create all leading components of DEST except the last, then copy\n \
                           SOURCE to DEST");
    opts.optflagopt("g", "group", "set group ownership, instead of process' current group",
                    "GROUP");
    opts.optflagopt("m", "mode", "set permission mode (as in chmod), instead of rwxr-xr-x",
                    "MODE");
    opts.optflagopt("o", "owner", "set ownership (super-user only)",
                    "OWNER");
    opts.optflag("p", "preserve-timestamps", "apply access/modification times of SOURCE files\n \
                       to corresponding destination files");
    opts.optflag("s", "strip", "strip symbol tables");
    opts.optflagopt("", "strip-program", "program used to strip binaries", "PROGRAM");
    opts.optopt("S", "suffix", "override the usual backup suffix", "SUFFIX");
    opts.optopt("t", "target-directory", "move all SOURCE arguments into DIRECTORY", "DIRECTORY");
    opts.optflag("T", "no-target-directory", "treat DEST as a normal file");
    opts.optflag("v", "verbose", "explain what is being done");
    opts.optflag("P", "preserve-context", "preserve security context");
    opts.optflagopt("Z", "context", "set security context of files and directories", "CONTEXT");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            show_error!("Invalid options\n{}", f);
            return 1;
        }
    };
    let usage = opts.usage("Copy SOURCE to DEST or multiple SOURCE(s) to the existing\n \
                            DIRECTORY, while setting permission modes and owner/group");

    let paths: Vec<PathBuf> = {
        fn string_to_path<'a>(s: &'a String) -> &'a Path {
            Path::new(s)
        };
        let to_owned = |p: &Path| p.to_owned();
        let arguments = matches.free.iter().map(string_to_path);

        arguments.map(to_owned).collect()
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        0
    } else if matches.opt_present("help") {
        help(&usage);
        0
    } else {
        println!("Not implemented.");
        1
    }
}

fn help(usage: &str) {
    println!("{0} {1}\n\n\
    Usage: {0} SOURCE DEST\n   \
       or: {0} SOURCE... DIRECTORY\n\n\
    {2}", NAME, VERSION, usage);
}
