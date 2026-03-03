// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) srcpath targetpath EEXIST

use clap::{Arg, ArgAction, Command};
use std::io::{Write, stdout};
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult};
use uucore::fs::{make_path_relative_to, paths_refer_to_same_file};
use uucore::translate;
use uucore::{format_usage, prompt_yes, show_error};

use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use thiserror::Error;

#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::path::{Path, PathBuf};
use uucore::backup_control::{self, BackupMode};
use uucore::fs::{MissingHandling, ResolveMode, canonicalize};

pub struct Settings {
    overwrite: OverwriteMode,
    backup: BackupMode,
    suffix: OsString,
    symbolic: bool,
    relative: bool,
    logical: bool,
    target_dir: Option<PathBuf>,
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

#[derive(Error, Debug)]
enum LnError {
    #[error("{}", translate!("ln-error-target-is-not-directory", "target" => _0.quote()))]
    TargetIsNotADirectory(PathBuf),

    #[error("")]
    SomeLinksFailed,

    #[error("{}", translate!("ln-error-same-file", "file1" => _0.quote(), "file2" => _1.quote()))]
    SameFile(PathBuf, PathBuf),

    #[error("{}", translate!("ln-error-missing-destination", "operand" => _0.quote()))]
    MissingDestination(PathBuf),

    #[error("{}", translate!("ln-error-extra-operand", "operand" => _0.quote(), "program" => _1.clone()))]
    ExtraOperand(OsString, String),

    #[error("{}", translate!("ln-failed-to-create-hard-link-dir", "source" => _0.to_string_lossy()))]
    FailedToCreateHardLinkDir(PathBuf),
}

impl UError for LnError {
    fn code(&self) -> i32 {
        1
    }
}

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
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    /* the list of files */

    let paths: Vec<PathBuf> = matches
        .get_many::<OsString>(ARG_FILES)
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
        suffix: OsString::from(backup_suffix),
        symbolic,
        logical,
        relative: matches.get_flag(options::RELATIVE),
        target_dir: matches
            .get_one::<OsString>(options::TARGET_DIRECTORY)
            .map(PathBuf::from),
        no_target_dir: matches.get_flag(options::NO_TARGET_DIRECTORY),
        no_dereference: matches.get_flag(options::NO_DEREFERENCE),
        verbose: matches.get_flag(options::VERBOSE),
    };

    exec(&paths[..], &settings)
}

pub fn uu_app() -> Command {
    let after_help = format!(
        "{}\n\n{}",
        translate!("ln-after-help"),
        backup_control::BACKUP_CONTROL_LONG_HELP
    );

    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("ln-about"))
        .override_usage(format_usage(&translate!("ln-usage")))
        .infer_long_args(true)
        .after_help(after_help)
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
                .help(translate!("ln-help-force"))
                .overrides_with(options::INTERACTIVE)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::INTERACTIVE)
                .short('i')
                .long(options::INTERACTIVE)
                .help(translate!("ln-help-interactive"))
                .overrides_with(options::FORCE)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_DEREFERENCE)
                .short('n')
                .long(options::NO_DEREFERENCE)
                .help(translate!("ln-help-no-dereference"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LOGICAL)
                .short('L')
                .long(options::LOGICAL)
                .help(translate!("ln-help-logical"))
                .overrides_with(options::PHYSICAL)
                .action(ArgAction::SetTrue),
        )
        .arg(
            // Not implemented yet
            Arg::new(options::PHYSICAL)
                .short('P')
                .long(options::PHYSICAL)
                .help(translate!("ln-help-physical"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SYMBOLIC)
                .short('s')
                .long(options::SYMBOLIC)
                .help(translate!("ln-help-symbolic"))
                // override added for https://github.com/uutils/coreutils/issues/2359
                .overrides_with(options::SYMBOLIC)
                .action(ArgAction::SetTrue),
        )
        .arg(backup_control::arguments::suffix())
        .arg(
            Arg::new(options::TARGET_DIRECTORY)
                .short('t')
                .long(options::TARGET_DIRECTORY)
                .help(translate!("ln-help-target-directory"))
                .value_name("DIRECTORY")
                .value_hint(clap::ValueHint::DirPath)
                .value_parser(clap::value_parser!(OsString))
                .conflicts_with(options::NO_TARGET_DIRECTORY),
        )
        .arg(
            Arg::new(options::NO_TARGET_DIRECTORY)
                .short('T')
                .long(options::NO_TARGET_DIRECTORY)
                .help(translate!("ln-help-no-target-directory"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RELATIVE)
                .short('r')
                .long(options::RELATIVE)
                .help(translate!("ln-help-relative"))
                .requires(options::SYMBOLIC)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help(translate!("ln-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(clap::value_parser!(OsString))
                .required(true)
                .num_args(1..),
        )
}

fn exec(files: &[PathBuf], settings: &Settings) -> UResult<()> {
    // Handle cases where we create links in a directory first.
    if let Some(ref target_path) = settings.target_dir {
        // 4th form: a directory is specified by -t.
        return link_files_in_dir(files, target_path, settings);
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
        return Err(LnError::ExtraOperand(
            files[2].clone().into(),
            uucore::execution_phrase().to_string(),
        )
        .into());
    }
    assert!(!files.is_empty());

    link(&files[0], &files[1], settings)
}

#[allow(clippy::cognitive_complexity)]
fn link_files_in_dir(files: &[PathBuf], target_dir: &Path, settings: &Settings) -> UResult<()> {
    if !target_dir.is_dir() {
        return Err(LnError::TargetIsNotADirectory(target_dir.to_owned()).into());
    }
    // remember the linked destinations for further usage
    let mut linked_destinations: HashSet<PathBuf> = HashSet::with_capacity(files.len());

    let mut all_successful = true;
    for srcpath in files {
        let targetpath = if settings.no_dereference
            && matches!(settings.overwrite, OverwriteMode::Force)
            && target_dir.is_symlink()
        {
            // In that case, we don't want to do link resolution
            // We need to clean the target
            if target_dir.is_file() {
                if let Err(e) = fs::remove_file(target_dir) {
                    show_error!(
                        "{}",
                        translate!("ln-error-could-not-update", "target" => target_dir.quote(), "error" => e)
                    );
                }
            }
            #[cfg(windows)]
            if target_dir.is_dir() {
                // Not sure why but on Windows, the symlink can be
                // considered as a dir
                // See test_ln::test_symlink_no_deref_dir
                if let Err(e) = fs::remove_dir(target_dir) {
                    show_error!(
                        "{}",
                        translate!("ln-error-could-not-update", "target" => target_dir.quote(), "error" => e)
                    );
                }
            }
            target_dir.to_path_buf()
        } else if let Some(name) = srcpath.as_os_str().to_str() {
            match Path::new(name).file_name() {
                Some(basename) => target_dir.join(basename),
                // This can be None only for "." or "..". Trying
                // to create a link with such name will fail with
                // EEXIST, which agrees with the behavior of GNU
                // coreutils.
                None => target_dir.join(name),
            }
        } else {
            show_error!(
                "{}",
                translate!("ln-error-cannot-stat", "path" => srcpath.quote())
            );
            all_successful = false;
            continue;
        };

        if linked_destinations.contains(&targetpath) {
            // If the target file was already created in this ln call, do not overwrite
            show_error!(
                "{}",
                translate!("ln-error-will-not-overwrite", "target" => targetpath.quote(), "source" => srcpath.quote())
            );
            all_successful = false;
        } else if let Err(e) = link(srcpath, &targetpath, settings) {
            show_error!("{e}");
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
        backup_path = backup_control::get_backup_path(settings.backup, dst, &settings.suffix);
        if settings.backup == BackupMode::Existing && !settings.symbolic {
            // when ln --backup f f, it should detect that it is the same file
            if paths_refer_to_same_file(src, dst, true) {
                return Err(LnError::SameFile(src.to_owned(), dst.to_owned()).into());
            }
        }
        if let Some(ref p) = backup_path {
            fs::rename(dst, p)
                .map_err_context(|| translate!("ln-cannot-backup", "file" => dst.quote()))?;
        }
        match settings.overwrite {
            OverwriteMode::NoClobber => {}
            OverwriteMode::Interactive => {
                if !prompt_yes!("{}", translate!("ln-prompt-replace", "file" => dst.quote())) {
                    return Err(LnError::SomeLinksFailed.into());
                }

                if fs::remove_file(dst).is_ok() {}
                // In case of error, don't do anything
            }
            OverwriteMode::Force => {
                if !dst.is_symlink() && paths_refer_to_same_file(src, dst, true) {
                    // Even in force overwrite mode, verify we are not targeting the same entry and return a SameFile error if so
                    let same_entry = match (
                        canonicalize(src, MissingHandling::Missing, ResolveMode::Physical),
                        canonicalize(dst, MissingHandling::Missing, ResolveMode::Physical),
                    ) {
                        (Ok(src), Ok(dst)) => src == dst,
                        _ => true,
                    };
                    if same_entry {
                        return Err(LnError::SameFile(src.to_owned(), dst.to_owned()).into());
                    }
                }
                if fs::remove_file(dst).is_ok() {}
                // In case of error, don't do anything
            }
        }
    }

    if settings.symbolic {
        symlink(&source, dst)?;
    } else {
        let p = if settings.logical && source.is_symlink() {
            fs::canonicalize(&source)
                .map_err_context(|| translate!("ln-failed-to-access", "file" => source.quote()))?
        } else {
            source.to_path_buf()
        };
        if let Err(e) = fs::hard_link(&p, dst) {
            if p.is_dir() {
                return Err(LnError::FailedToCreateHardLinkDir(source.to_path_buf()).into());
            }
            return Err(e).map_err_context(|| {
                translate!("ln-failed-to-create-hard-link", "source" => source.quote(), "dest" => dst.quote())
            });
        }
    }

    if settings.verbose {
        let mut out = stdout();
        write!(out, "{} -> {}", dst.quote(), source.quote())?;
        match backup_path {
            Some(path) => writeln!(
                out,
                " ({})",
                translate!("ln-backup", "backup" => path.quote())
            )?,
            None => writeln!(out)?,
        }
    }
    Ok(())
}

#[cfg(windows)]
pub fn symlink<P1: AsRef<Path>, P2: AsRef<Path>>(src: P1, dst: P2) -> std::io::Result<()> {
    if src.as_ref().is_dir() {
        symlink_dir(src, dst)
    } else {
        symlink_file(src, dst)
    }
}
