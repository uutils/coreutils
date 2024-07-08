// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) srcpath targetpath EEXIST

use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult};
use uucore::fs::{make_path_relative_to, paths_refer_to_same_file};
use uucore::{prompt_yes, show_error};

use std::borrow::Cow;
use std::collections::HashSet;
use std::error::Error;
use std::ffi::OsString;
use std::fmt::Display;
use std::fs;

#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::path::{Path, PathBuf};
use uucore::backup_control::{self, BackupMode};
use uucore::fs::{canonicalize, MissingHandling, ResolveMode};

pub struct Settings {
    overwrite: OverwriteMode,
    backup: BackupMode,
    suffix: String,
    symbolic: bool,
    relative: bool,
    logical: bool,
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

#[derive(Debug)]
enum LnError {
    TargetIsDirectory(PathBuf),
    SomeLinksFailed,
    SameFile(PathBuf, PathBuf),
    MissingDestination(PathBuf),
    ExtraOperand(OsString),
}

impl Display for LnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetIsDirectory(s) => write!(f, "target {} is not a directory", s.quote()),
            Self::SameFile(s, d) => {
                write!(f, "{} and {} are the same file", s.quote(), d.quote())
            }
            Self::SomeLinksFailed => Ok(()),
            Self::MissingDestination(s) => {
                write!(f, "missing destination file operand after {}", s.quote())
            }
            Self::ExtraOperand(s) => write!(
                f,
                "extra operand {}\nTry '{} --help' for more information.",
                s.quote(),
                uucore::execution_phrase()
            ),
        }
    }
}

impl Error for LnError {}

impl UError for LnError {
    fn code(&self) -> i32 {
        1
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let after_help = format!(
        "{}\n\n{}",
        crate::uu_args::AFTER_HELP,
        backup_control::BACKUP_CONTROL_LONG_HELP
    );

    let matches = crate::uu_app()
        .after_help(after_help)
        .try_get_matches_from(args)?;

    /* the list of files */

    let paths: Vec<PathBuf> = matches
        .get_many::<String>(crate::uu_args::ARG_FILES)
        .unwrap()
        .map(PathBuf::from)
        .collect();

    let symbolic = matches.get_flag(crate::options::SYMBOLIC);

    let overwrite_mode = if matches.get_flag(crate::options::FORCE) {
        OverwriteMode::Force
    } else if matches.get_flag(crate::options::INTERACTIVE) {
        OverwriteMode::Interactive
    } else {
        OverwriteMode::NoClobber
    };

    let backup_mode = backup_control::determine_backup_mode(&matches)?;
    let backup_suffix = backup_control::determine_backup_suffix(&matches);

    // When we have "-L" or "-L -P", false otherwise
    let logical = matches.get_flag(crate::options::LOGICAL);

    let settings = Settings {
        overwrite: overwrite_mode,
        backup: backup_mode,
        suffix: backup_suffix,
        symbolic,
        logical,
        relative: matches.get_flag(crate::options::RELATIVE),
        target_dir: matches
            .get_one::<String>(crate::options::TARGET_DIRECTORY)
            .map(String::from),
        no_target_dir: matches.get_flag(crate::options::NO_TARGET_DIRECTORY),
        no_dereference: matches.get_flag(crate::options::NO_DEREFERENCE),
        verbose: matches.get_flag(crate::options::VERBOSE),
    };

    exec(&paths[..], &settings)
}

fn exec(files: &[PathBuf], settings: &Settings) -> UResult<()> {
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
        return Err(LnError::MissingDestination(files[0].clone()).into());
    }
    if files.len() > 2 {
        return Err(LnError::ExtraOperand(files[2].clone().into()).into());
    }
    assert!(!files.is_empty());

    link(&files[0], &files[1], settings)
}

#[allow(clippy::cognitive_complexity)]
fn link_files_in_dir(files: &[PathBuf], target_dir: &Path, settings: &Settings) -> UResult<()> {
    if !target_dir.is_dir() {
        return Err(LnError::TargetIsDirectory(target_dir.to_owned()).into());
    }
    // remember the linked destinations for further usage
    let mut linked_destinations: HashSet<PathBuf> = HashSet::with_capacity(files.len());

    let mut all_successful = true;
    for srcpath in files {
        let targetpath =
            if settings.no_dereference && matches!(settings.overwrite, OverwriteMode::Force) {
                // In that case, we don't want to do link resolution
                // We need to clean the target
                if target_dir.is_symlink() {
                    if target_dir.is_file() {
                        if let Err(e) = fs::remove_file(target_dir) {
                            show_error!("Could not update {}: {}", target_dir.quote(), e);
                        };
                    }
                    if target_dir.is_dir() {
                        // Not sure why but on Windows, the symlink can be
                        // considered as a dir
                        // See test_ln::test_symlink_no_deref_dir
                        if let Err(e) = fs::remove_dir(target_dir) {
                            show_error!("Could not update {}: {}", target_dir.quote(), e);
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
                        show_error!("cannot stat {}: No such file or directory", srcpath.quote());
                        all_successful = false;
                        continue;
                    }
                }
            };

        if linked_destinations.contains(&targetpath) {
            // If the target file was already created in this ln call, do not overwrite
            show_error!(
                "will not overwrite just-created '{}' with '{}'",
                targetpath.display(),
                srcpath.display()
            );
            all_successful = false;
        } else if let Err(e) = link(srcpath, &targetpath, settings) {
            show_error!("{}", e);
            all_successful = false;
        }

        linked_destinations.insert(targetpath.clone());
    }
    if all_successful {
        Ok(())
    } else {
        Err(LnError::SomeLinksFailed.into())
    }
}

fn relative_path<'a>(src: &'a Path, dst: &Path) -> Cow<'a, Path> {
    if let Ok(src_abs) = canonicalize(src, MissingHandling::Missing, ResolveMode::Physical) {
        if let Ok(dst_abs) = canonicalize(
            dst.parent().unwrap(),
            MissingHandling::Missing,
            ResolveMode::Physical,
        ) {
            return make_path_relative_to(src_abs, dst_abs).into();
        }
    }
    src.into()
}

#[allow(clippy::cognitive_complexity)]
fn link(src: &Path, dst: &Path, settings: &Settings) -> UResult<()> {
    let mut backup_path = None;
    let source: Cow<'_, Path> = if settings.relative {
        relative_path(src, dst)
    } else {
        src.into()
    };

    if dst.is_symlink() || dst.exists() {
        backup_path = match settings.backup {
            BackupMode::NoBackup => None,
            BackupMode::SimpleBackup => Some(simple_backup_path(dst, &settings.suffix)),
            BackupMode::NumberedBackup => Some(numbered_backup_path(dst)),
            BackupMode::ExistingBackup => Some(existing_backup_path(dst, &settings.suffix)),
        };
        if settings.backup == BackupMode::ExistingBackup && !settings.symbolic {
            // when ln --backup f f, it should detect that it is the same file
            if paths_refer_to_same_file(src, dst, true) {
                return Err(LnError::SameFile(src.to_owned(), dst.to_owned()).into());
            }
        }
        if let Some(ref p) = backup_path {
            fs::rename(dst, p).map_err_context(|| format!("cannot backup {}", dst.quote()))?;
        }
        match settings.overwrite {
            OverwriteMode::NoClobber => {}
            OverwriteMode::Interactive => {
                if !prompt_yes!("replace {}?", dst.quote()) {
                    return Err(LnError::SomeLinksFailed.into());
                }

                if fs::remove_file(dst).is_ok() {};
                // In case of error, don't do anything
            }
            OverwriteMode::Force => {
                if !dst.is_symlink() && paths_refer_to_same_file(src, dst, true) {
                    return Err(LnError::SameFile(src.to_owned(), dst.to_owned()).into());
                }
                if fs::remove_file(dst).is_ok() {};
                // In case of error, don't do anything
            }
        };
    }

    if settings.symbolic {
        symlink(&source, dst)?;
    } else {
        let p = if settings.logical && source.is_symlink() {
            // if we want to have an hard link,
            // source is a symlink and -L is passed
            // we want to resolve the symlink to create the hardlink
            fs::canonicalize(&source)
                .map_err_context(|| format!("failed to access {}", source.quote()))?
        } else {
            source.to_path_buf()
        };
        fs::hard_link(p, dst).map_err_context(|| {
            format!(
                "failed to create hard link {} => {}",
                source.quote(),
                dst.quote()
            )
        })?;
    }

    if settings.verbose {
        print!("{} -> {}", dst.quote(), source.quote());
        match backup_path {
            Some(path) => println!(" (backup: {})", path.quote()),
            None => println!(),
        }
    }
    Ok(())
}

fn simple_backup_path(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.as_os_str().to_str().unwrap().to_owned();
    p.push_str(suffix);
    PathBuf::from(p)
}

fn numbered_backup_path(path: &Path) -> PathBuf {
    let mut i: u64 = 1;
    loop {
        let new_path = simple_backup_path(path, &format!(".~{i}~"));
        if !new_path.exists() {
            return new_path;
        }
        i += 1;
    }
}

fn existing_backup_path(path: &Path, suffix: &str) -> PathBuf {
    let test_path = simple_backup_path(path, ".~1~");
    if test_path.exists() {
        return numbered_backup_path(path);
    }
    simple_backup_path(path, suffix)
}

#[cfg(windows)]
pub fn symlink<P1: AsRef<Path>, P2: AsRef<Path>>(src: P1, dst: P2) -> std::io::Result<()> {
    if src.as_ref().is_dir() {
        symlink_dir(src, dst)
    } else {
        symlink_file(src, dst)
    }
}
