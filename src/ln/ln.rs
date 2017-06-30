#![crate_name = "uu_ln"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Joseph Crail <jbcrail@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */


#[macro_use]
extern crate uucore;

use std::fs;
use std::io::{BufRead, BufReader, Result, stdin, Write};
#[cfg(unix)] use std::os::unix::fs::symlink;
#[cfg(windows)] use std::os::windows::fs::{symlink_file,symlink_dir};
use std::path::{Path, PathBuf};

static NAME: &'static str = "ln"; 
static SUMMARY: &'static str = ""; 
static LONG_HELP: &'static str = "
 In the 1st form, create a link to TARGET with the name LINK_NAME.
 In the 2nd form, create a link to TARGET in the current directory.
 In the 3rd and 4th forms, create links to each TARGET in DIRECTORY.
 Create hard links by default, symbolic links with --symbolic.
 By default, each destination (name of new link) should not already exist.
 When creating hard links, each TARGET must exist.  Symbolic links
 can hold arbitrary text; if later resolved, a relative link is
 interpreted in relation to its parent directory.
"; 

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
    let syntax = format!("[OPTION]... [-T] TARGET LINK_NAME   (1st form)
 {0} [OPTION]... TARGET                  (2nd form)
 {0} [OPTION]... TARGET... DIRECTORY     (3rd form)
 {0} [OPTION]... -t DIRECTORY TARGET...  (4th form)", NAME);
    let matches = new_coreopts!(&syntax, SUMMARY, LONG_HELP)
        .optflag("b", "", "make a backup of each file that would otherwise be overwritten or removed")
        .optflagopt("", "backup", "make a backup of each file that would otherwise be overwritten or removed", "METHOD")
    // TODO: opts.optflag("d", "directory", "allow users with appropriate privileges to attempt to make hard links to directories");
        .optflag("f", "force", "remove existing destination files")
        .optflag("i", "interactive", "prompt whether to remove existing destination files")
    // TODO: opts.optflag("L", "logical", "dereference TARGETs that are symbolic links");
    // TODO: opts.optflag("n", "no-dereference", "treat LINK_NAME as a normal file if it is a symbolic link to a directory");
    // TODO: opts.optflag("P", "physical", "make hard links directly to symbolic links");
    // TODO: opts.optflag("r", "relative", "create symbolic links relative to link location");
        .optflag("s", "symbolic", "make symbolic links instead of hard links")
        .optopt("S", "suffix", "override the usual backup suffix", "SUFFIX")
        .optopt("t", "target-directory", "specify the DIRECTORY in which to create the links", "DIRECTORY")
        .optflag("T", "no-target-directory", "treat LINK_NAME as a normal file always")
        .optflag("v", "verbose", "print name of each linked file")
        .parse(args);

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

    exec(&paths[..], &settings)
}

fn exec(files: &[PathBuf], settings: &Settings) -> i32 {
    if files.len() == 0 {
        show_error!("missing file operand\nTry '{} --help' for more information.", NAME);
        return 1;
    }

    // Handle cases where we create links in a directory first.
    if let Some(ref name) = settings.target_dir {
        // 4th form: a directory is specified by -t.
        return link_files_in_dir(files, &PathBuf::from(name), &settings);
    }
    if !settings.no_target_dir {
        if files.len() == 1 {
            // 2nd form: the target directory is the current directory.
            return link_files_in_dir(files, &PathBuf::from("."), &settings);
        }
        let last_file = &PathBuf::from(files.last().unwrap());
        if files.len() > 2 || last_file.is_dir() {
            // 3rd form: create links in the last argument.
            return link_files_in_dir(&files[0..files.len()-1], last_file, &settings);
        }
    }

    // 1st form. Now there should be only two operands, but if -T is
    // specified we may have a wrong number of operands.
    if files.len() == 1 {
        show_error!("missing destination file operand after '{}'", files[0].to_string_lossy());
        return 1;
    }
    if files.len() > 2 {
        show_error!("extra operand '{}'\nTry '{} --help' for more information.", files[2].display(), NAME);
        return 1;
    }
    assert!(files.len() != 0);

    match link(&files[0], &files[1], settings) {
        Ok(_) => 0,
        Err(e) => {
            show_error!("{}", e);
            1
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
            Some(name) => {
                match Path::new(name).file_name() {
                    Some(basename) => target_dir.join(basename),
                    // This can be None only for "." or "..". Trying
                    // to create a link with such name will fail with
                    // EEXIST, which agrees with the bahavior of GNU
                    // coreutils.
                    None => target_dir.join(name),
                }
            }
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

#[cfg(windows)]
pub fn symlink<P: AsRef<Path>>(src: P, dst: P) -> Result<()> {
    if src.as_ref().is_dir()
    {
        symlink_dir(src,dst)
    }
    else
    {
        symlink_file(src,dst)
    }
}

pub fn is_symlink<P: AsRef<Path>>(path: P) -> bool {
    match fs::symlink_metadata(path) {
        Ok(m) => m.file_type().is_symlink(),
        Err(_) => false
    }
}
