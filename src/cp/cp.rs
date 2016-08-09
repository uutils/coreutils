#![crate_name = "uu_cp"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 * (c) Jeremy Neptune <jerenept@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 *
 * Some parts of this program are modeled after Orvar Segerström and Sokovikov Evgeniy's work on
 * mv.rs .
 */

extern crate getopts;
extern crate walkdir;
#[macro_use]
extern crate uucore;

use getopts::Options;
use std::fs;
use std::io::{BufReader, BufRead, stdin, ErrorKind, Result, Write};
use std::path::Path;
use uucore::fs::{canonicalize, CanonicalizeMode};
use walkdir::WalkDir;

#[derive(Clone, Eq, PartialEq)]
pub enum Mode {
    Copy,
    Help,
    Version,
}

#[derive (Clone, Eq, PartialEq)]
pub enum OverwriteMode {
    NoClobber,
    Interactive,
    Force,
}

#[derive(Clone, Eq, PartialEq)]
pub enum BackupMode {
    NoBackup,
    SimpleBackup,
    NumberedBackup,
    ExistingBackup,
}

pub struct Behaviour {
    overwrite: OverwriteMode,
    backup: BackupMode,
    suffix: String,
    update: bool,
    target_dir: Option<String>,
    no_target_dir: bool,
    verbose: bool,
    link : bool,
    recursive : bool,
}
static NAME: &'static str = "cp";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");
    opts.optopt("t", "target-directory", "copy all SOURCE arguments into DIRECTORY", "DIRECTORY");
    opts.optflag("T", "no-target-directory", "Treat DEST as a regular file and not a directory");
    opts.optflag("v", "verbose", "explicitly state what is being done");
    opts.optflag("n", "no-clobber", "don't overwrite a file that already exists");
    opts.optflag("r", "recursive", "copy directories recursively");
    opts.optflag("l", "link", "hard-link files instead of copying");
    opts.optflag("i", "interactive", "ask before overwriting files");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            show_error!("{}", e);
            panic!()
        },
    };
    let usage = opts.usage("Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.");
    let mode = if matches.opt_present("version") {
        Mode::Version
    } else if matches.opt_present("help") {
        Mode::Help
    } else {
        Mode::Copy
    };

    match mode {
        Mode::Copy    => copy(matches),
        Mode::Help    => help(&usage),
        Mode::Version => version(),
    }

    0
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn help(usage: &str) {
    let msg = format!("{0} {1}\n\n\
                       Usage: {0} SOURCE DEST\n  \
                         or:  {0} SOURCE... DIRECTORY\n  \
                         or:  {0} -t DIRECTORY SOURCE...\n\
                       \n\
                       {2}", NAME, VERSION, usage);
    println!("{}", msg);
}

fn copy(matches: getopts::Matches) {
    let behavior = Behaviour {
        overwrite : if matches.opt_present("no-clobber") {
            OverwriteMode::NoClobber
        } else if matches.opt_present("interactive") {
            OverwriteMode::Interactive
        } else {
            OverwriteMode::Force
        },
        recursive : matches.opt_present("recursive"),
        backup : BackupMode::NoBackup, //TODO: actually do backup
        suffix : String::from("~"), //TODO: implement backup with suffix
        update : false, //TODO: implement updating
        //target_dir = if matches.opt_present("no-target-dir"){
        //    None todo: implement this shit
        //}
        verbose : matches.opt_present("verbose"),
        no_target_dir : matches.opt_present("no-target-directory"),
        link : matches.opt_present("link"),
        target_dir : matches.opt_str("target-directory"),

    };
    let sources: Vec<String> = if matches.free.is_empty() {
        show_error!("Missing SOURCE or DEST argument. Try --help.");
        panic!()
    } else if !behavior.target_dir.is_some() {
        matches.free[..matches.free.len() - 1].iter().cloned().collect()
    } else {
        matches.free.iter().cloned().collect()
    };
    let dest_str = if behavior.target_dir.is_some() {
        matches.opt_str("target-directory").expect("Option -t/--target-directory requires an argument")
    } else {
        matches.free[matches.free.len() - 1].clone()
    };
    if behavior.no_target_dir && behavior.target_dir.is_some() {
        show_error!("Options --no-target-dir and --target-dir are mutually exclusive");
        panic!()
    }
    let dest = if matches.free.len() < 2 && !behavior.target_dir.is_some() {
        show_error!("Missing DEST argument. Try --help.");
        panic!()
    } else {
        //the argument to the -t/--target-directory= options
        let path = Path::new(&dest_str);
        if !path.is_dir() && behavior.target_dir.is_some() {
            show_error!("Target {} is not a directory",behavior.target_dir.unwrap());
            panic!()
        } else {
            path
        }

    };

    assert!(sources.len() >= 1);
    if behavior.no_target_dir && dest.is_dir() {
        show_error!("Can't overwrite directory {} with non-directory", dest.display());
        panic!()
    }


    if !dest.is_dir() && sources.len() != 1  {
        show_error!("TARGET must be a directory");
        panic!();
    }
    if !dest.exists() && (sources.len() != 1 && Path::new(&sources[0]).is_dir()) {
        let io_result = fs::create_dir(dest.clone()).err();
        match io_result {
            None => {},
            Some(t) => {
                show_error!("{}", t);
                panic!()
            }
        }
    }
    let folder_copy = !dest.exists();
    'outer: for src in &sources {
        for item in WalkDir::new(src){
            let item1 = item.unwrap();
            let item=item1.path();
            if !(item.is_dir() || item.is_file()) {
                show_error!("{} is invalid or inaccessible", item.display());
                panic!()
            }
            let full_dest = if Path::new(src).is_dir() && !folder_copy {
                dest.join(item.strip_prefix(Path::new(src).parent().unwrap()).unwrap()) //Christmas day!
            /*
                If the source of the files (as given by the user args) is a directory
                the ending destination of a particular file is:
                the
            */
            } else if Path::new(src).is_dir() && folder_copy {
                dest.join(item.canonicalize().unwrap().strip_prefix(&Path::new(src).canonicalize().unwrap()).unwrap())
            } else if !dest.is_dir() { //source is not a directory, for the rest
                dest.to_path_buf() //figure out how to copy a directory; not copy a directory into another
            } else  {
                dest.join(item)
            };
            //println!("{:?}", None);
            if item.is_dir() {
                if !behavior.recursive {
                    println!("{}: skipping directory '{}'", NAME, item.display());
                    continue 'outer;
                }
                if behavior.verbose {
                    println!("{} -> {}", item.display(), full_dest.display());
                }
                if full_dest.is_dir() {
                    continue; //merge the directories that already exist
                }
                match fs::create_dir_all(full_dest.clone()) {
                    Err(e) => {
                        show_error!("{}", e);
                        panic!();
                    },
                    Ok(t) => {
                        let permissions = fs::metadata(item).unwrap().permissions();
                         match fs::set_permissions(full_dest, permissions) {
                             Ok(t) => t,
                             Err(t) => show_error!("{}", t)
                         }
                        t
                    },
                }
            } else {
                let same_file = paths_refer_to_same_file(item, full_dest.as_path()).unwrap_or_else(|err| {
                    match err.kind() {
                        ErrorKind::NotFound => false,
                        _ => {
                            show_error!("{}", err);
                            panic!()
                        }
                    }
                });
                if !item.is_file() {
                    show_error!("\"{}\" is not a file", item.display());
                    continue;
                }
                if same_file {
                    show_error!("\"{}\" and \"{}\" are the same file",
                        item.display(),
                        full_dest.display());
                    panic!();
                }
                if full_dest.exists() {
                    match behavior.overwrite {
                        OverwriteMode::NoClobber => {
                            show_error!("Not overwriting {} because of option 'no-clobber'", full_dest.display());
                            continue; //if the destination file exists, we promised not to overwrite
                        },
                        OverwriteMode::Interactive => {
                            if !read_yes() {
                                continue;
                            }
                        },
                        OverwriteMode::Force => {
                            let io_result = fs::remove_file(full_dest.clone()).err();
                            match io_result {
                                None => {},
                                Some(t) => {
                                    show_error!("{}", t);
                                    panic!()
                                }
                            }
                        },

                    }
                }
                if behavior.verbose {
                    println!("{} -> {}", item.display(), full_dest.display());
                }
                let io_result = if behavior.link {
                    fs::hard_link(item, full_dest).err()
                } else {
                    fs::copy(item, full_dest).err() //carry out the copy
                };
                match io_result {
                    None => continue,
                    Some(t) => {
                        show_error!("{}", t);
                        panic!()
                    }
                }
            }
        }
    }
}

pub fn paths_refer_to_same_file(p1: &Path, p2: &Path) -> Result<bool> {
    // We have to take symlinks and relative paths into account.
    let pathbuf1 = try!(canonicalize(p1, CanonicalizeMode::Normal));
    let pathbuf2 = try!(canonicalize(p2, CanonicalizeMode::Normal));

    Ok(pathbuf1 == pathbuf2)
}

fn read_yes() -> bool {
    let mut s = String::new();
    match BufReader::new(stdin()).read_line(&mut s) {
        Ok(_) => match s.char_indices().nth(0) {
            Some((_, x)) => x == 'y' || x == 'Y',
            _ => false
        },
        _ => false
    }
}