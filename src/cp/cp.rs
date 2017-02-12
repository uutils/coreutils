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

pub struct CopyMode {
    link : bool,
    backup_mode : BackupMode,
    backup_suffix : String,
    verbose : bool,
    update : bool,
    overwrite: OverwriteMode,
}

pub struct Behaviour {
    copy_mode : CopyMode,
    target_dir: Option<String>,
    no_target_dir: bool,
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
    opts.optflag("u", "update", "copy when SOURCE is newer than DEST, or DEST is missing");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            crash!(1, "{}", e)
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
}

fn version() -> i32 {
    println!("{} {}", NAME, VERSION);
    0
}

fn help(usage: &str) -> i32 {
    let msg = format!("{0} {1}\n\n\
                       Usage: {0} SOURCE DEST\n  \
                         or:  {0} SOURCE... DIRECTORY\n  \
                         or:  {0} -t DIRECTORY SOURCE...\n\
                       \n\
                       {2}", NAME, VERSION, usage);
    println!("{}", msg);
    0
}

fn copy(matches: getopts::Matches) -> i32 {
    let copy_mode = CopyMode {
        link : matches.opt_present("link"),
        backup_mode : BackupMode::NoBackup, //TODO: Impl backup. 
        backup_suffix : String::from("~"),
        verbose : matches.opt_present("verbose"),
        update : matches.opt_present("update"),
        overwrite : if matches.opt_present("no-clobber"){
            OverwriteMode::NoClobber
        } else if matches.opt_present("interactive") {
            OverwriteMode::Interactive
        } else {
            OverwriteMode::Force
        },
    };
    let behavior = Behaviour {
        copy_mode : copy_mode,
        target_dir : matches.opt_str("target-directory"),
        no_target_dir : matches.opt_present("no-target-directory"),
        recursive : matches.opt_present("recursive"),
    };
    let sources: Vec<String> = if matches.free.is_empty() {
        crash!(1, "Missing SOURCE or DEST argument. Try --help for usage")
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
        crash!(1, "Options --no-target-dir and --target-dir are mutually exclusive");
    }
    let dest = if matches.free.len() < 2 && !behavior.target_dir.is_some() {
        crash!(1, "Missing DEST argument. Try --help for usage")
    } else {
        //the argument to the -t/--target-directory= options
        let path = Path::new(&dest_str);
        if !path.is_dir() && behavior.target_dir.is_some() {
            crash!(1, "Target {} is not a directory", behavior.target_dir.unwrap())
        } else {
            path
        }

    };

    if sources.len() < 1 {
        crash!(1, "No source files specified. Try --help for usage")
    }
    //assert!(sources.len() >= 1);
    if behavior.no_target_dir && dest.is_dir() {
        crash!(1, "Can't overwrite directory {} with non-directory", dest.display())
    }


    if !dest.is_dir() && sources.len() != 1  {
        crash!(1, "Multiple SOURCE files can only be copied to a directory")
    }
    if !dest.exists() && (sources.len() != 1 && Path::new(&sources[0]).is_dir()) {
        let io_result = fs::create_dir(dest.clone()).err();
        match io_result {
            None => {},
            Some(t) => {
                crash!(1, "{}", t);
            }
        }
    }
    let mut return_code = 0;
    let folder_copy = !dest.exists();
    'outer: for src in &sources {
        for item in WalkDir::new(src){
            let item1 = item.unwrap();
            let item=item1.path();
            if !(item.is_dir() || item.is_file()) {
                crash!(1, "{} is invalid or inaccessible", item.display())
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
            if item.is_dir() {
                if !behavior.recursive {
                    println!("{}: skipping directory '{}' (use -r/--recursive to include directories)", NAME, item.display());
                    continue 'outer;
                }
                if behavior.copy_mode.verbose {
                    println!("{} -> {}", item.display(), full_dest.display());
                }
                if full_dest.is_dir() {
                    continue; //merge the directories that already exist
                }
                match fs::create_dir_all(full_dest.clone()) {
                    Err(e) => {
                        crash!(1, "{}", e)
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
                match file_copy(&item, &full_dest, &behavior.copy_mode) {
                    Ok(result_code) => {
                        if return_code == 0 && return_code != result_code {
                            return_code = result_code;
                        }
                    },
                    Err(e) => {
                        crash!(1, "{}", e);
                    }
                }
            }
        }
    }
    return_code
}

pub fn paths_refer_to_same_file(p1: &Path, p2: &Path) -> Result<bool> {
    // We have to take symlinks and relative paths into account.
    let pathbuf1 = canonicalize(p1, CanonicalizeMode::Normal)?;
    let pathbuf2 = canonicalize(p2, CanonicalizeMode::Normal)?;

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

fn file_copy(source: &Path, dest: &Path, copy_mode: &CopyMode) -> Result<i32>  {
    let same_file = match paths_refer_to_same_file(source, dest) {
        Ok(result) => {
            result
        } ,
        Err(e) => {
            return Err(e);
        }
    }; 

//    let src_metadata = source.metadata()?;  TODO: GNU cp also quits if the two files have the same inode number. 
//    let dest_metadata = dest.metadata()?;   unsure how/if to implement this while preserving cross-platform compat
    if !source.is_file() {
        show_error!("\"{}\" is not a file", source.display());
        return Ok(1);
    }
    if same_file {
        show_error!("\"{} and \"{}\" are the same file",
            source.display(),
            dest.display());
            return Ok(1);
    }
    if dest.exists() {
        match copy_mode.overwrite {
            OverwriteMode::NoClobber => {
                show_error!("Not overwriting {} because of option \"no-clobber\"", dest.display());
                    return Ok(0); //Should I warn if a file is skipped because of no-clobber? GNU cp does not.
            },
            OverwriteMode::Interactive => {
                println!("Overwrite \"{}\"?", dest.display());
                if !read_yes() {
                    return Ok(0);
                }
            }
            OverwriteMode::Force => {
                let io_result = fs::remove_file(dest.clone());
                if io_result.is_err() {
                    return Err(io_result.err().unwrap());
                }
            }
            
        }
    }
//execute the copy itself
    if copy_mode.verbose {
        println!("{} -> {}", source.display(), dest.display());
    }
    if copy_mode.link {
        match fs::hard_link(source, dest) {
            Ok(_) => Ok(0),
            Err(e) => Err(e),
        }
    } else {
        match fs::copy(source, dest) {
            Ok(_) => Ok(0),
            Err(e) => Err(e),
        }
    }
}

