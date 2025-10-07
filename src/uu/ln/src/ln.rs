// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) srcpath targetpath EEXIST CLOEXEC RDONLY linkat

use clap::{Arg, ArgAction, Command};
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult};
use uucore::fs::{make_path_relative_to, paths_refer_to_same_file};
use uucore::translate;
use uucore::{format_usage, prompt_yes, show_error};

use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io;
use thiserror::Error;

#[cfg(target_os = "android")]
use std::ffi::{CString, OsStr};
#[cfg(target_os = "android")]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::path::{Path, PathBuf};
use uucore::backup_control::{self, BackupMode};
use uucore::fs::{MissingHandling, ResolveMode, canonicalize};
#[cfg(target_os = "android")]
use uucore::libc::{self, O_CLOEXEC, O_DIRECTORY, O_RDONLY};

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

    #[error("{}", translate!("ln-error-extra-operand", "operand" => _0.to_string_lossy(), "program" => _1.clone()))]
    ExtraOperand(OsString, String),
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
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::INTERACTIVE)
                .short('i')
                .long(options::INTERACTIVE)
                .help(translate!("ln-help-interactive"))
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
                        "{}",
                        translate!("ln-error-cannot-stat", "path" => srcpath.quote())
                    );
                    all_successful = false;
                    continue;
                }
            }
        };

        if linked_destinations.contains(&targetpath) {
            // If the target file was already created in this ln call, do not overwrite
            show_error!(
                "{}",
                translate!("ln-error-will-not-overwrite", "target" => targetpath.display(), "source" => srcpath.display())
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

fn refer_to_same_file(src: &Path, dst: &Path, dereference: bool) -> bool {
    #[cfg(unix)]
    {
        let src_meta = if dereference {
            fs::metadata(src)
        } else {
            fs::symlink_metadata(src)
        };
        let dst_meta = if dereference {
            fs::metadata(dst)
        } else {
            fs::symlink_metadata(dst)
        };

        if let (Ok(src_meta), Ok(dst_meta)) = (src_meta, dst_meta) {
            return src_meta.ino() == dst_meta.ino() && src_meta.dev() == dst_meta.dev();
        }
    }

    paths_refer_to_same_file(src, dst, dereference)
}

fn create_hard_link(src: &Path, dst: &Path) -> io::Result<()> {
    match fs::hard_link(src, dst) {
        Ok(()) => Ok(()),
        Err(err) => {
            #[cfg(target_os = "android")]
            {
                if err.kind() == io::ErrorKind::PermissionDenied {
                    match android_hard_link(src, dst) {
                        Ok(()) => return Ok(()),
                        Err(fallback_err) => {
                            if fallback_err.kind() == io::ErrorKind::PermissionDenied {
                                return Err(err);
                            }
                            return Err(fallback_err);
                        }
                    }
                }
            }
            Err(err)
        }
    }
}

#[cfg(target_os = "android")]
fn android_hard_link(src: &Path, dst: &Path) -> io::Result<()> {
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn os_str_to_cstring(value: &OsStr) -> io::Result<CString> {
        CString::new(value.as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains null byte"))
    }

    fn path_to_cstring(path: &Path) -> io::Result<CString> {
        os_str_to_cstring(path.as_os_str())
    }

    fn open_with_flags(path_c: &CString, flags: libc::c_int) -> io::Result<libc::c_int> {
        let fd = unsafe { libc::open(path_c.as_ptr(), flags) };
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(fd)
        }
    }

    fn open_directory(path: &Path) -> io::Result<libc::c_int> {
        let mut candidates = vec![path.to_path_buf()];
        if let Ok(canonical) = fs::canonicalize(path) {
            if canonical != path {
                candidates.push(canonical);
            }
        }

        let mut last_err: Option<io::Error> = None;
        for candidate in candidates {
            let path_c = path_to_cstring(&candidate)?;
            match open_with_flags(&path_c, O_RDONLY | O_DIRECTORY | O_CLOEXEC) {
                Ok(fd) => return Ok(fd),
                Err(err) => {
                    let err_kind = err.kind();
                    last_err = Some(err);
                    if err_kind == io::ErrorKind::PermissionDenied {
                        match open_with_flags(&path_c, libc::O_PATH | O_DIRECTORY | O_CLOEXEC) {
                            Ok(fd) => return Ok(fd),
                            Err(err2) => {
                                last_err = Some(err2);
                            }
                        }
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "failed to open directory for hard link",
            )
        }))
    }

    fn split_path(path: &Path) -> io::Result<(PathBuf, OsString)> {
        let file_name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing file name"))?;
        let parent = path.parent().map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        });
        let dir = parent.unwrap_or_else(|| PathBuf::from("."));
        Ok((dir, file_name.to_os_string()))
    }

    let (src_dir_path, src_name) = split_path(src)?;
    let (dst_dir_path, dst_name) = split_path(dst)?;

    let src_name_c = os_str_to_cstring(src_name.as_os_str())?;
    let dst_name_c = os_str_to_cstring(dst_name.as_os_str())?;

    let src_fd = open_directory(&src_dir_path)?;
    let dst_fd = match open_directory(&dst_dir_path) {
        Ok(fd) => fd,
        Err(e) => {
            unsafe {
                libc::close(src_fd);
            }
            return Err(e);
        }
    };

    let link_result =
        unsafe { libc::linkat(src_fd, src_name_c.as_ptr(), dst_fd, dst_name_c.as_ptr(), 0) };
    let link_error = if link_result == 0 {
        None
    } else {
        Some(io::Error::last_os_error())
    };

    unsafe {
        libc::close(src_fd);
        libc::close(dst_fd);
    }

    if let Some(err) = link_error {
        Err(err)
    } else {
        Ok(())
    }
}

#[cfg(target_os = "android")]
fn try_force_swap_android(src: &Path, dst: &Path) -> io::Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let parent = dst
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut attempt: u32 = 0;
    loop {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp_name = format!(
            ".uu_ln_force_tmp_{}_{}_{attempt}",
            std::process::id(),
            suffix
        );
        let tmp_path = parent.join(&tmp_name);

        match create_hard_link(src, &tmp_path) {
            Ok(()) => match fs::rename(&tmp_path, dst) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let _ = fs::remove_file(&tmp_path);
                    return Err(e);
                }
            },
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                attempt = attempt.wrapping_add(1);
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(target_os = "android")]
fn try_symlink_swap_android(src: &Path, dst: &Path) -> io::Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let parent = dst
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut attempt: u32 = 0;
    loop {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp_name = format!(
            ".uu_ln_symlink_tmp_{}_{}_{attempt}",
            std::process::id(),
            suffix
        );
        let tmp_path = parent.join(&tmp_name);

        match symlink(src, &tmp_path) {
            Ok(()) => match fs::rename(&tmp_path, dst) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let _ = fs::remove_file(&tmp_path);
                    return Err(e);
                }
            },
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                attempt = attempt.wrapping_add(1);
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

#[allow(clippy::cognitive_complexity)]
fn link(src: &Path, dst: &Path, settings: &Settings) -> UResult<()> {
    let mut backup_path = None;
    let mut dst_removed = false;
    let source: Cow<'_, Path> = if settings.relative {
        relative_path(src, dst)
    } else {
        src.into()
    };

    if dst.is_symlink() || dst.exists() {
        backup_path = match settings.backup {
            BackupMode::None => None,
            BackupMode::Simple => Some(simple_backup_path(dst, &settings.suffix)),
            BackupMode::Numbered => Some(numbered_backup_path(dst)),
            BackupMode::Existing => Some(existing_backup_path(dst, &settings.suffix)),
        };
        if settings.backup == BackupMode::Existing && !settings.symbolic {
            // when ln --backup f f, it should detect that it is the same file
            if refer_to_same_file(src, dst, true) {
                return Err(LnError::SameFile(src.to_owned(), dst.to_owned()).into());
            }
        }
        if let Some(ref p) = backup_path {
            fs::rename(dst, p)
                .map_err_context(|| translate!("ln-cannot-backup", "file" => dst.quote()))?;
            dst_removed = true;
        }
        match settings.overwrite {
            OverwriteMode::NoClobber => {}
            OverwriteMode::Interactive => {
                if !prompt_yes!("{}", translate!("ln-prompt-replace", "file" => dst.quote())) {
                    return Err(LnError::SomeLinksFailed.into());
                }

                if fs::remove_file(dst).is_ok() {
                    dst_removed = true;
                }
                // In case of error, don't do anything
            }
            OverwriteMode::Force => {
                if !dst.is_symlink()
                    && (refer_to_same_file(src, dst, true) || refer_to_same_file(src, dst, false))
                {
                    let same_entry = match (
                        canonicalize(src, MissingHandling::Missing, ResolveMode::Physical),
                        canonicalize(dst, MissingHandling::Missing, ResolveMode::Physical),
                    ) {
                        (Ok(src_abs), Ok(dst_abs)) => src_abs == dst_abs,
                        _ => true,
                    };
                    if same_entry {
                        return Err(LnError::SameFile(src.to_owned(), dst.to_owned()).into());
                    }
                    if backup_path.is_none() {
                        // Hard link already points to the same inode; nothing to do
                        return Ok(());
                    }
                }
                #[cfg(not(target_os = "android"))]
                {
                    if fs::remove_file(dst).is_ok() {
                        dst_removed = true;
                    }
                }
                // In case of error, don't do anything
            }
        }
    }

    if settings.symbolic {
        #[cfg(target_os = "android")]
        {
            let mut created_via_symlink_swap = false;

            if matches!(settings.overwrite, OverwriteMode::Force) && !dst_removed {
                if dst.exists() || dst.is_symlink() {
                    if let Ok(()) = try_symlink_swap_android(source.as_ref(), dst) {
                        created_via_symlink_swap = true;
                    }
                }
            }

            if !created_via_symlink_swap {
                symlink(&source, dst)?;
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            symlink(&source, dst)?;
        }
    } else {
        let p = if settings.logical && source.is_symlink() {
            // if we want to have an hard link,
            // source is a symlink and -L is passed
            // we want to resolve the symlink to create the hardlink
            fs::canonicalize(&source)
                .map_err_context(|| translate!("ln-failed-to-access", "file" => source.quote()))?
        } else {
            source.to_path_buf()
        };
        #[cfg(target_os = "android")]
        let mut created_via_swap = false;

        #[cfg(target_os = "android")]
        if matches!(settings.overwrite, OverwriteMode::Force) && !dst_removed {
            if let Ok(()) = try_force_swap_android(&p, dst) {
                dst_removed = true;
                created_via_swap = true;
            }
        }

        if matches!(settings.overwrite, OverwriteMode::Force) && !dst_removed {
            let _ = fs::remove_file(dst);
        }

        #[cfg(target_os = "android")]
        if created_via_swap {
            // nothing more to do
        } else {
            create_hard_link(&p, dst)
                .or_else(|err| {
                    if err.kind() == io::ErrorKind::PermissionDenied
                        && (refer_to_same_file(&p, dst, true) || refer_to_same_file(&p, dst, false))
                    {
                        Ok(())
                    } else {
                        Err(err)
                    }
                })
                .map_err_context(|| {
                    translate!(
                        "ln-failed-to-create-hard-link",
                        "source" => source.quote(),
                        "dest" => dst.quote()
                    )
                })?;
        }

        #[cfg(not(target_os = "android"))]
        {
            create_hard_link(&p, dst).map_err_context(|| {
                translate!("ln-failed-to-create-hard-link", "source" => source.quote(), "dest" => dst.quote())
            })?;
        }
    }

    if settings.verbose {
        print!("{} -> {}", dst.quote(), source.quote());
        match backup_path {
            Some(path) => println!(" ({})", translate!("ln-backup", "backup" => path.quote())),
            None => println!(),
        }
    }
    Ok(())
}

fn simple_backup_path(path: &Path, suffix: &OsString) -> PathBuf {
    let mut file_name = path.file_name().unwrap_or_default().to_os_string();
    file_name.push(suffix);
    path.with_file_name(file_name)
}

fn numbered_backup_path(path: &Path) -> PathBuf {
    let mut i: u64 = 1;
    loop {
        let new_path = simple_backup_path(path, &OsString::from(format!(".~{i}~")));
        if !new_path.exists() {
            return new_path;
        }
        i += 1;
    }
}

fn existing_backup_path(path: &Path, suffix: &OsString) -> PathBuf {
    let test_path = simple_backup_path(path, &OsString::from(".~1~"));
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
