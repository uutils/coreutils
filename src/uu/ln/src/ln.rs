//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Joseph Crail <jbcrail@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) srcpath targetpath EEXIST

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult};
use uucore::format_usage;
use uucore::fs::{make_path_relative_to, paths_refer_to_same_file};

use std::borrow::Cow;
use std::error::Error;
use std::ffi::OsString;
use std::fmt::Display;
use std::fs;

use std::io::stdin;
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

fn long_usage() -> String {
    String::from(
        " In the 1st form, create a link to TARGET with the name LINK_NAME.
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
const USAGE: &str = "\
    {} [OPTION]... [-T] TARGET LINK_NAME
    {} [OPTION]... TARGET
    {} [OPTION]... TARGET... DIRECTORY
    {} [OPTION]... -t DIRECTORY TARGET...";

mod options {
    pub const FORCE: &str = "force";
    //pub const DIRECTORY: &str = "directory";
    pub const INTERACTIVE: &str = "interactive";
    pub const NO_DEREFERENCE: &str = "no-dereference";
    pub const SYMBOLIC: &str = "symbolic";
    pub const LOGICAL: &str = "logical";
    pub const PHYSICAL: &str = "physical";
    pub const TARGET_DIRECTORY: &str = "target-directory";
    pub const NO_TARGET_DIRECTORY: &str = "no-target-directory";
    pub const RELATIVE: &str = "relative";
    pub const VERBOSE: &str = "verbose";
}

static ARG_FILES: &str = "files";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // clap requires a 'static string
    let long_usage = format!(
        "{}\n{}",
        long_usage(),
        backup_control::BACKUP_CONTROL_LONG_HELP
    );

    let matches = uu_app().after_help(long_usage).try_get_matches_from(args)?;

    /* the list of files */

    let paths: Vec<PathBuf> = matches
        .get_many::<String>(ARG_FILES)
        .unwrap()
        .map(PathBuf::from)
        .collect();

    let symbolic = matches.get_flag(options::SYMBOLIC);

    let overwrite_mode = if matches.get_flag(options::FORCE) {
        OverwriteMode::Force
    } else if matches.get_flag(options::INTERACTIVE) {
        OverwriteMode::Interactive
    } else {
        OverwriteMode::NoClobber
    };

    let backup_mode = backup_control::determine_backup_mode(&matches)?;
    let backup_suffix = backup_control::determine_backup_suffix(&matches);

    // When we have "-L" or "-L -P", false otherwise
    let logical = matches.get_flag(options::LOGICAL);

    let settings = Settings {
        overwrite: overwrite_mode,
        backup: backup_mode,
        suffix: backup_suffix,
        symbolic,
        logical,
        relative: matches.get_flag(options::RELATIVE),
        target_dir: matches
            .get_one::<String>(options::TARGET_DIRECTORY)
            .map(String::from),
        no_target_dir: matches.get_flag(options::NO_TARGET_DIRECTORY),
        no_dereference: matches.get_flag(options::NO_DEREFERENCE),
        verbose: matches.get_flag(options::VERBOSE),
    };

    exec(&paths[..], &settings)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(backup_control::arguments::backup())
        .arg(backup_control::arguments::backup_no_args())
        /*.arg(
            Arg::new(options::DIRECTORY)
                .short('d')
                .long(options::DIRECTORY)
                .help("allow users with appropriate privileges to attempt to make hard links to directories")
        )*/
        .arg(
            Arg::new(options::FORCE)
                .short('f')
                .long(options::FORCE)
                .help("remove existing destination files")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::INTERACTIVE)
                .short('i')
                .long(options::INTERACTIVE)
                .help("prompt whether to remove existing destination files")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_DEREFERENCE)
                .short('n')
                .long(options::NO_DEREFERENCE)
                .help(
                    "treat LINK_NAME as a normal file if it is a \
                     symbolic link to a directory",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LOGICAL)
                .short('L')
                .long(options::LOGICAL)
                .help("dereference TARGETs that are symbolic links")
                .overrides_with(options::PHYSICAL)
                .action(ArgAction::SetTrue),
        )
        .arg(
            // Not implemented yet
            Arg::new(options::PHYSICAL)
                .short('P')
                .long(options::PHYSICAL)
                .help("make hard links directly to symbolic links")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SYMBOLIC)
                .short('s')
                .long(options::SYMBOLIC)
                .help("make symbolic links instead of hard links")
                // override added for https://github.com/uutils/coreutils/issues/2359
                .overrides_with(options::SYMBOLIC)
                .action(ArgAction::SetTrue),
        )
        .arg(backup_control::arguments::suffix())
        .arg(
            Arg::new(options::TARGET_DIRECTORY)
                .short('t')
                .long(options::TARGET_DIRECTORY)
                .help("specify the DIRECTORY in which to create the links")
                .value_name("DIRECTORY")
                .value_hint(clap::ValueHint::DirPath)
                .conflicts_with(options::NO_TARGET_DIRECTORY),
        )
        .arg(
            Arg::new(options::NO_TARGET_DIRECTORY)
                .short('T')
                .long(options::NO_TARGET_DIRECTORY)
                .help("treat LINK_NAME as a normal file always")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RELATIVE)
                .short('r')
                .long(options::RELATIVE)
                .help("create symbolic links relative to link location")
                .requires(options::SYMBOLIC)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help("print name of each linked file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .required(true)
                .num_args(1..),
        )
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

fn link_files_in_dir(files: &[PathBuf], target_dir: &Path, settings: &Settings) -> UResult<()> {
    if !target_dir.is_dir() {
        return Err(LnError::TargetIsDirectory(target_dir.to_owned()).into());
    }

    let mut all_successful = true;
    for srcpath in files.iter() {
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

        if let Err(e) = link(srcpath, &targetpath, settings) {
            show_error!("{}", e);
            all_successful = false;
        }
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
                print!("{}: overwrite {}? ", uucore::util_name(), dst.quote());
                if !read_yes() {
                    return Ok(());
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
        fs::hard_link(&p, dst).map_err_context(|| {
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
