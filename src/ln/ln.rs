#![crate_name = "uu_ln"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Joseph Crail <jbcrail@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::fs;
use std::io::{BufRead, BufReader, Result, stdin, Write};
#[cfg(unix)] use std::os::unix::fs::symlink as symlink_file;
#[cfg(windows)] use std::os::windows::fs::symlink_file;
use std::path::{Path, PathBuf};

static NAME: &'static str = "ln";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct Settings {
    overwrite: OverwriteMode,
    backup: BackupMode,
    suffix: String,
    symbolic: bool,
    target_dir: Option<String>,
    no_target_dir: bool,
    verbose: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OverwriteMode {
    NoClobber,
    Interactive,
    Force,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupMode {
    NoBackup,
    SimpleBackup,
    NumberedBackup,
    ExistingBackup,
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("b", "", "make a backup of each file that would otherwise be overwritten or removed");
    opts.optflagopt("", "backup", "make a backup of each file that would otherwise be overwritten or removed", "METHOD");
    // TODO: opts.optflag("d", "directory", "allow users with appropriate privileges to attempt to make hard links to directories");
    opts.optflag("f", "force", "remove existing destination files");
    opts.optflag("i", "interactive", "prompt whether to remove existing destination files");
    // TODO: opts.optflag("L", "logical", "dereference TARGETs that are symbolic links");
    // TODO: opts.optflag("n", "no-dereference", "treat LINK_NAME as a normal file if it is a symbolic link to a directory");
    // TODO: opts.optflag("P", "physical", "make hard links directly to symbolic links");
    // TODO: opts.optflag("r", "relative", "create symbolic links relative to link location");
    opts.optflag("s", "symbolic", "make symbolic links instead of hard links");
    opts.optopt("S", "suffix", "override the usual backup suffix", "SUFFIX");
    opts.optopt("t", "target-directory", "specify the DIRECTORY in which to create the links", "DIRECTORY");
    opts.optflag("T", "no-target-directory", "treat LINK_NAME as a normal file always");
    opts.optflag("v", "verbose", "print name of each linked file");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => crash!(1, "{}", e),
    };

    let overwrite_mode = if matches.opt_present("force") {
        OverwriteMode::Force
    } else if matches.opt_present("interactive") {
        OverwriteMode::Interactive
    } else {
        OverwriteMode::NoClobber
    };

    let backup_mode = if matches.opt_present("b") {
        BackupMode::ExistingBackup
    } else if matches.opt_present("backup") {
        match matches.opt_str("backup") {
            None => BackupMode::ExistingBackup,
            Some(mode) => match &mode[..] {
                "simple" | "never" => BackupMode::SimpleBackup,
                "numbered" | "t"   => BackupMode::NumberedBackup,
                "existing" | "nil" => BackupMode::ExistingBackup,
                "none" | "off"     => BackupMode::NoBackup,
                x => {
                    show_error!("invalid argument '{}' for 'backup method'\n\
                                Try '{} --help' for more information.", x, NAME);
                    return 1;
                }
            }
        }
    } else {
        BackupMode::NoBackup
    };

    let backup_suffix = if matches.opt_present("suffix") {
        match matches.opt_str("suffix") {
            Some(x) => x,
            None => {
                show_error!("option '--suffix' requires an argument\n\
                            Try '{} --help' for more information.", NAME);
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

    let settings = Settings {
        overwrite: overwrite_mode,
        backup: backup_mode,
        suffix: backup_suffix,
        symbolic: matches.opt_present("s"),
        target_dir: matches.opt_str("t"),
        no_target_dir: matches.opt_present("T"),
        verbose: matches.opt_present("v"),
    };

    let string_to_path = |s: &String| { PathBuf::from(s) };
    let paths: Vec<PathBuf> = matches.free.iter().map(string_to_path).collect();

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        0 
    } else if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage: {0} [OPTION]... [-T] TARGET LINK_NAME   (1st form)
   or: {0} [OPTION]... TARGET                  (2nd form)
   or: {0} [OPTION]... TARGET... DIRECTORY     (3rd form)
   or: {0} [OPTION]... -t DIRECTORY TARGET...  (4th form)

In the 1st form, create a link to TARGET with the name LINK_NAME.
In the 2nd form, create a link to TARGET in the current directory.
In the 3rd and 4th forms, create links to each TARGET in DIRECTORY.
Create hard links by default, symbolic links with --symbolic.
By default, each destination (name of new link) should not already exist.
When creating hard links, each TARGET must exist.  Symbolic links
can hold arbitrary text; if later resolved, a relative link is
interpreted in relation to its parent directory.", NAME, VERSION);

        print!("{}", opts.usage(&msg));
        0
    } else {
        exec(&paths[..], &settings)
    }
}

fn exec(files: &[PathBuf], settings: &Settings) -> i32 {
    match settings.target_dir {
        Some(ref name) => return link_files_in_dir(files, &PathBuf::from(name), &settings),
        None => {}
    }
    match files.len() {
        0 => {
            show_error!("missing file operand\nTry '{} --help' for more information.", NAME);
            1
        },
        1 => match link(&files[0], &files[0], settings) {
            Ok(_) => 0,
            Err(e) => {
                show_error!("{}", e);
                1
            }
        },
        2 => match link(&files[0], &files[1], settings) {
            Ok(_) => 0,
            Err(e) => {
                show_error!("{}", e);
                1
            }
        },
        _ => {
            if settings.no_target_dir {
                show_error!("extra operand '{}'\nTry '{} --help' for more information.", files[2].display(), NAME);
                return 1;
            }
            let (targets, dir) = match settings.target_dir {
                Some(ref dir) => (files, PathBuf::from(dir.clone())),
                None => (&files[0..files.len()-1], files[files.len()-1].clone())
            };
            link_files_in_dir(targets, &dir, settings)
        }
    }
}

fn link_files_in_dir(files: &[PathBuf], target_dir: &PathBuf, settings: &Settings) -> i32 {
    if !target_dir.is_dir() {
        show_error!("target '{}' is not a directory", target_dir.display());
        return 1;
    }

    let mut all_successful = true;
    for srcpath in files.iter() {
        let targetpath = match srcpath.as_os_str().to_str() {
            Some(name) => target_dir.join(name),
            None => {
                show_error!("cannot stat '{}': No such file or directory",
                            srcpath.display());
                all_successful = false;
                continue;
            }
        };

        if let Err(e) = link(srcpath, &targetpath, settings) {
            show_error!("cannot link '{}' to '{}': {}",
                        targetpath.display(), srcpath.display(), e);
            all_successful = false;
        }
    }
    if all_successful { 0 } else { 1 }
}

fn link(src: &PathBuf, dst: &PathBuf, settings: &Settings) -> Result<()> {
    let mut backup_path = None;

    if dst.is_dir() && settings.no_target_dir {
        try!(fs::remove_dir(dst));
    }

    if is_symlink(dst) || dst.exists() {
        match settings.overwrite {
            OverwriteMode::NoClobber => {},
            OverwriteMode::Interactive => {
                print!("{}: overwrite '{}'? ", NAME, dst.display());
                if !read_yes() {
                    return Ok(());
                }
                try!(fs::remove_file(dst))
            },
            OverwriteMode::Force => {
                try!(fs::remove_file(dst))
            }
        };

        backup_path = match settings.backup {
            BackupMode::NoBackup => None,
            BackupMode::SimpleBackup => Some(simple_backup_path(dst, &settings.suffix)),
            BackupMode::NumberedBackup => Some(numbered_backup_path(dst)),
            BackupMode::ExistingBackup => Some(existing_backup_path(dst, &settings.suffix))
        };
        if let Some(ref p) = backup_path {
            try!(fs::rename(dst, p));
        }
    }

    if settings.symbolic {
        try!(symlink(src, dst));
    } else {
        try!(fs::hard_link(src, dst));
    }

    if settings.verbose {
        print!("'{}' -> '{}'", dst.display(), src.display());
        match backup_path {
            Some(path) => println!(" (backup: '{}')", path.display()),
            None => println!("")
        }
    }
    Ok(())
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

fn simple_backup_path(path: &PathBuf, suffix: &str) -> PathBuf {
    let mut p = path.as_os_str().to_str().unwrap().to_owned();
    p.push_str(suffix);
    PathBuf::from(p)
}

fn numbered_backup_path(path: &PathBuf) -> PathBuf {
    let mut i: u64 = 1;
    loop {
        let new_path = simple_backup_path(path, &format!(".~{}~", i));
        if !new_path.exists() {
            return new_path;
        }
        i += 1;
    }
}

fn existing_backup_path(path: &PathBuf, suffix: &str) -> PathBuf {
    let test_path = simple_backup_path(path, &".~1~".to_owned());
    if test_path.exists() {
        return numbered_backup_path(path);
    }
    simple_backup_path(path, suffix)
}

pub fn symlink<P: AsRef<Path>>(src: P, dst: P) -> Result<()> {
    symlink_file(src, dst)
}

pub fn is_symlink<P: AsRef<Path>>(path: P) -> bool {
    match fs::symlink_metadata(path) {
        Ok(m) => m.file_type().is_symlink(),
        Err(_) => false
    }
}
