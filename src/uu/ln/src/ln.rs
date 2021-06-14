//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Joseph Crail <jbcrail@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) srcpath targetpath EEXIST

#[macro_use]
extern crate uucore;

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

use crate::app::{get_app, options, ARG_FILES};

pub mod app;

pub struct Settings {
    overwrite: OverwriteMode,
    backup: BackupMode,
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
        "{0} [OPTION]... [-T] TARGET LINK_NAME   (1st form)
       {0} [OPTION]... TARGET                  (2nd form)
       {0} [OPTION]... TARGET... DIRECTORY     (3rd form)
       {0} [OPTION]... -t DIRECTORY TARGET...  (4th form)",
        executable!()
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    /* the list of files */

    let paths: Vec<PathBuf> = matches
        .values_of(ARG_FILES)
        .unwrap()
        .map(PathBuf::from)
        .collect();

    let overwrite_mode = if matches.is_present(options::FORCE) {
        OverwriteMode::Force
    } else if matches.is_present(options::INTERACTIVE) {
        OverwriteMode::Interactive
    } else {
        OverwriteMode::NoClobber
    };

    let backup_mode = if matches.is_present(options::B) {
        BackupMode::ExistingBackup
    } else if matches.is_present(options::BACKUP) {
        match matches.value_of(options::BACKUP) {
            None => BackupMode::ExistingBackup,
            Some(mode) => match mode {
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

    let backup_suffix = if matches.is_present(options::SUFFIX) {
        matches.value_of(options::SUFFIX).unwrap()
    } else {
        "~"
    };

    let settings = Settings {
        overwrite: overwrite_mode,
        backup: backup_mode,
        suffix: backup_suffix.to_string(),
        symbolic: matches.is_present(options::SYMBOLIC),
        relative: matches.is_present(options::RELATIVE),
        target_dir: matches
            .value_of(options::TARGET_DIRECTORY)
            .map(String::from),
        no_target_dir: matches.is_present(options::NO_TARGET_DIRECTORY),
        no_dereference: matches.is_present(options::NO_DEREFERENCE),
        verbose: matches.is_present(options::VERBOSE),
    };

    exec(&paths[..], &settings)
}

fn exec(files: &[PathBuf], settings: &Settings) -> i32 {
    // Handle cases where we create links in a directory first.
    if let Some(ref name) = settings.target_dir {
        // 4th form: a directory is specified by -t.
        return link_files_in_dir(files, &PathBuf::from(name), settings);
    }
    if !settings.no_target_dir {
        if files.len() == 1 {
            // 2nd form: the target directory is the current directory.
            return link_files_in_dir(files, &PathBuf::from("."), settings);
        }
        let last_file = &PathBuf::from(files.last().unwrap());
        if files.len() > 2 || last_file.is_dir() {
            // 3rd form: create links in the last argument.
            return link_files_in_dir(&files[0..files.len() - 1], last_file, settings);
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

fn link_files_in_dir(files: &[PathBuf], target_dir: &Path, settings: &Settings) -> i32 {
    if !target_dir.is_dir() {
        show_error!("target '{}' is not a directory", target_dir.display());
        return 1;
    }

    let mut all_successful = true;
    for srcpath in files.iter() {
        let targetpath =
            if settings.no_dereference && matches!(settings.overwrite, OverwriteMode::Force) {
                // In that case, we don't want to do link resolution
                // We need to clean the target
                if is_symlink(target_dir) {
                    if target_dir.is_file() {
                        if let Err(e) = fs::remove_file(target_dir) {
                            show_error!("Could not update {}: {}", target_dir.display(), e)
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
                target_dir.to_path_buf()
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

fn relative_path<'a>(src: &Path, dst: &Path) -> Result<Cow<'a, Path>> {
    let src_abs = canonicalize(src, CanonicalizeMode::Normal)?;
    let mut dst_abs = canonicalize(dst.parent().unwrap(), CanonicalizeMode::Normal)?;
    dst_abs.push(dst.components().last().unwrap());
    let suffix_pos = src_abs
        .components()
        .zip(dst_abs.components())
        .take_while(|(s, d)| s == d)
        .count();

    let src_iter = src_abs.components().skip(suffix_pos).map(|x| x.as_os_str());

    let result: PathBuf = dst_abs
        .components()
        .skip(suffix_pos + 1)
        .map(|_| OsStr::new(".."))
        .chain(src_iter)
        .collect();
    Ok(result.into())
}

fn link(src: &Path, dst: &Path, settings: &Settings) -> Result<()> {
    let mut backup_path = None;
    let source: Cow<'_, Path> = if settings.relative {
        relative_path(src, dst)?
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

fn simple_backup_path(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.as_os_str().to_str().unwrap().to_owned();
    p.push_str(suffix);
    PathBuf::from(p)
}

fn numbered_backup_path(path: &Path) -> PathBuf {
    let mut i: u64 = 1;
    loop {
        let new_path = simple_backup_path(path, &format!(".~{}~", i));
        if !new_path.exists() {
            return new_path;
        }
        i += 1;
    }
}

fn existing_backup_path(path: &Path, suffix: &str) -> PathBuf {
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
