// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) Chmoder cmode fmode fperm fref ugoa RFILE RFILE's

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{ExitCode, UError, UResult, USimpleError, UUsageError, set_exit_code};
use uucore::fs::display_permissions_unix;
use uucore::libc::mode_t;
use uucore::mode;
use uucore::perms::{TraverseSymlinks, configure_symlink_and_recursion};

#[cfg(target_os = "linux")]
use uucore::safe_traversal::DirFd;
use uucore::{format_usage, show, show_error};

use uucore::translate;

#[derive(Debug, Error)]
enum ChmodError {
    #[error("{}", translate!("chmod-error-cannot-stat", "file" => _0.quote()))]
    CannotStat(PathBuf),
    #[error("{}", translate!("chmod-error-dangling-symlink", "file" => _0.quote()))]
    DanglingSymlink(PathBuf),
    #[error("{}", translate!("chmod-error-no-such-file", "file" => _0.quote()))]
    NoSuchFile(PathBuf),
    #[error("{}", translate!("chmod-error-preserve-root", "file" => _0.quote()))]
    PreserveRoot(PathBuf),
    #[error("{}", translate!("chmod-error-permission-denied", "file" => _0.quote()))]
    PermissionDenied(PathBuf),
    #[error("{}", translate!("chmod-error-new-permissions", "file" => _0.maybe_quote(), "actual" => _1.clone(), "expected" => _2.clone()))]
    NewPermissions(PathBuf, String, String),
}

impl UError for ChmodError {}

mod options {
    pub const HELP: &str = "help";
    pub const CHANGES: &str = "changes";
    pub const QUIET: &str = "quiet"; // visible_alias("silent")
    pub const VERBOSE: &str = "verbose";
    pub const NO_PRESERVE_ROOT: &str = "no-preserve-root";
    pub const PRESERVE_ROOT: &str = "preserve-root";
    pub const REFERENCE: &str = "RFILE";
    pub const RECURSIVE: &str = "recursive";
    pub const MODE: &str = "MODE";
    pub const FILE: &str = "FILE";
}

/// Extract negative modes (starting with '-') from the rest of the arguments.
///
/// This is mainly required for GNU compatibility, where "non-positional negative" modes are used
/// as the actual positional MODE. Some examples of these cases are:
/// * "chmod -w -r file", which is the same as "chmod -w,-r file"
/// * "chmod -w file -r", which is the same as "chmod -w,-r file"
///
/// These can currently not be handled by clap.
/// Therefore it might be possible that a pseudo MODE is inserted to pass clap parsing.
/// The pseudo MODE is later replaced by the extracted (and joined) negative modes.
fn extract_negative_modes(mut args: impl uucore::Args) -> (Option<String>, Vec<OsString>) {
    // we look up the args until "--" is found
    // "-mode" will be extracted into parsed_cmode_vec
    let (parsed_cmode_vec, pre_double_hyphen_args): (Vec<OsString>, Vec<OsString>) =
        args.by_ref().take_while(|a| a != "--").partition(|arg| {
            let arg = if let Some(arg) = arg.to_str() {
                arg.to_string()
            } else {
                return false;
            };
            arg.len() >= 2
                && arg.starts_with('-')
                && matches!(
                    arg.chars().nth(1).unwrap(),
                    'r' | 'w' | 'x' | 'X' | 's' | 't' | 'u' | 'g' | 'o' | '0'..='7'
                )
        });

    let mut clean_args = Vec::new();
    if !parsed_cmode_vec.is_empty() {
        // we need a pseudo cmode for clap, which won't be used later.
        // this is required because clap needs the default "chmod MODE FILE" scheme.
        clean_args.push("w".into());
    }
    clean_args.extend(pre_double_hyphen_args);

    if let Some(arg) = args.next() {
        // as there is still something left in the iterator, we previously consumed the "--"
        // -> add it to the args again
        clean_args.push("--".into());
        clean_args.push(arg);
    }
    clean_args.extend(args);

    let parsed_cmode = Some(
        parsed_cmode_vec
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<&str>>()
            .join(","),
    )
    .filter(|s| !s.is_empty());
    (parsed_cmode, clean_args)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let (parsed_cmode, args) = extract_negative_modes(args.skip(1)); // skip binary name
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let changes = matches.get_flag(options::CHANGES);
    let quiet = matches.get_flag(options::QUIET);
    let verbose = matches.get_flag(options::VERBOSE);
    let preserve_root = matches.get_flag(options::PRESERVE_ROOT);
    let fmode = match matches.get_one::<OsString>(options::REFERENCE) {
        Some(fref) => match fs::metadata(fref) {
            Ok(meta) => Some(meta.mode() & 0o7777),
            Err(_) => {
                return Err(ChmodError::CannotStat(fref.into()).into());
            }
        },
        None => None,
    };

    let modes = matches.get_one::<String>(options::MODE);
    let cmode = if let Some(parsed_cmode) = parsed_cmode {
        parsed_cmode
    } else {
        modes.unwrap().to_owned() // modes is required
    };
    let mut files: Vec<OsString> = matches
        .get_many::<OsString>(options::FILE)
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let cmode = if fmode.is_some() {
        // "--reference" and MODE are mutually exclusive
        // if "--reference" was used MODE needs to be interpreted as another FILE
        // it wasn't possible to implement this behavior directly with clap
        files.push(OsString::from(cmode));
        None
    } else {
        Some(cmode)
    };

    if files.is_empty() {
        return Err(UUsageError::new(
            1,
            translate!("chmod-error-missing-operand"),
        ));
    }

    let (recursive, dereference, traverse_symlinks) =
        configure_symlink_and_recursion(&matches, TraverseSymlinks::First)?;

    let chmoder = Chmoder {
        changes,
        quiet,
        verbose,
        preserve_root,
        recursive,
        fmode,
        cmode,
        traverse_symlinks,
        dereference,
    };

    chmoder.chmod(&files)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("chmod-about"))
        .override_usage(format_usage(&translate!("chmod-usage")))
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .args_override_self(true)
        .infer_long_args(true)
        .no_binary_name(true)
        .disable_help_flag(true)
        .after_help(translate!("chmod-after-help"))
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("chmod-help-print-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::CHANGES)
                .long(options::CHANGES)
                .short('c')
                .help(translate!("chmod-help-changes"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::QUIET)
                .long(options::QUIET)
                .visible_alias("silent")
                .short('f')
                .help(translate!("chmod-help-quiet"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .long(options::VERBOSE)
                .short('v')
                .help(translate!("chmod-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_PRESERVE_ROOT)
                .long(options::NO_PRESERVE_ROOT)
                .help(translate!("chmod-help-no-preserve-root"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRESERVE_ROOT)
                .long(options::PRESERVE_ROOT)
                .help(translate!("chmod-help-preserve-root"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .long(options::RECURSIVE)
                .short('R')
                .help(translate!("chmod-help-recursive"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .long("reference")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString))
                .help(translate!("chmod-help-reference")),
        )
        .arg(
            Arg::new(options::MODE).required_unless_present(options::REFERENCE),
            // It would be nice if clap could parse with delimiter, e.g. "g-x,u+x",
            // however .multiple_occurrences(true) cannot be used here because FILE already needs that.
            // Only one positional argument with .multiple_occurrences(true) set is allowed per command
        )
        .arg(
            Arg::new(options::FILE)
                .required_unless_present(options::MODE)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(clap::value_parser!(OsString)),
        )
        // Add common arguments with chgrp, chown & chmod
        .args(uucore::perms::common_args())
}

struct Chmoder {
    changes: bool,
    quiet: bool,
    verbose: bool,
    preserve_root: bool,
    recursive: bool,
    fmode: Option<u32>,
    cmode: Option<String>,
    traverse_symlinks: TraverseSymlinks,
    dereference: bool,
}

impl Chmoder {
    /// Calculate the new mode based on the current mode and the chmod specification.
    /// Returns (`new_mode`, `naively_expected_new_mode`) for symbolic modes, or (`new_mode`, `new_mode`) for numeric/reference modes.
    fn calculate_new_mode(&self, current_mode: u32, is_dir: bool) -> UResult<(u32, u32)> {
        match self.fmode {
            Some(mode) => Ok((mode, mode)),
            None => {
                let cmode_unwrapped = self.cmode.clone().unwrap();
                let mut new_mode = current_mode;
                let mut naively_expected_new_mode = current_mode;

                for mode in cmode_unwrapped.split(',') {
                    let result = if mode.chars().any(|c| c.is_ascii_digit()) {
                        mode::parse_numeric(new_mode, mode, is_dir).map(|v| (v, v))
                    } else {
                        mode::parse_symbolic(new_mode, mode, mode::get_umask(), is_dir).map(|m| {
                            // calculate the new mode as if umask was 0
                            let naive_mode =
                                mode::parse_symbolic(naively_expected_new_mode, mode, 0, is_dir)
                                    .unwrap(); // we know that mode must be valid, so this cannot fail
                            (m, naive_mode)
                        })
                    };

                    match result {
                        Ok((mode, naive_mode)) => {
                            new_mode = mode;
                            naively_expected_new_mode = naive_mode;
                        }
                        Err(f) => {
                            return if self.quiet {
                                Err(ExitCode::new(1))
                            } else {
                                Err(USimpleError::new(1, f))
                            };
                        }
                    }
                }
                Ok((new_mode, naively_expected_new_mode))
            }
        }
    }

    /// Report permission changes based on verbose and changes flags
    fn report_permission_change(&self, file_path: &Path, old_mode: u32, new_mode: u32) {
        if self.verbose || self.changes {
            let current_permissions = display_permissions_unix(old_mode as mode_t, false);
            let new_permissions = display_permissions_unix(new_mode as mode_t, false);

            if new_mode != old_mode {
                println!(
                    "mode of {} changed from {:04o} ({}) to {:04o} ({})",
                    file_path.quote(),
                    old_mode,
                    current_permissions,
                    new_mode,
                    new_permissions
                );
            } else if self.verbose {
                println!(
                    "mode of {} retained as {:04o} ({})",
                    file_path.quote(),
                    old_mode,
                    current_permissions
                );
            }
        }
    }

    /// Handle symlinks during directory traversal based on traversal mode
    #[cfg(not(target_os = "linux"))]
    fn handle_symlink_during_traversal(
        &self,
        path: &Path,
        is_command_line_arg: bool,
    ) -> UResult<()> {
        let should_follow_symlink = match self.traverse_symlinks {
            TraverseSymlinks::All => true,
            TraverseSymlinks::First => is_command_line_arg,
            TraverseSymlinks::None => false,
        };

        if !should_follow_symlink {
            return self.chmod_file_internal(path, false);
        }

        match fs::metadata(path) {
            Ok(meta) if meta.is_dir() => self.walk_dir_with_context(path, false),
            Ok(_) => {
                // It's a file symlink, chmod it
                self.chmod_file(path)
            }
            Err(_) => {
                // Dangling symlink, chmod it without dereferencing
                self.chmod_file_internal(path, false)
            }
        }
    }

    fn chmod(&self, files: &[OsString]) -> UResult<()> {
        let mut r = Ok(());

        for filename in files {
            let file = Path::new(filename);
            if !file.exists() {
                if file.is_symlink() {
                    if !self.dereference && !self.recursive {
                        // The file is a symlink and we should not follow it
                        // Don't try to change the mode of the symlink itself
                        continue;
                    }
                    if self.recursive && self.traverse_symlinks == TraverseSymlinks::None {
                        continue;
                    }

                    if !self.quiet {
                        show!(ChmodError::DanglingSymlink(filename.into()));
                        set_exit_code(1);
                    }

                    if self.verbose {
                        println!(
                            "{}",
                            translate!("chmod-verbose-failed-dangling", "file" => filename.quote())
                        );
                    }
                } else if !self.quiet {
                    show!(ChmodError::NoSuchFile(filename.into()));
                }
                // GNU exits with exit code 1 even if -q or --quiet are passed
                // So we set the exit code, because it hasn't been set yet if `self.quiet` is true.
                set_exit_code(1);
                continue;
            } else if !self.dereference && file.is_symlink() {
                // The file is a symlink and we should not follow it
                // chmod 755 --no-dereference a/link
                // should not change the permissions in this case
                continue;
            }
            if self.recursive && self.preserve_root && file == Path::new("/") {
                return Err(ChmodError::PreserveRoot("/".into()).into());
            }
            if self.recursive {
                r = self.walk_dir_with_context(file, true);
            } else {
                r = self.chmod_file(file).and(r);
            }
        }
        r
    }

    #[cfg(not(target_os = "linux"))]
    fn walk_dir_with_context(&self, file_path: &Path, is_command_line_arg: bool) -> UResult<()> {
        let mut r = self.chmod_file(file_path);

        // Determine whether to traverse symlinks based on context and traversal mode
        let should_follow_symlink = match self.traverse_symlinks {
            TraverseSymlinks::All => true,
            TraverseSymlinks::First => is_command_line_arg, // Only follow symlinks that are command line args
            TraverseSymlinks::None => false,
        };

        // If the path is a directory (or we should follow symlinks), recurse into it
        if (!file_path.is_symlink() || should_follow_symlink) && file_path.is_dir() {
            for dir_entry in file_path.read_dir()? {
                let path = match dir_entry {
                    Ok(entry) => entry.path(),
                    Err(err) => {
                        r = r.and(Err(err.into()));
                        continue;
                    }
                };
                if path.is_symlink() {
                    r = self.handle_symlink_during_recursion(&path).and(r);
                } else {
                    r = self.walk_dir_with_context(path.as_path(), false).and(r);
                }
            }
        }
        r
    }

    #[cfg(target_os = "linux")]
    fn walk_dir_with_context(&self, file_path: &Path, is_command_line_arg: bool) -> UResult<()> {
        let mut r = self.chmod_file(file_path);

        // Determine whether to traverse symlinks based on context and traversal mode
        let should_follow_symlink = match self.traverse_symlinks {
            TraverseSymlinks::All => true,
            TraverseSymlinks::First => is_command_line_arg, // Only follow symlinks that are command line args
            TraverseSymlinks::None => false,
        };

        // If the path is a directory (or we should follow symlinks), recurse into it using safe traversal
        if (!file_path.is_symlink() || should_follow_symlink) && file_path.is_dir() {
            match DirFd::open(file_path) {
                Ok(dir_fd) => {
                    r = self.safe_traverse_dir(&dir_fd, file_path).and(r);
                }
                Err(err) => {
                    // Handle permission denied errors with proper file path context
                    if err.kind() == std::io::ErrorKind::PermissionDenied {
                        r = r.and(Err(ChmodError::PermissionDenied(file_path.into()).into()));
                    } else {
                        r = r.and(Err(err.into()));
                    }
                }
            }
        }
        r
    }

    #[cfg(target_os = "linux")]
    fn safe_traverse_dir(&self, dir_fd: &DirFd, dir_path: &Path) -> UResult<()> {
        let mut r = Ok(());

        let entries = dir_fd.read_dir()?;

        // Determine if we should follow symlinks (doesn't depend on entry_name)
        let should_follow_symlink = self.traverse_symlinks == TraverseSymlinks::All;

        for entry_name in entries {
            let entry_path = dir_path.join(&entry_name);

            let dir_meta = dir_fd.metadata_at(&entry_name, should_follow_symlink);
            let Ok(meta) = dir_meta else {
                // Handle permission denied with proper file path context
                let e = dir_meta.unwrap_err();
                let error = if e.kind() == std::io::ErrorKind::PermissionDenied {
                    ChmodError::PermissionDenied(entry_path).into()
                } else {
                    e.into()
                };
                r = r.and(Err(error));
                continue;
            };

            if entry_path.is_symlink() {
                r = self
                    .handle_symlink_during_safe_recursion(&entry_path, dir_fd, &entry_name)
                    .and(r);
            } else {
                // For regular files and directories, chmod them
                r = self
                    .safe_chmod_file(&entry_path, dir_fd, &entry_name, meta.mode() & 0o7777)
                    .and(r);

                // Recurse into subdirectories using the existing directory fd
                if meta.is_dir() {
                    match dir_fd.open_subdir(&entry_name) {
                        Ok(child_dir_fd) => {
                            r = self.safe_traverse_dir(&child_dir_fd, &entry_path).and(r);
                        }
                        Err(err) => {
                            let error = if err.kind() == std::io::ErrorKind::PermissionDenied {
                                ChmodError::PermissionDenied(entry_path).into()
                            } else {
                                err.into()
                            };
                            r = r.and(Err(error));
                        }
                    }
                }
            }
        }
        r
    }

    #[cfg(target_os = "linux")]
    fn handle_symlink_during_safe_recursion(
        &self,
        path: &Path,
        dir_fd: &DirFd,
        entry_name: &std::ffi::OsStr,
    ) -> UResult<()> {
        // During recursion, determine behavior based on traversal mode
        match self.traverse_symlinks {
            TraverseSymlinks::All => {
                // Follow all symlinks during recursion
                // Check if the symlink target is a directory, but handle dangling symlinks gracefully
                match fs::metadata(path) {
                    Ok(meta) if meta.is_dir() => self.walk_dir_with_context(path, false),
                    Ok(meta) => {
                        // It's a file symlink, chmod it using safe traversal
                        self.safe_chmod_file(path, dir_fd, entry_name, meta.mode() & 0o7777)
                    }
                    Err(_) => {
                        // Dangling symlink, chmod it without dereferencing
                        self.chmod_file_internal(path, false)
                    }
                }
            }
            TraverseSymlinks::First | TraverseSymlinks::None => {
                // Don't follow symlinks encountered during recursion
                // For these symlinks, don't dereference them even if dereference is normally true
                self.chmod_file_internal(path, false)
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn safe_chmod_file(
        &self,
        file_path: &Path,
        dir_fd: &DirFd,
        entry_name: &std::ffi::OsStr,
        current_mode: u32,
    ) -> UResult<()> {
        // Calculate the new mode using the helper method
        let (new_mode, _) = self.calculate_new_mode(current_mode, file_path.is_dir())?;

        // Use safe traversal to change the mode
        let follow_symlinks = self.dereference;
        if let Err(_e) = dir_fd.chmod_at(entry_name, new_mode, follow_symlinks) {
            if self.verbose {
                println!(
                    "failed to change mode of {} to {:o}",
                    file_path.quote(),
                    new_mode
                );
            }
            return Err(ChmodError::PermissionDenied(file_path.into()).into());
        }

        // Report the change using the helper method
        self.report_permission_change(file_path, current_mode, new_mode);

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn handle_symlink_during_recursion(&self, path: &Path) -> UResult<()> {
        // Use the common symlink handling logic
        self.handle_symlink_during_traversal(path, false)
    }

    fn chmod_file(&self, file: &Path) -> UResult<()> {
        self.chmod_file_internal(file, self.dereference)
    }

    fn chmod_file_internal(&self, file: &Path, dereference: bool) -> UResult<()> {
        use uucore::perms::get_metadata;

        let metadata = get_metadata(file, dereference);

        let fperm = match metadata {
            Ok(meta) => meta.mode() & 0o7777,
            Err(err) => {
                // Handle dangling symlinks or other errors
                return if file.is_symlink() && !dereference {
                    if self.verbose {
                        println!(
                            "neither symbolic link {} nor referent has been changed",
                            file.quote()
                        );
                    }
                    Ok(()) // Skip dangling symlinks
                } else if err.kind() == std::io::ErrorKind::PermissionDenied {
                    Err(ChmodError::PermissionDenied(file.into()).into())
                } else {
                    Err(ChmodError::CannotStat(file.into()).into())
                };
            }
        };

        // Calculate the new mode using the helper method
        let (new_mode, naively_expected_new_mode) =
            self.calculate_new_mode(fperm, file.is_dir())?;

        // Determine how to apply the permissions
        match self.fmode {
            Some(mode) => self.change_file(fperm, mode, file)?,
            None => {
                // Special handling for symlinks when not dereferencing
                if file.is_symlink() && !dereference {
                    // TODO: On most Unix systems, symlink permissions are ignored by the kernel,
                    // so changing them has no effect. We skip this operation for compatibility.
                    // Note that "chmod without dereferencing" effectively does nothing on symlinks.
                    if self.verbose {
                        println!(
                            "neither symbolic link {} nor referent has been changed",
                            file.quote()
                        );
                    }
                } else {
                    self.change_file(fperm, new_mode, file)?;
                }
                // if a permission would have been removed if umask was 0, but it wasn't because umask was not 0, print an error and fail
                if (new_mode & !naively_expected_new_mode) != 0 {
                    return Err(ChmodError::NewPermissions(
                        file.into(),
                        display_permissions_unix(new_mode as mode_t, false),
                        display_permissions_unix(naively_expected_new_mode as mode_t, false),
                    )
                    .into());
                }
            }
        }

        Ok(())
    }

    fn change_file(&self, fperm: u32, mode: u32, file: &Path) -> Result<(), i32> {
        if fperm == mode {
            // Use the helper method for consistent reporting
            self.report_permission_change(file, fperm, mode);
            Ok(())
        } else if let Err(err) = fs::set_permissions(file, fs::Permissions::from_mode(mode)) {
            if !self.quiet {
                show_error!("{err}");
            }
            if self.verbose {
                println!(
                    "failed to change mode of file {} from {fperm:04o} ({}) to {mode:04o} ({})",
                    file.quote(),
                    display_permissions_unix(fperm as mode_t, false),
                    display_permissions_unix(mode as mode_t, false)
                );
            }
            Err(1)
        } else {
            // Use the helper method for consistent reporting
            self.report_permission_change(file, fperm, mode);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_negative_modes() {
        // "chmod -w -r file" becomes "chmod -w,-r file". clap does not accept "-w,-r" as MODE.
        // Therefore, "w" is added as pseudo mode to pass clap.
        let (c, a) = extract_negative_modes(["-w", "-r", "file"].iter().map(OsString::from));
        assert_eq!(c, Some("-w,-r".to_string()));
        assert_eq!(a, ["w", "file"]);

        // "chmod -w file -r" becomes "chmod -w,-r file". clap does not accept "-w,-r" as MODE.
        // Therefore, "w" is added as pseudo mode to pass clap.
        let (c, a) = extract_negative_modes(["-w", "file", "-r"].iter().map(OsString::from));
        assert_eq!(c, Some("-w,-r".to_string()));
        assert_eq!(a, ["w", "file"]);

        // "chmod -w -- -r file" becomes "chmod -w -r file", where "-r" is interpreted as file.
        // Again, "w" is needed as pseudo mode.
        let (c, a) = extract_negative_modes(["-w", "--", "-r", "f"].iter().map(OsString::from));
        assert_eq!(c, Some("-w".to_string()));
        assert_eq!(a, ["w", "--", "-r", "f"]);

        // "chmod -- -r file" becomes "chmod -r file".
        let (c, a) = extract_negative_modes(["--", "-r", "file"].iter().map(OsString::from));
        assert_eq!(c, None);
        assert_eq!(a, ["--", "-r", "file"]);
    }
}
