// This file is part of the uutils coreutils package.
//
// (c) Orvar Segerström <orvarsegerstrom@gmail.com>
// (c) Sokovikov Evgeniy  <skv-headless@yandex.ru>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) sourcepath targetpath

mod error;

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, ArgMatches, Command};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, stdin};
#[cfg(unix)]
use std::os::unix;
#[cfg(windows)]
use std::os::windows;
use std::path::{Path, PathBuf};
use uucore::backup_control::{self, BackupMode};
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError, UUsageError};
use uucore::format_usage;

use fs_extra::dir::{move_dir, CopyOptions as DirCopyOptions};

use crate::error::MvError;

pub struct Behavior {
    overwrite: OverwriteMode,
    backup: BackupMode,
    suffix: String,
    update: bool,
    target_dir: Option<OsString>,
    no_target_dir: bool,
    verbose: bool,
    strip_slashes: bool,
}

#[derive(Clone, Eq, PartialEq)]
pub enum OverwriteMode {
    NoClobber,
    Interactive,
    Force,
}

static ABOUT: &str = "Move SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.";
static LONG_HELP: &str = "";
const USAGE: &str = "\
    {} [OPTION]... [-T] SOURCE DEST
    {} [OPTION]... SOURCE... DIRECTORY
    {} [OPTION]... -t DIRECTORY SOURCE...";

static OPT_FORCE: &str = "force";
static OPT_INTERACTIVE: &str = "interactive";
static OPT_NO_CLOBBER: &str = "no-clobber";
static OPT_STRIP_TRAILING_SLASHES: &str = "strip-trailing-slashes";
static OPT_TARGET_DIRECTORY: &str = "target-directory";
static OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
static OPT_UPDATE: &str = "update";
static OPT_VERBOSE: &str = "verbose";

static ARG_FILES: &str = "files";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app()
        .after_help(&*format!(
            "{}\n{}",
            LONG_HELP,
            backup_control::BACKUP_CONTROL_LONG_HELP
        ))
        .get_matches_from(args);

    let files: Vec<OsString> = matches
        .values_of_os(ARG_FILES)
        .unwrap_or_default()
        .map(|v| v.to_os_string())
        .collect();

    let overwrite_mode = determine_overwrite_mode(&matches);
    let backup_mode = backup_control::determine_backup_mode(&matches)?;

    if overwrite_mode == OverwriteMode::NoClobber && backup_mode != BackupMode::NoBackup {
        return Err(UUsageError::new(
            1,
            "options --backup and --no-clobber are mutually exclusive",
        ));
    }

    let backup_suffix = backup_control::determine_backup_suffix(&matches);

    let behavior = Behavior {
        overwrite: overwrite_mode,
        backup: backup_mode,
        suffix: backup_suffix,
        update: matches.is_present(OPT_UPDATE),
        target_dir: matches
            .value_of_os(OPT_TARGET_DIRECTORY)
            .map(OsString::from),
        no_target_dir: matches.is_present(OPT_NO_TARGET_DIRECTORY),
        verbose: matches.is_present(OPT_VERBOSE),
        strip_slashes: matches.is_present(OPT_STRIP_TRAILING_SLASHES),
    };

    exec(&files[..], &behavior)
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
    .arg(
        backup_control::arguments::backup()
    )
    .arg(
        backup_control::arguments::backup_no_args()
    )
    .arg(
            Arg::new(OPT_FORCE)
            .short('f')
            .long(OPT_FORCE)
            .help("do not prompt before overwriting")
    )
    .arg(
            Arg::new(OPT_INTERACTIVE)
            .short('i')
            .long(OPT_INTERACTIVE)
            .help("prompt before override")
    )
    .arg(
            Arg::new(OPT_NO_CLOBBER).short('n')
            .long(OPT_NO_CLOBBER)
            .help("do not overwrite an existing file")
    )
    .arg(
            Arg::new(OPT_STRIP_TRAILING_SLASHES)
            .long(OPT_STRIP_TRAILING_SLASHES)
            .help("remove any trailing slashes from each SOURCE argument")
    )
    .arg(
        backup_control::arguments::suffix()
    )
    .arg(
        Arg::new(OPT_TARGET_DIRECTORY)
        .short('t')
        .long(OPT_TARGET_DIRECTORY)
        .help("move all SOURCE arguments into DIRECTORY")
        .takes_value(true)
        .value_name("DIRECTORY")
        .conflicts_with(OPT_NO_TARGET_DIRECTORY)
        .allow_invalid_utf8(true)
    )
    .arg(
            Arg::new(OPT_NO_TARGET_DIRECTORY)
            .short('T')
            .long(OPT_NO_TARGET_DIRECTORY).
            help("treat DEST as a normal file")
    )
    .arg(
            Arg::new(OPT_UPDATE)
            .short('u')
            .long(OPT_UPDATE)
            .help("move only when the SOURCE file is newer than the destination file or when the destination file is missing")
    )
    .arg(
            Arg::new(OPT_VERBOSE)
            .short('v')
            .long(OPT_VERBOSE).help("explain what is being done")
    )
    .arg(
        Arg::new(ARG_FILES)
            .multiple_occurrences(true)
            .takes_value(true)
            .min_values(2)
            .required(true)
            .allow_invalid_utf8(true)
        )
}

fn determine_overwrite_mode(matches: &ArgMatches) -> OverwriteMode {
    // This does not exactly match the GNU implementation:
    // The GNU mv defaults to Force, but if more than one of the
    // overwrite options are supplied, only the last takes effect.
    // To default to no-clobber in that situation seems safer:
    //
    if matches.is_present(OPT_NO_CLOBBER) {
        OverwriteMode::NoClobber
    } else if matches.is_present(OPT_INTERACTIVE) {
        OverwriteMode::Interactive
    } else {
        OverwriteMode::Force
    }
}

fn exec(files: &[OsString], b: &Behavior) -> UResult<()> {
    let paths: Vec<PathBuf> = {
        let paths = files.iter().map(Path::new);

        // Strip slashes from path, if strip opt present
        if b.strip_slashes {
            paths
                .map(|p| p.components().as_path().to_owned())
                .collect::<Vec<PathBuf>>()
        } else {
            paths.map(|p| p.to_owned()).collect::<Vec<PathBuf>>()
        }
    };

    if let Some(ref name) = b.target_dir {
        return move_files_into_dir(&paths, &PathBuf::from(name), b);
    }
    match paths.len() {
        /* case 0/1 are not possible thanks to clap */
        2 => {
            let source = &paths[0];
            let target = &paths[1];
            // Here we use the `symlink_metadata()` method instead of `exists()`,
            // since it handles dangling symlinks correctly. The method gives an
            // `Ok()` results unless the source does not exist, or the user
            // lacks permission to access metadata.
            if source.symlink_metadata().is_err() {
                return Err(MvError::NoSuchFile(source.quote().to_string()).into());
            }

            // GNU semantics are: if the source and target are the same, no move occurs and we print an error
            if source.eq(target) {
                // Done to match GNU semantics for the dot file
                if source.eq(Path::new(".")) || source.ends_with("/.") || source.is_file() {
                    return Err(MvError::SameFile(
                        source.quote().to_string(),
                        target.quote().to_string(),
                    )
                    .into());
                } else {
                    return Err(MvError::SelfSubdirectory(source.display().to_string()).into());
                }
            }

            if target.is_dir() {
                if b.no_target_dir {
                    if !source.is_dir() {
                        Err(MvError::DirectoryToNonDirectory(target.quote().to_string()).into())
                    } else {
                        rename(source, target, b).map_err_context(|| {
                            format!("cannot move {} to {}", source.quote(), target.quote())
                        })
                    }
                } else {
                    move_files_into_dir(&[source.clone()], target, b)
                }
            } else if target.exists() && source.is_dir() {
                Err(MvError::NonDirectoryToDirectory(
                    source.quote().to_string(),
                    target.quote().to_string(),
                )
                .into())
            } else {
                rename(source, target, b).map_err(|e| USimpleError::new(1, format!("{}", e)))
            }
        }
        _ => {
            if b.no_target_dir {
                return Err(UUsageError::new(
                    1,
                    format!("mv: extra operand {}", files[2].quote()),
                ));
            }
            let target_dir = paths.last().unwrap();
            move_files_into_dir(&paths[..paths.len() - 1], target_dir, b)
        }
    }
}

fn move_files_into_dir(files: &[PathBuf], target_dir: &Path, b: &Behavior) -> UResult<()> {
    if !target_dir.is_dir() {
        return Err(MvError::NotADirectory(target_dir.quote().to_string()).into());
    }

    for sourcepath in files.iter() {
        let targetpath = match sourcepath.file_name() {
            Some(name) => target_dir.join(name),
            None => {
                show!(MvError::NoSuchFile(sourcepath.quote().to_string()));
                continue;
            }
        };
        show_if_err!(
            rename(sourcepath, &targetpath, b).map_err_context(|| format!(
                "cannot move {} to {}",
                sourcepath.quote(),
                targetpath.quote()
            ))
        );
    }
    Ok(())
}

fn rename(from: &Path, to: &Path, b: &Behavior) -> io::Result<()> {
    let mut backup_path = None;

    if to.exists() {
        match b.overwrite {
            OverwriteMode::NoClobber => return Ok(()),
            OverwriteMode::Interactive => {
                println!("{}: overwrite {}? ", uucore::util_name(), to.quote());
                if !read_yes() {
                    return Ok(());
                }
            }
            OverwriteMode::Force => {}
        };

        backup_path = backup_control::get_backup_path(b.backup, to, &b.suffix);
        if let Some(ref backup_path) = backup_path {
            rename_with_fallback(to, backup_path)?;
        }

        if b.update && fs::metadata(from)?.modified()? <= fs::metadata(to)?.modified()? {
            return Ok(());
        }
    }

    // "to" may no longer exist if it was backed up
    if to.exists() && to.is_dir() {
        // normalize behavior between *nix and windows
        if from.is_dir() {
            if is_empty_dir(to) {
                fs::remove_dir(to)?;
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "Directory not empty"));
            }
        }
    }

    rename_with_fallback(from, to)?;

    if b.verbose {
        print!("{} -> {}", from.quote(), to.quote());
        match backup_path {
            Some(path) => println!(" (backup: {})", path.quote()),
            None => println!(),
        }
    }
    Ok(())
}

/// A wrapper around `fs::rename`, so that if it fails, we try falling back on
/// copying and removing.
fn rename_with_fallback(from: &Path, to: &Path) -> io::Result<()> {
    if fs::rename(from, to).is_err() {
        // Get metadata without following symlinks
        let metadata = from.symlink_metadata()?;
        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            rename_symlink_fallback(from, to)?;
        } else if file_type.is_dir() {
            // We remove the destination directory if it exists to match the
            // behavior of `fs::rename`. As far as I can tell, `fs_extra`'s
            // `move_dir` would otherwise behave differently.
            if to.exists() {
                fs::remove_dir_all(to)?;
            }
            let options = DirCopyOptions {
                // From the `fs_extra` documentation:
                // "Recursively copy a directory with a new name or place it
                // inside the destination. (same behaviors like cp -r in Unix)"
                copy_inside: true,
                ..DirCopyOptions::new()
            };
            if let Err(err) = move_dir(from, to, &options) {
                return match err.kind {
                    fs_extra::error::ErrorKind::PermissionDenied => Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Permission denied",
                    )),
                    _ => Err(io::Error::new(io::ErrorKind::Other, format!("{:?}", err))),
                };
            }
        } else {
            fs::copy(from, to).and_then(|_| fs::remove_file(from))?;
        }
    }
    Ok(())
}

/// Move the given symlink to the given destination. On Windows, dangling
/// symlinks return an error.
#[inline]
fn rename_symlink_fallback(from: &Path, to: &Path) -> io::Result<()> {
    let path_symlink_points_to = fs::read_link(from)?;
    #[cfg(unix)]
    {
        unix::fs::symlink(&path_symlink_points_to, &to).and_then(|_| fs::remove_file(&from))?;
    }
    #[cfg(windows)]
    {
        if path_symlink_points_to.exists() {
            if path_symlink_points_to.is_dir() {
                windows::fs::symlink_dir(&path_symlink_points_to, &to)?;
            } else {
                windows::fs::symlink_file(&path_symlink_points_to, &to)?;
            }
            fs::remove_file(&from)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "can't determine symlink type, since it is dangling",
            ));
        }
    }
    #[cfg(not(any(windows, unix)))]
    {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "your operating system does not support symlinks",
        ));
    }
    Ok(())
}

fn read_yes() -> bool {
    let mut s = String::new();
    match stdin().read_line(&mut s) {
        Ok(_) => match s.chars().next() {
            Some(x) => x == 'y' || x == 'Y',
            _ => false,
        },
        _ => false,
    }
}

fn is_empty_dir(path: &Path) -> bool {
    match fs::read_dir(path) {
        Ok(contents) => contents.peekable().peek().is_none(),
        Err(_e) => false,
    }
}
