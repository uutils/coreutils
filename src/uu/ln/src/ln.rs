// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) srcpath targetpath EEXIST renameat

use clap::{Arg, ArgAction, Command};
use std::io::{self, Write, stdout};
use uucore::display::Quotable;
use uucore::error::{UError, UIoError, UResult};

use uucore::fs::replace_link;
use uucore::fs::{make_path_relative_to, paths_refer_to_same_file};
use uucore::translate;
use uucore::{format_usage, prompt_yes, show_error};

use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;

#[cfg(all(unix, not(target_os = "redox")))]
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use uucore::backup_control::{self, BackupMode};
#[cfg(all(unix, not(target_os = "redox")))]
use uucore::fs::link_at;
use uucore::fs::{MissingHandling, ResolveMode, canonicalize};
#[cfg(all(unix, not(target_os = "redox")))]
use uucore::safe_traversal::{DirFd, SymlinkBehavior};

/// Public visibility allows other apps to integrate with our
/// `ln` utility by calling `exec` directly with their `Settings`.
pub struct Settings {
    pub overwrite: OverwriteMode,
    pub backup: BackupMode,
    pub suffix: OsString,
    pub symbolic: bool,
    pub relative: bool,
    pub logical: bool,
    pub target_dir: Option<PathBuf>,
    pub no_target_dir: bool,
    pub no_dereference: bool,
    pub verbose: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OverwriteMode {
    NoClobber,
    Interactive,
    Force,
}

#[derive(Debug, thiserror::Error)]
pub enum LnError {
    #[error("{}", translate!("ln-error-target-is-not-directory", "target" => _0.quote()))]
    TargetIsNotADirectory(PathBuf),

    #[error("{0}")]
    Io(#[from] UIoError),

    #[error("{1}: {0}")]
    IoContext(UIoError, String),

    #[error("")]
    SomeLinksFailed,

    #[error("{}", translate!("ln-error-same-file", "file1" => _0.quote(), "file2" => _1.quote()))]
    SameFile(PathBuf, PathBuf),

    #[error("{}", translate!("ln-error-missing-destination", "operand" => _0.quote()))]
    MissingDestination(PathBuf),

    #[error("{}", translate!("ln-error-extra-operand", "operand" => _0.quote(), "program" => _1))]
    ExtraOperand(OsString, String),

    #[error("{}", translate!("ln-failed-to-create-hard-link-dir", "source" => _0.to_string_lossy()))]
    FailedToCreateHardLinkDir(PathBuf),
}

impl UError for LnError {
    fn code(&self) -> i32 {
        1
    }
}
pub type LnResult<T> = Result<T, LnError>;

impl From<io::Error> for LnError {
    fn from(err: io::Error) -> Self {
        Self::Io(UIoError::from(err))
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

    let backup_mode =
        backup_control::determine_backup_mode(std::env::var("VERSION_CONTROL").ok(), &matches)?;
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

    exec(&paths[..], &settings)?;
    Ok(())
}

pub fn uu_app() -> Command {
    let after_help = format!(
        "{}\n\n{}",
        translate!("ln-after-help"),
        backup_control::BACKUP_CONTROL_LONG_HELP
    );

    Command::new("ln")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("ln"))
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

fn exec(files: &[PathBuf], settings: &Settings) -> LnResult<()> {
    // Handle cases where we create links in a directory first.
    if let Some(ref target_path) = settings.target_dir {
        // 4th form: a directory is specified by -t.
        return link_files_in_dir(files, target_path, settings);
    }

    // if -T is specified we may have a wrong number of operands.
    match files.split_last().expect("clap rejects empty") {
        // the target directory is the current directory
        (_, []) if !settings.no_target_dir => {
            link_files_in_dir(files, &PathBuf::from("."), settings)
        }
        // create links in the last argument
        (last, rest) if !settings.no_target_dir && (rest.len() > 1 || last.is_dir()) => {
            link_files_in_dir(rest, last, settings)
        }
        (f0, []) => Err(LnError::MissingDestination(f0.clone())),
        (last, [f]) => link(f, last, settings),
        (extra, [_, _]) | (_, [_, _, extra, ..]) => Err(LnError::ExtraOperand(
            extra.into(),
            uucore::execution_phrase().to_string(),
        )),
    }
}

#[allow(clippy::cognitive_complexity)]
fn link_files_in_dir(files: &[PathBuf], target_dir: &Path, settings: &Settings) -> LnResult<()> {
    if !target_dir.is_dir() {
        return Err(LnError::TargetIsNotADirectory(target_dir.to_owned()));
    }
    let replace_target_dir = settings.no_dereference && target_dir.is_symlink();
    // Links are made through the open directory, so they land in it even if
    // `target_dir` is renamed meanwhile.
    #[cfg(all(unix, not(target_os = "redox")))]
    let dir = if replace_target_dir {
        None
    } else {
        Some(DirFd::open_anchor(target_dir).map_err(|e| {
            LnError::IoContext(
                UIoError::from(e),
                translate!("ln-failed-to-access", "file" => target_dir.quote()),
            )
        })?)
    };
    // remember the linked destinations for further usage
    let mut linked_destinations: HashSet<PathBuf> = HashSet::with_capacity(files.len());

    let mut all_successful = true;
    for srcpath in files {
        let dest = if replace_target_dir {
            let remove_target = || {
                // Not sure why but on Windows, the symlink can be
                // considered as a dir
                // See test_ln::test_symlink_no_deref_dir
                #[cfg(windows)]
                if let Err(e) = fs::remove_dir(target_dir) {
                    show_error!(
                        "{}",
                        translate!("ln-error-could-not-update", "target" => target_dir.quote(), "error" => e)
                    );
                }
            };
            match settings.overwrite {
                OverwriteMode::NoClobber => {}
                OverwriteMode::Interactive => {
                    if prompt_yes!(
                        "{}",
                        translate!("ln-prompt-replace", "file" => target_dir.quote())
                    ) {
                        remove_target();
                    }
                }
                OverwriteMode::Force => {
                    remove_target();
                }
            }
            Dest::by_path(target_dir.into())
        } else {
            // This can be None only for "." or "..". Trying
            // to create a link with such name will fail with
            // EEXIST, which agrees with the behavior of GNU
            // coreutils.
            let name = srcpath.file_name().unwrap_or(srcpath.as_os_str());
            Dest {
                path: target_dir.join(name).into(),
                #[cfg(all(unix, not(target_os = "redox")))]
                at: dir.as_ref().map(|dir| InDir {
                    dir,
                    dir_path: target_dir,
                    name: name.into(),
                }),
            }
        };

        if linked_destinations.contains(dest.path.as_ref()) {
            // If the target file was already created in this ln call, do not overwrite
            show_error!(
                "{}",
                translate!("ln-error-will-not-overwrite", "target" => dest.path.quote(), "source" => srcpath.quote())
            );
            all_successful = false;
        } else if let Err(e) = link_to(srcpath, &dest, settings) {
            // A declined prompt has nothing to report.
            if !matches!(e, LnError::SomeLinksFailed) {
                show_error!("{e}");
            }
            all_successful = false;
        }

        linked_destinations.insert(dest.path.into_owned());
    }
    if all_successful {
        Ok(())
    } else {
        Err(LnError::SomeLinksFailed)
    }
}

fn relative_path<'a>(src: &'a Path, dst: &Path) -> Cow<'a, Path> {
    // `dst.parent()` is None for a destination with no parent (`/`, `""`, or a
    // bare Windows prefix). Fall through to the non-relative `src` rather than
    // unwrapping it; the caller then reports the usual error.
    let Some(dst_parent) = dst.parent() else {
        return src.into();
    };
    let (Ok(src_abs), Ok(dst_abs)) = (
        canonicalize(src, MissingHandling::Missing, ResolveMode::Physical),
        canonicalize(dst_parent, MissingHandling::Missing, ResolveMode::Physical),
    ) else {
        return src.into();
    };

    make_path_relative_to(src_abs, dst_abs).into()
}

/// Decide whether `src` and `dst` are actually the same directory entry.
fn is_same_entry(src: &Path, dst: &Path) -> bool {
    match (
        canonicalize(src, MissingHandling::Missing, ResolveMode::Physical),
        canonicalize(dst, MissingHandling::Missing, ResolveMode::Physical),
    ) {
        (Ok(src), Ok(dst)) => src == dst,
        _ => true,
    }
}

/// Create symlink to src at dst with the given settings
///
/// This is made public to allow other apps to use `ln` as a library.
pub fn link(src: &Path, dst: &Path, settings: &Settings) -> LnResult<()> {
    link_to(src, &Dest::by_path(dst.into()), settings)
}

fn link_to(src: &Path, dest: &Dest, settings: &Settings) -> LnResult<()> {
    let dst = dest.path.as_ref();
    let mut backup = None;
    // Remove an existing destination only after a link attempt fails with EEXIST.
    let mut overwrite_on_conflict = false;
    let source: Cow<'_, Path> = if settings.relative {
        relative_path(src, dst)
    } else {
        src.into()
    };

    if dest.exists() {
        backup = dest.backup(settings);
        if settings.backup != BackupMode::None && !settings.symbolic {
            // `ln --backup f f`: the backup would move `f` away before the
            // link is made, so detect the same file first, for any backup mode
            if paths_refer_to_same_file(src, dst, true) && is_same_entry(src, dst) {
                return Err(LnError::SameFile(src.to_owned(), dst.to_owned()));
            }
        }
        match settings.overwrite {
            OverwriteMode::NoClobber => {}
            OverwriteMode::Interactive => {
                if !prompt_yes!("{}", translate!("ln-prompt-replace", "file" => dst.quote())) {
                    return Err(LnError::SomeLinksFailed);
                }

                overwrite_on_conflict = true;
            }
            OverwriteMode::Force => {
                // A backup moves the destination away before the link is made.
                if backup.is_none()
                    && !dst.is_symlink()
                    && paths_refer_to_same_file(src, dst, true)
                    && is_same_entry(src, dst)
                {
                    // Even in force overwrite mode, verify we are not targeting the same entry and return a SameFile error if so
                    return Err(LnError::SameFile(src.to_owned(), dst.to_owned()));
                }
                overwrite_on_conflict = true;
            }
        }
        if let Some(backup) = &backup {
            dest.rename_to(backup).map_err(|e| {
                LnError::IoContext(
                    UIoError::from(e),
                    translate!("ln-cannot-backup", "file" => dst.quote()),
                )
            })?;
        }
    }

    // Resolved once: this can fail independently of the destination.
    let hard_link_src = if settings.symbolic {
        None
    } else if settings.logical && source.is_symlink() {
        Some(fs::canonicalize(&source).map_err(|e| {
            LnError::IoContext(
                UIoError::from(e),
                translate!("ln-failed-to-access", "file" => source.quote()),
            )
        })?)
    } else {
        Some(source.to_path_buf())
    };

    // Link first, matching GNU. Replacing uses a temp name + `renameat`, so
    // `dst` is never briefly free for another user to claim.
    let link_target = hard_link_src.as_deref().unwrap_or(&source);
    let raw = dest.create_link(link_target, settings.symbolic, overwrite_on_conflict);

    let res = raw.map_err(|e| {
        if settings.symbolic {
            LnError::IoContext(
                UIoError::from(e),
                translate!(
                    "ln-failed-to-create-symbolic-link",
                    "dest" => dst.quote()
                ),
            )
        } else if hard_link_src.as_ref().is_some_and(|p| p.is_dir()) {
            LnError::FailedToCreateHardLinkDir(source.to_path_buf())
        } else {
            LnError::IoContext(
                UIoError::from(e),
                translate!(
                    "ln-failed-to-create-hard-link",
                    "source" => source.quote(),
                    "dest" => dst.quote()
                ),
            )
        }
    });

    if let Err(e) = res {
        if let Some(backup) = &backup {
            backup.rename_to(dest).map_err(|e| {
                LnError::IoContext(
                    UIoError::from(e),
                    translate!("ln-cannot-backup", "file" => dst.quote()),
                )
            })?;
        }
        return Err(e);
    }

    if settings.verbose {
        let mut out = stdout();
        write!(out, "{} -> {}", dst.quote(), source.quote())?;
        match backup {
            Some(backup) => writeln!(
                out,
                " ({})",
                translate!("ln-backup", "backup" => backup.path.quote())
            )?,
            None => writeln!(out)?,
        }
    }
    Ok(())
}

/// Where [`link_to`] puts a link or a backup.
struct Dest<'a> {
    path: Cow<'a, Path>,
    #[cfg(all(unix, not(target_os = "redox")))]
    at: Option<InDir<'a>>,
}

/// The entry `name` in the directory `link_files_in_dir` opened, at `dir_path`.
#[cfg(all(unix, not(target_os = "redox")))]
struct InDir<'a> {
    dir: &'a DirFd,
    dir_path: &'a Path,
    name: Cow<'a, OsStr>,
}

impl<'a> Dest<'a> {
    fn by_path(path: Cow<'a, Path>) -> Self {
        Self {
            path,
            #[cfg(all(unix, not(target_os = "redox")))]
            at: None,
        }
    }

    fn exists(&self) -> bool {
        #[cfg(all(unix, not(target_os = "redox")))]
        if let Some(at) = &self.at {
            return at.dir.stat_at(&at.name, SymlinkBehavior::NoFollow).is_ok();
        }
        self.path.is_symlink() || self.path.exists()
    }

    /// Where the backup of this entry goes, in the same directory.
    fn backup(&self, settings: &Settings) -> Option<Self> {
        #[cfg(all(unix, not(target_os = "redox")))]
        if let Some(at) = &self.at {
            let exists = |path: &Path| {
                at.dir
                    .stat_at(path.as_os_str(), SymlinkBehavior::NoFollow)
                    .is_ok()
            };
            let name = backup_control::get_backup_path_with_exists(
                settings.backup,
                Path::new(&at.name),
                &settings.suffix,
                exists,
            )?;
            return Some(Self {
                path: at.dir_path.join(&name).into(),
                at: Some(InDir {
                    name: name.into_os_string().into(),
                    ..*at
                }),
            });
        }
        backup_control::get_backup_path(settings.backup, &self.path, &settings.suffix)
            .map(|path| Self::by_path(path.into()))
    }

    /// Rename this entry to `to`, its backup or the entry it backs up.
    fn rename_to(&self, to: &Self) -> io::Result<()> {
        #[cfg(all(unix, not(target_os = "redox")))]
        if let (Some(from), Some(to)) = (&self.at, &to.at) {
            return rustix::fs::renameat(from.dir, &*from.name, to.dir, &*to.name)
                .map_err(Into::into);
        }
        fs::rename(&self.path, &to.path)
    }

    fn create_link(&self, target: &Path, symbolic: bool, replace: bool) -> io::Result<()> {
        #[cfg(all(unix, not(target_os = "redox")))]
        if let Some(at) = &self.at {
            let create = |dir: &DirFd, name: &OsStr| link_at(target, dir, name, symbolic);
            return if replace {
                at.dir.replace_entry_at(&at.name, create)
            } else {
                create(at.dir, &at.name)
            };
        }
        if replace {
            replace_link(target, &self.path, symbolic)
        } else if symbolic {
            symlink(target, &self.path)
        } else {
            fs::hard_link(target, &self.path)
        }
    }
}

#[cfg(windows)]
pub fn symlink<P1: AsRef<Path>, P2: AsRef<Path>>(src: P1, dst: P2) -> io::Result<()> {
    use std::os::windows::fs::{symlink_dir, symlink_file};
    // Windows does not resolve `/` inside a symlink target, so store it with `\`.
    let src = windows_link_target(src.as_ref());
    if src.is_dir() {
        symlink_dir(src, dst)
    } else {
        symlink_file(src, dst)
    }
}

#[cfg(windows)]
fn windows_link_target(src: &Path) -> PathBuf {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    let wide: Vec<u16> = src
        .as_os_str()
        .encode_wide()
        .map(|c| {
            if c == u16::from(b'/') {
                u16::from(b'\\')
            } else {
                c
            }
        })
        .collect();
    PathBuf::from(OsString::from_wide(&wide))
}

#[cfg(any(unix, target_os = "wasi"))]
pub fn symlink<P1: AsRef<Path>, P2: AsRef<Path>>(src: P1, dst: P2) -> io::Result<()> {
    rustix::fs::symlink(src.as_ref(), dst.as_ref()).map_err(io::Error::from)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn link_target_uses_backslashes() {
        assert_eq!(
            windows_link_target(Path::new("test/dir/file")),
            PathBuf::from("test\\dir\\file")
        );
        assert_eq!(
            windows_link_target(Path::new("..\\up/mixed\\sep/file")),
            PathBuf::from("..\\up\\mixed\\sep\\file")
        );
    }
}
