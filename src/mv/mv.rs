#![crate_name = "uu_mv"]

// This file is part of the uutils coreutils package.
//
// (c) Orvar Segerström <orvarsegerstrom@gmail.com>
// (c) Sokovikov Evgeniy  <skv-headless@yandex.ru>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::fs;
use std::env;
use std::io::{BufRead, BufReader, Result, stdin, Write};
use std::path::{Path, PathBuf};

static NAME: &'static str = "mv";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct Behaviour {
    overwrite: OverwriteMode,
    backup: BackupMode,
    suffix: String,
    update: bool,
    target_dir: Option<String>,
    no_target_dir: bool,
    verbose: bool,
}

#[derive(Clone, Eq, PartialEq)]
pub enum OverwriteMode {
    NoClobber,
    Interactive,
    Force,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BackupMode {
    NoBackup,
    SimpleBackup,
    NumberedBackup,
    ExistingBackup,
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflagopt("",
                    "backup",
                    "make a backup of each existing destination file",
                    "CONTROL");
    opts.optflag("b", "", "like --backup but does not accept an argument");
    opts.optflag("f", "force", "do not prompt before overwriting");
    opts.optflag("i", "interactive", "prompt before override");
    opts.optflag("n", "no-clobber", "do not overwrite an existing file");
    opts.optflag("",
                 "strip-trailing-slashes",
                 "remove any trailing slashes from each SOURCE\n \
                                                 argument");
    opts.optopt("S", "suffix", "override the usual backup suffix", "SUFFIX");
    opts.optopt("t",
                "target-directory",
                "move all SOURCE arguments into DIRECTORY",
                "DIRECTORY");
    opts.optflag("T", "no-target-directory", "treat DEST as a normal file");
    opts.optflag("u",
                 "update",
                 "move only when the SOURCE file is newer\n \
                                than the destination file or when the\n \
                                destination file is missing");
    opts.optflag("v", "verbose", "explain what is being done");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            show_error!("Invalid options\n{}", f);
            return 1;
        }
    };
    let usage = opts.usage("Move SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.");

    let overwrite_mode = determine_overwrite_mode(&matches);
    let backup_mode = determine_backup_mode(&matches);

    if overwrite_mode == OverwriteMode::NoClobber && backup_mode != BackupMode::NoBackup {
        show_error!("options --backup and --no-clobber are mutually exclusive\n\
                    Try '{} --help' for more information.",
                    NAME);
        return 1;
    }

    let backup_suffix = determine_backup_suffix(backup_mode, &matches);

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

    let paths: Vec<PathBuf> = {
        fn strip_slashes<'a>(p: &'a Path) -> &'a Path {
            p.components().as_path()
        }
        let to_owned = |p: &Path| p.to_owned();
        let arguments = matches.free.iter().map(Path::new);
        if matches.opt_present("strip-trailing-slashes") {
            arguments.map(strip_slashes).map(to_owned).collect()
        } else {
            arguments.map(to_owned).collect()
        }
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        0
    } else if matches.opt_present("help") {
        help(&usage);
        0
    } else {
        exec(&paths[..], behaviour)
    }
}

fn determine_overwrite_mode(matches: &getopts::Matches) -> OverwriteMode {
    // This does not exactly match the GNU implementation:
    // The GNU mv defaults to Force, but if more than one of the
    // overwrite options are supplied, only the last takes effect.
    // To default to no-clobber in that situation seems safer:
    //
    if matches.opt_present("no-clobber") {
        OverwriteMode::NoClobber
    } else if matches.opt_present("interactive") {
        OverwriteMode::Interactive
    } else {
        OverwriteMode::Force
    }
}

fn determine_backup_mode(matches: &getopts::Matches) -> BackupMode {
    if matches.opt_present("b") {
        BackupMode::SimpleBackup
    } else if matches.opt_present("backup") {
        match matches.opt_str("backup") {
            None => BackupMode::SimpleBackup,
            Some(mode) => {
                match &mode[..] {
                    "simple" | "never" => BackupMode::SimpleBackup,
                    "numbered" | "t" => BackupMode::NumberedBackup,
                    "existing" | "nil" => BackupMode::ExistingBackup,
                    "none" | "off" => BackupMode::NoBackup,
                    x => {
                        crash!(1, "invalid argument ‘{}’ for ‘backup type’\n\
                                Try '{} --help' for more information.",
                                    x,
                                    NAME);
                    }
                }
            }
        }
    } else {
        BackupMode::NoBackup
    }
}

fn determine_backup_suffix(backup_mode: BackupMode, matches: &getopts::Matches) -> String {
    if matches.opt_present("suffix") {
        match matches.opt_str("suffix") {
            Some(x) => x,
            None => {
                crash!(1, "option '--suffix' requires an argument\n\
                            Try '{} --help' for more information.",
                            NAME);
            }
        }
    } else {
        if let (Ok(s), BackupMode::SimpleBackup) = (env::var("SIMPLE_BACKUP_SUFFIX"), backup_mode) {
            s
        } else {
            "~".to_owned()
        }
    }
}

fn help(usage: &str) {
    println!("{0} {1}\n\n\
    Usage: {0} SOURCE DEST\n   \
       or: {0} SOURCE... DIRECTORY\n\n\
    {2}",
             NAME,
             VERSION,
             usage);
}

fn exec(files: &[PathBuf], b: Behaviour) -> i32 {
    if let Some(ref name) = b.target_dir {
        return move_files_into_dir(files, &PathBuf::from(name), &b);
    }
    match files.len() {
        0 | 1 => {
            show_error!("missing file operand\n\
                        Try '{} --help' for more information.",
                        NAME);
            return 1;
        }
        2 => {
            let source = &files[0];
            let target = &files[1];
            if !source.exists() {
                show_error!("cannot stat ‘{}’: No such file or directory",
                            source.display());
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
                        }
                        _ => 0,
                    };
                }

                return move_files_into_dir(&[source.clone()], target, &b);
            }

            if let Err(e) = rename(source, target, &b) {
                show_error!("{}", e);
                return 1;
            }
        }
        _ => {
            if b.no_target_dir {
                show_error!("mv: extra operand ‘{}’\n\
                            Try '{} --help' for more information.",
                            files[2].display(),
                            NAME);
                return 1;
            }
            let target_dir = files.last().unwrap();
            move_files_into_dir(&files[..files.len() - 1], target_dir, &b);
        }
    }
    0
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
                        sourcepath.display(),
                        targetpath.display(),
                        e);
            all_successful = false;
        }
    }
    if all_successful {
        0
    } else {
        1
    }
}

fn rename(from: &PathBuf, to: &PathBuf, b: &Behaviour) -> Result<()> {
    let mut backup_path = None;

    if to.exists() {
        match b.overwrite {
            OverwriteMode::NoClobber => return Ok(()),
            OverwriteMode::Interactive => {
                print!("{}: overwrite ‘{}’? ", NAME, to.display());
                if !read_yes() {
                    return Ok(());
                }
            }
            OverwriteMode::Force => {}
        };

        backup_path = match b.backup {
            BackupMode::NoBackup => None,
            BackupMode::SimpleBackup => Some(simple_backup_path(to, &b.suffix)),
            BackupMode::NumberedBackup => Some(numbered_backup_path(to)),
            BackupMode::ExistingBackup => Some(existing_backup_path(to, &b.suffix)),
        };
        if let Some(ref p) = backup_path {
            try!(fs::rename(to, p));
        }

        if b.update {
            if try!(try!(fs::metadata(from)).modified()) <= try!(try!(fs::metadata(to)).modified()) {
                return Ok(());
            }
        }
    }

    try!(fs::rename(from, to));

    if b.verbose {
        print!("‘{}’ -> ‘{}’", from.display(), to.display());
        match backup_path {
            Some(path) => println!(" (backup: ‘{}’)", path.display()),
            None => println!(""),
        }
    }
    Ok(())
}

fn read_yes() -> bool {
    let mut s = String::new();
    match BufReader::new(stdin()).read_line(&mut s) {
        Ok(_) => {
            match s.chars().nth(0) {
                Some(x) => x == 'y' || x == 'Y',
                _ => false,
            }
        }
        _ => false,
    }
}

fn simple_backup_path(path: &PathBuf, suffix: &str) -> PathBuf {
    let mut p = path.to_string_lossy().into_owned();
    p.push_str(suffix);
    PathBuf::from(p)
}

fn numbered_backup_path(path: &PathBuf) -> PathBuf {
    (1_u64..)
        .map(|i| path.with_extension(format!("~{}~", i)))
        .skip_while(|p| p.exists())
        .next()
        .expect("cannot create backup")
}

fn existing_backup_path(path: &PathBuf, suffix: &str) -> PathBuf {
    let test_path = path.with_extension("~1~");
    if test_path.exists() {
        numbered_backup_path(path)
    } else {
        simple_backup_path(path, suffix)
    }
}
