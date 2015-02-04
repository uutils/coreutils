#![crate_name = "mv"]
#![feature(collections, core, io, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Orvar Segerström <orvarsegerstrom@gmail.com>
 * (c) Sokovikov Evgeniy  <skv-headless@yandex.ru>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;

use std::old_io::{BufferedReader, IoResult, fs};
use std::old_io::stdio::stdin_raw;
use std::old_io::fs::PathExtensions;
use std::path::GenericPath;
use getopts::{
    getopts,
    optflag,
    optflagopt,
    optopt,
    usage,
};
use std::borrow::ToOwned;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "mv";
static VERSION:  &'static str = "0.0.1";

pub struct Behaviour {
    overwrite: OverwriteMode,
    backup: BackupMode,
    suffix: String,
    update: bool,
    target_dir: Option<String>,
    no_target_dir: bool,
    verbose: bool,
}

#[derive(Eq, PartialEq)]
pub enum OverwriteMode {
    NoClobber,
    Interactive,
    Force,
}

impl Copy for OverwriteMode {}

#[derive(Eq, PartialEq)]
pub enum BackupMode {
    NoBackup,
    SimpleBackup,
    NumberedBackup,
    ExistingBackup,
}

impl Copy for BackupMode {}

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].as_slice();
    let opts = [
        optflagopt("",  "backup", "make a backup of each existing destination file", "CONTROL"),
        optflag("b", "", "like --backup but does not accept an argument"),
        optflag("f", "force", "do not prompt before overwriting"),
        optflag("i", "interactive", "prompt before override"),
        optflag("n", "no-clobber", "do not overwrite an existing file"),
        // I have yet to find a use-case (and thereby write a test) where this option is useful.
        //optflag("",  "strip-trailing-slashes", "remove any trailing slashes from each SOURCE\n \
        //                                        argument"),
        optopt("S", "suffix", "override the usual backup suffix", "SUFFIX"),
        optopt("t", "target-directory", "move all SOURCE arguments into DIRECTORY", "DIRECTORY"),
        optflag("T", "no-target-directory", "treat DEST as a normal file"),
        optflag("u", "update", "move only when the SOURCE file is newer\n \
                                  than the destination file or when the\n \
                                  destination file is missing"),
        optflag("v", "verbose", "explain what is being done"),
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            show_error!("Invalid options\n{}", f);
            return 1;
        }
    };
    let usage = usage("Move SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.", &opts);

    /* This does not exactly match the GNU implementation:
     * The GNU mv defaults to Force, but if more than one of the
     * overwrite options are supplied, only the last takes effect.
     * To default to no-clobber in that situation seems safer:
     */
    let overwrite_mode = if matches.opt_present("no-clobber") {
        OverwriteMode::NoClobber
    } else if matches.opt_present("interactive") {
        OverwriteMode::Interactive
    } else {
        OverwriteMode::Force
    };

    let backup_mode = if matches.opt_present("b") {
        BackupMode::SimpleBackup
    } else if matches.opt_present("backup") {
        match matches.opt_str("backup") {
            None => BackupMode::SimpleBackup,
            Some(mode) => match mode.as_slice() {
                "simple" | "never" => BackupMode::SimpleBackup,
                "numbered" | "t"   => BackupMode::NumberedBackup,
                "existing" | "nil" => BackupMode::ExistingBackup,
                "none" | "off"     => BackupMode::NoBackup,
                x => {
                    show_error!("invalid argument ‘{}’ for ‘backup type’\n\
                                Try 'mv --help' for more information.", x);
                    return 1;
                }
            }
        }
    } else {
        BackupMode::NoBackup
    };

    if overwrite_mode == OverwriteMode::NoClobber && backup_mode != BackupMode::NoBackup {
        show_error!("options --backup and --no-clobber are mutually exclusive\n\
                    Try 'mv --help' for more information.");
        return 1;
    }

    let backup_suffix = if matches.opt_present("suffix") {
        match matches.opt_str("suffix") {
            Some(x) => x,
            None => {
                show_error!("option '--suffix' requires an argument\n\
                            Try 'mv --help' for more information.");
                return 1;
            }
        }
    } else {
        "~".to_owned()
    };

    if matches.opt_present("T") && matches.opt_present("t") {
        show_error!("cannot combine --target-directory (-t) and --no-target-directory (-T)");
        return 1;
    }

    let behaviour = Behaviour {
        overwrite: overwrite_mode,
        backup: backup_mode,
        suffix: backup_suffix,
        update: matches.opt_present("u"),
        target_dir: matches.opt_str("t"),
        no_target_dir: matches.opt_present("T"),
        verbose: matches.opt_present("v"),
    };

    let string_to_path = |&: s: &String| { Path::new(s.as_slice()) };
    let paths: Vec<Path> = matches.free.iter().map(string_to_path).collect();

    if matches.opt_present("version") {
        version();
        0
    } else if matches.opt_present("help") {
        help(program.as_slice(), usage.as_slice());
        0
    } else {
        exec(paths.as_slice(), behaviour)
    }
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn help(progname: &str, usage: &str) {
    let msg = format!("Usage: {0} SOURCE DEST\n  \
                         or:  {0} SOURCE... DIRECTORY \
                       \n\
                       {1}", progname, usage);
    println!("{}", msg);
}

fn exec(files: &[Path], b: Behaviour) -> isize {
    match b.target_dir {
        Some(ref name) => return move_files_into_dir(files, &Path::new(name.as_slice()), &b),
        None => {}
    }
    match files {
        [] | [_] => {
            show_error!("missing file operand\n\
                        Try 'mv --help' for more information.");
            return 1;
        },
        [ref source, ref target] => {
            if !source.exists() {
                show_error!("cannot stat ‘{}’: No such file or directory", source.display());
                return 1;
            }

            if target.is_dir() {
                if b.no_target_dir {
                    if !source.is_dir() {
                        show_error!("cannot overwrite directory ‘{}’ with non-directory",
                            target.display());
                        return 1;
                    }

                    return match rename(source, target, &b) {
                        Err(e) => {
                            show_error!("{}", e);
                            1
                        },
                        _ => 0
                    }
                }

                return move_files_into_dir(&[source.clone()], target, &b);
            }

            match rename(source, target, &b) {
                Err(e) => {
                    show_error!("{}", e);
                    return 1;
                },
                _ => {}
            }
        }
        fs => {
            if b.no_target_dir {
                show_error!("mv: extra operand ‘{}’\n\
                            Try 'mv --help' for more information.", fs[2].display());
                return 1;
            }
            let target_dir = fs.last().unwrap();
            move_files_into_dir(fs.init(), target_dir, &b);
        }
    }
    0
}

fn move_files_into_dir(files: &[Path], target_dir: &Path, b: &Behaviour) -> isize {
    if !target_dir.is_dir() {
        show_error!("target ‘{}’ is not a directory", target_dir.display());
        return 1;
    }

    let mut all_successful = true;
    for sourcepath in files.iter() {
        let targetpath = match sourcepath.filename_str() {
            Some(name) => target_dir.join(name),
            None => {
                show_error!("cannot stat ‘{}’: No such file or directory",
                            sourcepath.display());

                all_successful = false;
                continue;
            }
        };

        match rename(sourcepath, &targetpath, b) {
            Err(e) => {
                show_error!("mv: cannot move ‘{}’ to ‘{}’: {}",
                            sourcepath.display(), targetpath.display(), e);
                all_successful = false;
            },
            _ => {}
        }
    };
    if all_successful { 0 } else { 1 }
}

fn rename(from: &Path, to: &Path, b: &Behaviour) -> IoResult<()> {
    let mut backup_path = None;

    if to.exists() {
        match b.overwrite {
            OverwriteMode::NoClobber => return Ok(()),
            OverwriteMode::Interactive => {
                print!("{}: overwrite ‘{}’? ", NAME, to.display());
                if !read_yes() {
                    return Ok(());
                }
            },
            OverwriteMode::Force => {}
        };

        backup_path = match b.backup {
            BackupMode::NoBackup => None,
            BackupMode::SimpleBackup => Some(simple_backup_path(to, &b.suffix)),
            BackupMode::NumberedBackup => Some(numbered_backup_path(to)),
            BackupMode::ExistingBackup => Some(existing_backup_path(to, &b.suffix))
        };
        if let Some(ref p) = backup_path {
            try!(fs::rename(to, p));
        }

        if b.update {
            if try!(from.stat()).modified <= try!(to.stat()).modified {
                return Ok(());
            }
        }
    }

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

fn read_yes() -> bool {
    match BufferedReader::new(stdin_raw()).read_line() {
        Ok(s) => match s.as_slice().slice_shift_char() {
            Some((x, _)) => x == 'y' || x == 'Y',
            _ => false
        },
        _ => false
    }
}

fn simple_backup_path(path: &Path, suffix: &String) -> Path {
    let mut p = path.clone().into_vec();
    p.push_all(suffix.as_slice().as_bytes());
    return Path::new(p);
}

fn numbered_backup_path(path: &Path) -> Path {
    let mut i: u64 = 1;
    loop {
        let new_path = simple_backup_path(path, &format!(".~{}~", i));
        if !new_path.exists() {
            return new_path;
        }
        i = i + 1;
    }
}

fn existing_backup_path(path: &Path, suffix: &String) -> Path {
    let test_path = simple_backup_path(path, &".~1~".to_owned());
    if test_path.exists() {
        return numbered_backup_path(path);
    }
    return simple_backup_path(path, suffix);
}
