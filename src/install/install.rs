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

use std::fs;
use std::io::{Write};
use std::path::{Path, PathBuf};
use std::result::Result;

static NAME: &'static str = "install";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct Behaviour {
    main_function: MainFunction,
    suffix: String,
    verbose: bool,
}

#[derive(Clone, Eq, PartialEq)]
pub enum MainFunction {
    Version,
    Help,
    Standard
}

pub fn uumain(args: Vec<String>) -> i32 {
    let opts = opts();

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            show_error!("Invalid options\n{}", f);
            return 1;
        }
    };

    let usage = opts.usage("Copy SOURCE to DEST or multiple SOURCE(s) to the existing\n \
                            DIRECTORY, while setting permission modes and owner/group");

    let behaviour = match behaviour(&matches) {
        Ok(x) => x,
        Err(ret) => {
            return ret;
        }
    };

    let paths: Vec<PathBuf> = {
        fn string_to_path<'a>(s: &'a String) -> &'a Path {
            Path::new(s)
        };
        let to_owned = |p: &Path| p.to_owned();
        let arguments = matches.free.iter().map(string_to_path);

        arguments.map(to_owned).collect()
    };

    match behaviour.main_function {
        MainFunction::Version => {
            println!("{} {}", NAME, VERSION);
            0
        },
        MainFunction::Help => {
            help(&usage);
            0
        },
        MainFunction::Standard => {
            exec(&paths[..], behaviour)
        }
    }
}

fn opts() -> getopts::Options {
    let mut opts = getopts::Options::new();

    // TODO implement flag
    opts.optflagopt("",  "backup", "make a backup of each existing destination file", "CONTROL");

    // TODO implement flag
    opts.optflag("b", "", "like --backup but does not accept an argument");

    // TODO implement flag
    opts.optflag("C", "compare", "compare each pair of source and destination files, and in\n \
                                  some cases, do not modify the destination at all");

    // TODO implement flag
    opts.optflag("d", "directory", "treat all arguments as directory names; create all\n \
                                    components of the specified directories");


    // TODO implement flagd
    opts.optflag("D", "", "create all leading components of DEST except the last, then copy\n \
                           SOURCE to DEST");

    // TODO implement flag
    opts.optflagopt("g", "group", "set group ownership, instead of process' current group",
                    "GROUP");

    // TODO implement flag
    opts.optflagopt("m", "mode", "set permission mode (as in chmod), instead of rwxr-xr-x",
                    "MODE");

    // TODO implement flag
    opts.optflagopt("o", "owner", "set ownership (super-user only)",
                    "OWNER");

    // TODO implement flag
    opts.optflag("p", "preserve-timestamps", "apply access/modification times of SOURCE files\n \
                       to corresponding destination files");

    // TODO implement flag
    opts.optflag("s", "strip", "strip symbol tables");

    // TODO implement flag
    opts.optflagopt("", "strip-program", "program used to strip binaries", "PROGRAM");

    // TODO implement flag
    opts.optopt("S", "suffix", "override the usual backup suffix", "SUFFIX");

    // TODO implement flag
    opts.optopt("t", "target-directory", "move all SOURCE arguments into DIRECTORY", "DIRECTORY");

    // TODO implement flag
    opts.optflag("T", "no-target-directory", "treat DEST as a normal file");

    // TODO implement flag
    opts.optflag("v", "verbose", "explain what is being done");

    // TODO implement flag
    opts.optflag("P", "preserve-context", "preserve security context");

    // TODO implement flag
    opts.optflagopt("Z", "context", "set security context of files and directories", "CONTEXT");

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    opts
}

fn behaviour(matches: &getopts::Matches) -> Result<Behaviour, i32> {
    let main_function = if matches.opt_present("version") {
        MainFunction::Version
    } else if matches.opt_present("help") {
        MainFunction::Help
    } else {
        MainFunction::Standard
    };

    let backup_suffix = if matches.opt_present("suffix") {
        match matches.opt_str("suffix") {
            Some(x) => x,
            None => {
                show_error!("option '--suffix' requires an argument\n\
                            Try '{} --help' for more information.", NAME);
                return Err(1);
            }
        }
    } else {
        "~".to_owned()
    };

    Ok(Behaviour {
        main_function: main_function,
        suffix: backup_suffix,
        verbose: matches.opt_present("v"),
    })
}

fn help(usage: &str) {
    println!("{0} {1}\n\n\
    Usage: {0} SOURCE DEST\n   \
       or: {0} SOURCE... DIRECTORY\n\n\
    {2}", NAME, VERSION, usage);
}

fn exec(files: &[PathBuf], b: Behaviour) -> i32 {
    if 2 == files.len() {
        move_files_into_dir(&files[0..1], &files[1], &b)
    } else {
        println!("Not implemented.");
        1
    }
}

fn move_files_into_dir(files: &[PathBuf], target_dir: &PathBuf, b: &Behaviour) -> i32 {
    if !target_dir.is_dir() {
        show_error!("target ‘{}’ is not a directory", target_dir.display());
        return 1;
    }

    let mut all_successful = true;
    for sourcepath in files.iter() {
        let targetpath = match sourcepath.as_os_str().to_str() {
            Some(name) => target_dir.join(name),
            None => {
                show_error!("cannot stat ‘{}’: No such file or directory",
                            sourcepath.display());

                all_successful = false;
                continue;
            }
        };

        if let Err(e) = rename(sourcepath, &targetpath, b) {
            show_error!("mv: cannot move ‘{}’ to ‘{}’: {}",
                        sourcepath.display(), targetpath.display(), e);
            all_successful = false;
        }
    };
    if all_successful { 0 } else { 1 }
}

fn rename(from: &PathBuf, to: &PathBuf, b: &Behaviour) -> std::io::Result<()> {
    let backup_path: Option<PathBuf> = None;

    try!(fs::rename(from, to));

    if b.verbose {
        print!("‘{}’ -> ‘{}’", from.display(), to.display());
        match backup_path {
            Some(path) => println!(" (backup: ‘{}’)", path.display()),
            None => println!("")
        }
    }
    Ok(())
}
