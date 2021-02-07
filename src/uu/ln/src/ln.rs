//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Joseph Crail <jbcrail@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) srcpath targetpath EEXIST

#[macro_use]
extern crate uucore;

use clap::{App, Arg};

use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs;

use std::io::{stdin, Result};
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::path::{Path, PathBuf};
use uucore::fs::{canonicalize, CanonicalizeMode};

pub struct Settings {
    overwrite: OverwriteMode,
    backup: BackupMode,
    force: bool,
    suffix: String,
    symbolic: bool,
    relative: bool,
    target_dir: Option<String>,
    no_target_dir: bool,
    no_dereference: bool,
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

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... [-T] TARGET LINK_executable!()   (1st form)
       {0} [OPTION]... TARGET                  (2nd form)
       {0} [OPTION]... TARGET... DIRECTORY     (3rd form)
       {0} [OPTION]... -t DIRECTORY TARGET...  (4th form)",
        executable!()
    )
}

fn get_long_usage() -> String {
    String::from(
        " In the 1st form, create a link to TARGET with the name LINK_executable!().
        In the 2nd form, create a link to TARGET in the current directory.
        In the 3rd and 4th forms, create links to each TARGET in DIRECTORY.
        Create hard links by default, symbolic links with --symbolic.
        By default, each destination (name of new link) should not already exist.
        When creating hard links, each TARGET must exist.  Symbolic links
        can hold arbitrary text; if later resolved, a relative link is
        interpreted in relation to its parent directory.
        ",
    )
}

static ABOUT: &str = "change file owner and group";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static OPT_B: &str = "b";
static OPT_BACKUP: &str = "backup";
static OPT_FORCE: &str = "force";
static OPT_INTERACTIVE: &str = "interactive";
static OPT_NO_DEREFERENCE: &str = "no-dereference";
static OPT_SYMBOLIC: &str = "symbolic";
static OPT_SUFFIX: &str = "suffix";
static OPT_TARGET_DIRECTORY: &str = "target-directory";
static OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
static OPT_RELATIVE: &str = "relative";
static OPT_VERBOSE: &str = "verbose";

static ARG_FILES: &str = "files";

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let long_usage = get_long_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(&long_usage[..])
        .arg(Arg::with_name(OPT_B).short(OPT_B).help(
            "make a backup of each file that would otherwise be overwritten or \
             removed",
        ))
        .arg(
            Arg::with_name(OPT_BACKUP)
                .long(OPT_BACKUP)
                .help(
                    "make a backup of each file that would otherwise be overwritten \
             or removed",
                )
                .takes_value(true)
                .possible_value("simple")
                .possible_value("never")
                .possible_value("numbered")
                .possible_value("t")
                .possible_value("existing")
                .possible_value("nil")
                .possible_value("none")
                .possible_value("off")
                .value_name("METHOD"),
        )
        // TODO: opts.arg(
        //    Arg::with_name(("d", "directory", "allow users with appropriate privileges to attempt \
        //                                       to make hard links to directories");
        .arg(
            Arg::with_name(OPT_FORCE)
                .short("f")
                .long(OPT_FORCE)
                .help("remove existing destination files"),
        )
        .arg(
            Arg::with_name(OPT_INTERACTIVE)
                .short("i")
                .long(OPT_INTERACTIVE)
                .help("prompt whether to remove existing destination files"),
        )
        .arg(
            Arg::with_name(OPT_NO_DEREFERENCE)
                .short("n")
                .long(OPT_NO_DEREFERENCE)
                .help(
                    "treat LINK_executable!() as a normal file if it is a \
                                                    symbolic link to a directory",
                ),
        )
        // TODO: opts.arg(
        //    Arg::with_name(("L", "logical", "dereference TARGETs that are symbolic links");
        //
        // TODO: opts.arg(
        //    Arg::with_name(("P", "physical", "make hard links directly to symbolic links");
        .arg(
            Arg::with_name(OPT_SYMBOLIC)
                .short("s")
                .long("symbolic")
                .help("make symbolic links instead of hard links"),
        )
        .arg(
            Arg::with_name(OPT_SUFFIX)
                .short("S")
                .long(OPT_SUFFIX)
                .help("override the usual backup suffix")
                .value_name("SUFFIX")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(OPT_TARGET_DIRECTORY)
                .short("t")
                .long(OPT_TARGET_DIRECTORY)
                .help("specify the DIRECTORY in which to create the links")
                .value_name("DIRECTORY")
                .conflicts_with(OPT_NO_TARGET_DIRECTORY),
        )
        .arg(
            Arg::with_name(OPT_NO_TARGET_DIRECTORY)
                .short("T")
                .long(OPT_NO_TARGET_DIRECTORY)
                .help("treat LINK_executable!() as a normal file always"),
        )
        .arg(
            Arg::with_name(OPT_RELATIVE)
                .short("r")
                .long(OPT_RELATIVE)
                .help("create symbolic links relative to link location"),
        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
                .short("v")
                .long(OPT_VERBOSE)
                .help("print name of each linked file"),
        )
        .arg(
            Arg::with_name(ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .required(true)
                .min_values(1),
        )
        .get_matches_from(args);

    /* the list of files */

    let paths: Vec<PathBuf> = matches
        .values_of(ARG_FILES)
        .unwrap()
        .map(|path| PathBuf::from(path))
        .collect();

    let overwrite_mode = if matches.is_present(OPT_FORCE) {
        OverwriteMode::Force
    } else if matches.is_present(OPT_INTERACTIVE) {
        OverwriteMode::Interactive
    } else {
        OverwriteMode::NoClobber
    };

    let backup_mode = if matches.is_present(OPT_B) {
        BackupMode::ExistingBackup
    } else if matches.is_present(OPT_BACKUP) {
        match matches.value_of(OPT_BACKUP) {
            None => BackupMode::ExistingBackup,
            Some(mode) => match &mode[..] {
                "simple" | "never" => BackupMode::SimpleBackup,
                "numbered" | "t" => BackupMode::NumberedBackup,
                "existing" | "nil" => BackupMode::ExistingBackup,
                "none" | "off" => BackupMode::NoBackup,
                _ => panic!(), // cannot happen as it is managed by clap
            },
        }
    } else {
        BackupMode::NoBackup
    };

    let backup_suffix = if matches.is_present(OPT_SUFFIX) {
        matches.value_of(OPT_SUFFIX).unwrap()
    } else {
        "~"
    };

    let settings = Settings {
        overwrite: overwrite_mode,
        backup: backup_mode,
        force: matches.is_present(OPT_FORCE),
        suffix: backup_suffix.to_string(),
        symbolic: matches.is_present(OPT_SYMBOLIC),
        relative: matches.is_present(OPT_RELATIVE),
        target_dir: matches.value_of(OPT_TARGET_DIRECTORY).map(String::from),
        no_target_dir: matches.is_present(OPT_NO_TARGET_DIRECTORY),
        no_dereference: matches.is_present(OPT_NO_DEREFERENCE),
        verbose: matches.is_present(OPT_VERBOSE),
    };

    exec(&paths[..], &settings)
}

fn exec(files: &[PathBuf], settings: &Settings) -> i32 {
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
            return link_files_in_dir(&files[0..files.len() - 1], last_file, &settings);
        }
    }

    // 1st form. Now there should be only two operands, but if -T is
    // specified we may have a wrong number of operands.
    if files.len() == 1 {
        show_error!(
            "missing destination file operand after '{}'",
            files[0].to_string_lossy()
        );
        return 1;
    }
    if files.len() > 2 {
        show_error!(
            "extra operand '{}'\nTry '{} --help' for more information.",
            files[2].display(),
            executable!()
        );
        return 1;
    }
    assert!(!files.is_empty());

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
        let targetpath = if settings.no_dereference && settings.force {
            // In that case, we don't want to do link resolution
            // We need to clean the target
            if is_symlink(target_dir) {
                if target_dir.is_file() {
                    match fs::remove_file(target_dir) {
                        Err(e) => show_error!("Could not update {}: {}", target_dir.display(), e),
                        _ => (),
                    };
                }
                if target_dir.is_dir() {
                    // Not sure why but on Windows, the symlink can be
                    // considered as a dir
                    // See test_ln::test_symlink_no_deref_dir
                    if let Err(e) = fs::remove_dir(target_dir) {
                        show_error!("Could not update {}: {}", target_dir.display(), e)
                    };
                }
            }
            target_dir.clone()
        } else {
            match srcpath.as_os_str().to_str() {
                Some(name) => {
                    match Path::new(name).file_name() {
                        Some(basename) => target_dir.join(basename),
                        // This can be None only for "." or "..". Trying
                        // to create a link with such name will fail with
                        // EEXIST, which agrees with the behavior of GNU
                        // coreutils.
                        None => target_dir.join(name),
                    }
                }
                None => {
                    show_error!(
                        "cannot stat '{}': No such file or directory",
                        srcpath.display()
                    );
                    all_successful = false;
                    continue;
                }
            }
        };

        if let Err(e) = link(srcpath, &targetpath, settings) {
            show_error!(
                "cannot link '{}' to '{}': {}",
                targetpath.display(),
                srcpath.display(),
                e
            );
            all_successful = false;
        }
    }
    if all_successful {
        0
    } else {
        1
    }
}

fn relative_path<'a>(src: &PathBuf, dst: &PathBuf) -> Result<Cow<'a, Path>> {
    let abssrc = canonicalize(src, CanonicalizeMode::Normal)?;
    let absdst = canonicalize(dst, CanonicalizeMode::Normal)?;
    let suffix_pos = abssrc
        .components()
        .zip(absdst.components())
        .take_while(|(s, d)| s == d)
        .count();

    let srciter = abssrc.components().skip(suffix_pos).map(|x| x.as_os_str());

    let result: PathBuf = absdst
        .components()
        .skip(suffix_pos + 1)
        .map(|_| OsStr::new(".."))
        .chain(srciter)
        .collect();
    Ok(result.into())
}

fn link(src: &PathBuf, dst: &PathBuf, settings: &Settings) -> Result<()> {
    let mut backup_path = None;
    let source: Cow<'_, Path> = if settings.relative {
        relative_path(&src, dst)?
    } else {
        src.into()
    };

    if is_symlink(dst) || dst.exists() {
        match settings.overwrite {
            OverwriteMode::NoClobber => {}
            OverwriteMode::Interactive => {
                print!("{}: overwrite '{}'? ", executable!(), dst.display());
                if !read_yes() {
                    return Ok(());
                }
                fs::remove_file(dst)?
            }
            OverwriteMode::Force => fs::remove_file(dst)?,
        };

        backup_path = match settings.backup {
            BackupMode::NoBackup => None,
            BackupMode::SimpleBackup => Some(simple_backup_path(dst, &settings.suffix)),
            BackupMode::NumberedBackup => Some(numbered_backup_path(dst)),
            BackupMode::ExistingBackup => Some(existing_backup_path(dst, &settings.suffix)),
        };
        if let Some(ref p) = backup_path {
            fs::rename(dst, p)?;
        }
    }

    if settings.no_dereference && settings.force {
        if dst.exists() {
            fs::remove_file(dst)?;
        }
    }

    if settings.symbolic {
        symlink(&source, dst)?;
    } else {
        fs::hard_link(&source, dst)?;
    }

    if settings.verbose {
        print!("'{}' -> '{}'", dst.display(), &source.display());
        match backup_path {
            Some(path) => println!(" (backup: '{}')", path.display()),
            None => println!(),
        }
    }
    Ok(())
}

fn read_yes() -> bool {
    let mut s = String::new();
    match stdin().read_line(&mut s) {
        Ok(_) => match s.char_indices().next() {
            Some((_, x)) => x == 'y' || x == 'Y',
            _ => false,
        },
        _ => false,
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
pub fn symlink<P1: AsRef<Path>, P2: AsRef<Path>>(src: P1, dst: P2) -> Result<()> {
    if src.as_ref().is_dir() {
        symlink_dir(src, dst)
    } else {
        symlink_file(src, dst)
    }
}

pub fn is_symlink<P: AsRef<Path>>(path: P) -> bool {
    match fs::symlink_metadata(path) {
        Ok(m) => m.file_type().is_symlink(),
        Err(_) => false,
    }
}
