// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) Chmoder cmode fmode fperm fref ugoa RFILE RFILE's

#![cfg(unix)]

use clap::{Arg, ArgAction, Command};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{
    ExitCode, UError, UResult, USimpleError, UUsageError, set_exit_code, strip_errno,
};
use uucore::fs::{FileInformation, display_permissions_unix, path_is_root_dir};
use uucore::mode;
use uucore::perms::{TraverseSymlinks, configure_symlink_and_recursion};

#[cfg(not(target_os = "redox"))]
use uucore::safe_traversal::{DirFd, SymlinkBehavior};
use uucore::{format_usage, show, show_error};

use uucore::translate;

#[derive(Debug, thiserror::Error)]
enum ChmodError {
    #[error("{}", translate!("perms-cannot-access", "file" => _0.quote(), "error" => strip_errno(_1)))]
    CannotStat(PathBuf, std::io::Error),
    #[error("{}", translate!("chmod-error-reference-stat", "file" => _0.quote(), "error" => strip_errno(_1)))]
    ReferenceStat(PathBuf, std::io::Error),
    #[error("{}", translate!("chmod-error-dangling-symlink", "file" => _0.quote()))]
    DanglingSymlink(PathBuf),
    #[error("{}", translate!("chmod-error-no-such-file", "file" => _0.quote()))]
    NoSuchFile(PathBuf),
    #[error("{}", translate!("chmod-error-preserve-root", "file" => _0.quote()))]
    PreserveRoot(PathBuf),
    #[error("{}", translate!("chmod-error-preserve-root-same-as", "file" => _0.quote()))]
    PreserveRootSameAs(PathBuf),
    #[error("{}", translate!("chmod-error-permission-denied", "file" => _0.quote()))]
    PermissionDenied(PathBuf),
    #[error("{}", translate!("chmod-error-cannot-read-directory", "file" => _0.quote(), "err" => strip_errno(_1)))]
    CannotReadDirectory(PathBuf, std::io::Error),
    #[error("{}", translate!("chmod-error-new-permissions", "file" => _0.maybe_quote(), "actual" => _1, "expected" => _2))]
    NewPermissions(PathBuf, String, String),
    #[error("{}", translate!("chmod-error-changing-permissions", "file" => _0.quote(), "err" => strip_errno(_1)))]
    ChangingPermissions(PathBuf, std::io::Error),
}

impl UError for ChmodError {}

// Keep a small window of directory descriptors, rather than one per ancestor.
// The operand's descriptor and temporary descriptors are additional to this cache.
// Keep the operand open as the anchor for reopening evicted frames.
#[cfg(not(target_os = "redox"))]
const MAX_CACHED_DIR_FDS: usize = 16;

#[cfg(not(target_os = "redox"))]
struct TraversalFrame {
    name: OsString,
    info: FileInformation,
    entries: std::vec::IntoIter<OsString>,
    fd: Option<DirFd>,
}

#[cfg(not(target_os = "redox"))]
fn reopen_directory(
    root_fd: &DirFd,
    frames: &[TraversalFrame],
    symlink_behavior: SymlinkBehavior,
) -> std::io::Result<DirFd> {
    let anchor = frames
        .iter()
        .rposition(|frame| frame.fd.is_some())
        .unwrap_or(0);
    let anchor_fd = frames[anchor].fd.as_ref().unwrap_or(root_fd);
    let mut fd = anchor_fd.open_subdir(std::ffi::OsStr::new("."), SymlinkBehavior::NoFollow)?;
    for frame in &frames[anchor + 1..] {
        fd = fd.open_subdir(&frame.name, symlink_behavior)?;
        // A rename or replacement must not redirect a suspended traversal into
        // another directory, even when -L permits following symbolic links.
        if FileInformation::from_file(&fd)? != frame.info {
            return Err(std::io::Error::from_raw_os_error(uucore::libc::ESTALE));
        }
    }
    Ok(fd)
}

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
fn extract_negative_modes(args: impl uucore::Args) -> (Option<String>, Vec<OsString>, Vec<usize>) {
    let is_negative_mode = |arg: &OsString| {
        let Some(arg) = arg.to_str() else {
            return false;
        };
        // A mode can also be "-=r", "-+x" or "-,u+x". No option starts with '=', '+' or ',',
        // and a second '-' (a long option) is deliberately not listed.
        arg.len() >= 2
            && arg.starts_with('-')
            && matches!(
                arg.chars().nth(1).unwrap(),
                'r' | 'w' | 'x' | 'X' | 's' | 't' | 'u' | 'g' | 'o' | '=' | '+' | ',' | '0'..='7'
            )
    };

    // we look up the args until "--" is found
    // "-mode" will be extracted into parsed_cmode_vec
    let mut args = args.enumerate();
    let mut parsed_cmode_vec: Vec<OsString> = Vec::new();
    // Where each extracted mode sat in the original argument list, so that a
    // diagnostic can point back at it.
    let mut mode_indices = Vec::new();
    let mut pre_double_hyphen_args = Vec::new();
    let mut tail = Vec::new();
    while let Some((index, arg)) = args.next() {
        if arg == "--" {
            if let Some((_, next)) = args.next() {
                // as there is still something left in the iterator, we previously consumed the "--"
                // -> add it to the args again
                tail.push(OsString::from("--"));
                tail.push(next);
                tail.extend(args.map(|(_, arg)| arg));
            }
            break;
        }
        if is_negative_mode(&arg) {
            mode_indices.push(index);
            parsed_cmode_vec.push(arg);
        } else {
            pre_double_hyphen_args.push(arg);
        }
    }

    let mut clean_args = Vec::new();
    if !parsed_cmode_vec.is_empty() {
        // we need a pseudo cmode for clap, which won't be used later.
        // this is required because clap needs the default "chmod MODE FILE" scheme.
        clean_args.push("w".into());
    }
    clean_args.extend(pre_double_hyphen_args);
    clean_args.extend(tail);

    let parsed_cmode = Some(
        parsed_cmode_vec
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<&str>>()
            .join(","),
    )
    .filter(|s| !s.is_empty());
    (parsed_cmode, clean_args, mode_indices)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<OsString> = args.skip(1).collect(); // skip binary name
    // Kept for the caret in mode diagnostics, which needs the mode as typed,
    // before `extract_negative_modes` replaces a negative mode by a pseudo one.
    let mode_args = uucore::diagnostics::capture(&args);
    let (parsed_cmode, args, mode_indices) = extract_negative_modes(args.into_iter());
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let changes = matches.get_flag(options::CHANGES);
    let quiet = matches.get_flag(options::QUIET);
    let verbose = matches.get_flag(options::VERBOSE);
    let preserve_root = matches.get_flag(options::PRESERVE_ROOT);
    let fmode = match matches.get_one::<OsString>(options::REFERENCE) {
        Some(fref) => Some(
            fs::metadata(fref)
                .map(|meta| meta.mode() & 0o7777)
                .map_err(|err| ChmodError::ReferenceStat(fref.into(), err))?,
        ),
        None => None,
    };

    let modes = matches.get_one::<String>(options::MODE);
    // Whether the mode reached us as an option-like operand ("chmod -w f") rather than as an
    // ordinary positional operand ("chmod -- -w f"). This decides whether the umask diagnostic
    // below is emitted; see the comment on `option_like_mode`.
    let option_like_mode = parsed_cmode.is_some();
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
        // if "--reference" was used MODE needs to be interpreted as another FILE,
        // it is the first FILE, keep the command-line order
        // it wasn't possible to implement this behavior directly with clap
        files.insert(0, OsString::from(cmode));
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
        option_like_mode,
        traverse_symlinks,
        dereference,
        args: mode_args,
        mode_indices,
    };

    chmoder.chmod(&files)
}

pub fn uu_app() -> Command {
    Command::new("chmod")
        .version(uucore::crate_version!())
        .about(translate!("chmod-about"))
        .override_usage(format_usage(&translate!("chmod-usage")))
        .help_template(uucore::localized_help_template("chmod"))
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
    /// Set when the mode was given as an option-like operand, e.g. `chmod -w f`, instead of as a
    /// plain positional operand, e.g. `chmod -- -w f`. GNU only reports a mode whose effect was
    /// curtailed by the umask for the first spelling: the second one is unambiguous, so there is
    /// nothing to warn about.
    option_like_mode: bool,
    traverse_symlinks: TraverseSymlinks,
    dereference: bool,
    /// The arguments as typed, kept only when a diagnostic may be rendered.
    args: Option<Vec<OsString>>,
    /// Where each extracted negative mode sat in `args`, in the order they
    /// were joined into `cmode`. Empty unless `option_like_mode` is set.
    mode_indices: Vec<usize>,
}

impl Chmoder {
    /// Calculate the new mode based on the current mode and the chmod specification.
    /// Returns (`new_mode`, `naively_expected_new_mode`) for symbolic modes, or (`new_mode`, `new_mode`) for numeric/reference modes.
    fn calculate_new_mode(&self, current_mode: u32, is_dir: bool) -> UResult<(u32, u32)> {
        if let Some(mode) = self.fmode {
            Ok((mode, mode))
        } else {
            let cmode_unwrapped = self.cmode.clone().unwrap();
            let mut new_mode = current_mode;
            let mut naively_expected_new_mode = current_mode;

            // Where the clause being parsed starts inside `cmode_unwrapped`, so
            // that an error can be pointed back at it.
            let mut offset = 0;
            for mode in cmode_unwrapped.split(',') {
                let clause_start = offset;
                offset += mode.len() + 1; // past the clause and its comma

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
                    Err(error) => {
                        if self.quiet {
                            return Err(ExitCode::new(1));
                        }
                        if let Some(args) = &self.args
                            && let Some((index, operand, offset)) =
                                self.locate_clause(args, &cmode_unwrapped, clause_start)
                            && error.render_at(args, index, &operand, offset, &error.to_string())
                        {
                            // The diagnostic is already on stderr; exit quietly.
                            return Err(ExitCode::new(1));
                        }
                        return Err(USimpleError::new(1, error.to_string()));
                    }
                }
            }
            Ok((new_mode, naively_expected_new_mode))
        }
    }

    /// Map `clause_start` — an offset into `cmode`, the joined mode string —
    /// back to the argument the failing clause came from.
    ///
    /// Returns the argument's index in `args`, the mode text that argument
    /// carries, and the clause's offset inside that text. A mode given as a
    /// plain positional is its own argument; extracted negative modes
    /// (`chmod -w -x f`) were joined with commas, so the offset is walked back
    /// through the recorded [`Chmoder::mode_indices`].
    fn locate_clause(
        &self,
        args: &[OsString],
        cmode: &str,
        clause_start: usize,
    ) -> Option<(usize, String, usize)> {
        if !self.option_like_mode {
            let index = args.iter().position(|arg| arg == cmode)?;
            return Some((index, cmode.to_string(), clause_start));
        }
        let mut start = 0;
        for &index in &self.mode_indices {
            let element = args.get(index)?.to_str()?;
            // The `+ 1` is the comma the join put after this element.
            if clause_start < start + element.len() + 1 {
                return Some((index, element.to_string(), clause_start - start));
            }
            start += element.len() + 1;
        }
        None
    }

    /// Report permission changes based on verbose and changes flags
    fn report_permission_change(&self, file_path: &Path, old_mode: u32, new_mode: u32) {
        if self.verbose || self.changes {
            let current_permissions = display_permissions_unix(old_mode, false);
            let new_permissions = display_permissions_unix(new_mode, false);

            if new_mode != old_mode {
                println!(
                    "mode of {} changed from {old_mode:04o} ({current_permissions}) to {new_mode:04o} ({new_permissions})",
                    file_path.quote(),
                );
            } else if self.verbose {
                println!(
                    "mode of {} retained as {old_mode:04o} ({current_permissions})",
                    file_path.quote(),
                );
            }
        }
    }

    /// Report when the umask kept bits a bare mode like -w asked to drop
    fn report_umask_conflict(&self, file: &Path, new_mode: u32, naively_expected_new_mode: u32) {
        // A bare mode such as `-w` is umask-relative, so the umask can keep permissions that
        // the user asked to drop. GNU reports that as an error, but only when the mode was
        // written in the option-like form (`chmod -w f`), where it doubles as a hint that the
        // argument was consumed as a mode. After `--` the operand is unambiguous and GNU stays
        // silent, so the diagnostic is suppressed here too.
        if self.fmode.is_none()
            && self.option_like_mode
            && (new_mode & !naively_expected_new_mode) != 0
        {
            // Shown even under `-f`, as GNU does: it is about the mode as
            // typed, not about this file.
            show!(ChmodError::NewPermissions(
                file.into(),
                display_permissions_unix(new_mode, false),
                display_permissions_unix(naively_expected_new_mode, false),
            ));
        }
    }

    /// Report a failure met on one file.
    ///
    /// The remaining operands, and the rest of a tree under `-R`, are still
    /// processed after a failure, so each one is reported where it happens
    /// rather than returned, which would keep only one of them. `-f` hides
    /// the message; the failure still decides the exit status, as in GNU.
    fn report_failure(&self, error: Box<dyn UError>) {
        if !self.quiet {
            show_error!("{error}");
        }
        set_exit_code(error.code());
    }

    /// Report a directory whose entries cannot be listed. Its own mode was
    /// changed before the attempt, so with `-v` GNU also says that the
    /// subtree was left alone.
    fn report_unreadable_dir(&self, path: &Path, err: std::io::Error) {
        self.report_failure(ChmodError::CannotReadDirectory(path.into(), err).into());
        if self.verbose {
            println!(
                "{}",
                translate!("chmod-verbose-could-not-be-accessed", "file" => path.quote())
            );
        }
    }

    fn print_neither_changed(file: OsString) -> std::io::Result<()> {
        use std::io::{Write as _, stdout};
        writeln!(
            stdout(),
            "{}",
            translate!("chmod-verbose-neither-changed", "file" => file.quote())
        )
    }

    fn chmod(&self, files: &[OsString]) -> UResult<()> {
        // Whether the mode parses does not depend on the file, so check it
        // once before any file is touched: a bad mode is reported once and
        // changes nothing, as in GNU.
        self.calculate_new_mode(0, false)?;

        for filename in files {
            let file = Path::new(filename);
            // `Path::exists()` returns `false` when the path's metadata cannot be
            // read at all (for example a parent directory without search
            // permission), which made chmod misreport an existing but
            // inaccessible file as "No such file or directory". Use
            // `try_exists()` so a permission error is surfaced as such, matching
            // GNU (issue #9789).
            match file.try_exists() {
                Ok(false) => {
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
                }
                Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                    if !self.quiet {
                        show!(ChmodError::PermissionDenied(filename.into()));
                    }
                    set_exit_code(1);
                    continue;
                }
                // Present, or an unexpected error that the chmod attempt below
                // will surface with a precise message.
                Ok(true) | Err(_) => {
                    if !self.dereference && file.is_symlink() {
                        // The file is a symlink and we should not follow it
                        // chmod 755 --no-dereference a/link
                        // should not change the permissions in this case
                        continue;
                    }
                }
            }
            if self.recursive && self.preserve_root && Self::is_root(file) {
                // Name the operand the user gave: with a bind mount of "/" it is
                // not spelled "/", so say which path it is the same as.
                return Err(if file.as_os_str() == "/" {
                    ChmodError::PreserveRoot("/".into())
                } else {
                    ChmodError::PreserveRootSameAs(file.into())
                }
                .into());
            }
            if self.recursive {
                let mut ancestors = HashSet::new();
                self.walk_dir_with_context(file, true, &mut ancestors);
            } else if let Err(err) = self.chmod_file(file) {
                self.report_failure(err);
            }
        }
        Ok(())
    }

    /// Whether `file` is `/`, by `(st_dev, st_ino)` rather than by name, so a
    /// bind mount of `/` cannot slip past `--preserve-root` (as GNU does).
    fn is_root(file: impl AsRef<Path>) -> bool {
        path_is_root_dir(file, true)
    }

    /// `--preserve-root` guard re-checked at every descent: a symlink to `/`
    /// (under `-L`) or a bind mount of `/` met inside the tree is still `/`, so
    /// the operand-only check is not enough. GNU re-checks every entry too.
    fn descends_into_root(&self, path: &Path) -> bool {
        self.preserve_root && Self::is_root(path)
    }

    // Non-safe traversal implementation for platforms without safe_traversal support
    #[cfg(target_os = "redox")]
    fn walk_dir_with_context(
        &self,
        file_path: &Path,
        is_command_line_arg: bool,
        ancestors: &mut HashSet<FileInformation>,
    ) {
        // Skip (and diagnose) an entry that is '/' (a symlink to it, or a bind
        // mount) before touching it.
        if self.descends_into_root(file_path) {
            show!(ChmodError::PreserveRootSameAs(file_path.into()));
            return;
        }

        if let Err(err) = self.chmod_file(file_path) {
            self.report_failure(err);
        }

        // Determine whether to traverse symlinks based on context and traversal mode
        let should_follow_symlink = match self.traverse_symlinks {
            TraverseSymlinks::All => true,
            TraverseSymlinks::First => is_command_line_arg, // Only follow symlinks that are command line args
            TraverseSymlinks::None => false,
        };

        // If the path is a directory (or we should follow symlinks), recurse into it
        if (!file_path.is_symlink() || should_follow_symlink) && file_path.is_dir() {
            let entries = match file_path.read_dir() {
                Ok(entries) => entries,
                Err(err) => {
                    self.report_unreadable_dir(file_path, err);
                    return;
                }
            };

            // Cycle detection: identify this directory by its target's inode (resolved
            // via dereference=true) once, and reuse it for the backtrack below so we
            // don't stat the same directory twice. If it's already on the current path,
            // it's a cycle.
            let dir_info = FileInformation::from_path(file_path, true).ok();
            if let Some(info) = &dir_info
                && !ancestors.insert(info.clone())
            {
                return;
            }

            // We buffer all paths in this dir to not keep too many fd's open during recursion
            let mut paths_in_this_dir = Vec::new();

            for dir_entry in entries {
                match dir_entry {
                    Ok(entry) => paths_in_this_dir.push(entry.path()),
                    Err(err) => self.report_failure(err.into()),
                }
            }
            for path in paths_in_this_dir {
                self.walk_dir_with_context(path.as_path(), false, ancestors);
            }

            // Backtrack: remove this directory from ancestors so sibling subtrees
            // are not falsely treated as cycles.
            if let Some(info) = dir_info {
                ancestors.remove(&info);
            }
        }
    }

    #[cfg(not(target_os = "redox"))]
    fn walk_dir_with_context(
        &self,
        file_path: &Path,
        is_command_line_arg: bool,
        ancestors: &mut HashSet<FileInformation>,
    ) {
        // Skip (and diagnose) an entry that is '/' (a symlink to it, or a bind
        // mount) before touching it.
        if self.descends_into_root(file_path) {
            show!(ChmodError::PreserveRootSameAs(file_path.into()));
            return;
        }

        if let Err(err) = self.chmod_file(file_path) {
            self.report_failure(err);
        }

        // Determine whether to traverse symlinks based on context and traversal mode
        let should_follow_symlink = match self.traverse_symlinks {
            TraverseSymlinks::All => true,
            TraverseSymlinks::First => is_command_line_arg, // Only follow symlinks that are command line args
            TraverseSymlinks::None => false,
        };

        // Recurse via safe traversal, opening under the same symlink policy the checks
        // above used: the pathname is resolved again here, so under `-P` (the `-R`
        // default) O_NOFOLLOW fails the open rather than redirecting the descent into a
        // swapped-in symlink. `-H`/`-L` still follow, which is what they ask for.
        if (!file_path.is_symlink() || should_follow_symlink) && file_path.is_dir() {
            match DirFd::open(file_path, should_follow_symlink.into()) {
                Ok(dir_fd) => self.safe_traverse_dir(&dir_fd, file_path, ancestors),
                // A directory that does not open cannot be read.
                Err(err) => self.report_unreadable_dir(file_path, err),
            }
        }
    }

    #[cfg(not(target_os = "redox"))]
    fn safe_traverse_dir(
        &self,
        root_fd: &DirFd,
        root_path: &Path,
        ancestors: &mut HashSet<FileInformation>,
    ) {
        let root_info = match FileInformation::from_file(root_fd) {
            Ok(info) => info,
            Err(err) => {
                self.report_failure(ChmodError::CannotStat(root_path.into(), err).into());
                return;
            }
        };
        let entries = match root_fd.read_dir() {
            Ok(entries) => entries,
            Err(err) => {
                self.report_unreadable_dir(root_path, err);
                return;
            }
        };
        if !ancestors.insert(root_info.clone()) {
            return;
        }
        let mut frames = vec![TraversalFrame {
            name: OsString::new(),
            info: root_info,
            entries: entries.into_iter(),
            fd: None,
        }];
        let mut dir_path = root_path.to_path_buf();
        let follow = self.traverse_symlinks == TraverseSymlinks::All;
        let protected_root = if self.preserve_root {
            fs::metadata("/").ok().map(|meta| (meta.dev(), meta.ino()))
        } else {
            None
        };

        while !frames.is_empty() {
            let index = frames.len() - 1;
            let Some(entry_name) = frames[index].entries.next() else {
                // Usually the physical parent is also the logical parent. Reuse
                // it after checking its identity, avoiding a root-to-leaf reopen
                // on every ascent in deep trees. Symlinks and renames may make
                // the two parents differ; in that case use the verified chain.
                if index > 1
                    && frames[index - 1].fd.is_none()
                    && let Some(fd) = &frames[index].fd
                    && let Ok(parent) =
                        fd.open_subdir(std::ffi::OsStr::new(".."), SymlinkBehavior::NoFollow)
                    && FileInformation::from_file(&parent).ok().as_ref()
                        == Some(&frames[index - 1].info)
                {
                    frames[index - 1].fd = Some(parent);
                }
                if let Some(frame) = frames.pop() {
                    ancestors.remove(&frame.info);
                }
                if index != 0 {
                    dir_path.pop();
                }
                continue;
            };
            let entry_path = dir_path.join(&entry_name);

            if index != 0 && frames[index].fd.is_none() {
                match reopen_directory(root_fd, &frames, follow.into()) {
                    Ok(fd) => frames[index].fd = Some(fd),
                    Err(err) => {
                        self.report_unreadable_dir(&dir_path, err);
                        // Discard this frame, but continue with its parent's siblings.
                        if let Some(frame) = frames.pop() {
                            ancestors.remove(&frame.info);
                        }
                        dir_path.pop();
                        continue;
                    }
                }
            }
            let dir_fd = frames[index].fd.as_ref().unwrap_or(root_fd);
            let metadata = dir_fd.metadata_at(&entry_name, SymlinkBehavior::NoFollow);
            let meta = match metadata {
                Ok(meta) => meta,
                Err(err) => {
                    self.report_failure(if err.kind() == std::io::ErrorKind::PermissionDenied {
                        ChmodError::PermissionDenied(entry_path).into()
                    } else {
                        err.into()
                    });
                    continue;
                }
            };
            let is_symlink = meta.file_type().is_symlink();
            if is_symlink && !follow {
                if let Err(err) = self.chmod_file_internal(&entry_path, false) {
                    self.report_failure(err);
                }
                continue;
            }
            let meta = if is_symlink {
                if let Ok(meta) = dir_fd.metadata_at(&entry_name, SymlinkBehavior::Follow) {
                    meta
                } else {
                    if let Err(err) = self.chmod_file_internal(&entry_path, false) {
                        self.report_failure(err);
                    }
                    continue;
                }
            } else {
                meta
            };
            if meta.is_dir() && protected_root == Some((meta.dev(), meta.ino())) {
                show!(ChmodError::PreserveRootSameAs(entry_path));
                continue;
            }
            if let Err(err) = self.safe_chmod_file(
                &entry_path,
                dir_fd,
                &entry_name,
                &meta,
                if is_symlink {
                    self.dereference.into()
                } else {
                    SymlinkBehavior::NoFollow
                },
            ) {
                self.report_failure(err);
            }

            if meta.is_dir() {
                let child = (|| -> std::io::Result<_> {
                    let fd = dir_fd.open_subdir(&entry_name, follow.into())?;
                    let info = FileInformation::from_file(&fd)?;
                    if ancestors.contains(&info) {
                        return Ok(None);
                    }
                    let entries = fd.read_dir()?;
                    Ok(Some(TraversalFrame {
                        name: entry_name,
                        info,
                        entries: entries.into_iter(),
                        fd: Some(fd),
                    }))
                })();
                match child {
                    Ok(Some(frame)) => {
                        ancestors.insert(frame.info.clone());
                        frames.push(frame);
                        dir_path = entry_path;
                        // Retain only the most recent ancestors. Evicted frames
                        // store names and identities, not descriptors or full paths.
                        if frames.len() > MAX_CACHED_DIR_FDS + 1 {
                            let evict = frames.len() - MAX_CACHED_DIR_FDS - 1;
                            frames[evict].fd = None;
                        }
                    }
                    Ok(None) => {}
                    Err(err) => {
                        self.report_unreadable_dir(&entry_path, err);
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "redox"))]
    fn safe_chmod_file(
        &self,
        file_path: &Path,
        dir_fd: &DirFd,
        entry_name: &std::ffi::OsStr,
        metadata: &uucore::safe_traversal::Metadata,
        symlink_behavior: SymlinkBehavior,
    ) -> UResult<()> {
        // Use fd-relative metadata even when the display path exceeds PATH_MAX
        // or the kernel's limit on the number of symbolic links in a pathname.
        let current_mode = metadata.mode() & 0o7777;
        let (new_mode, naively_expected_new_mode) =
            self.calculate_new_mode(current_mode, metadata.is_dir())?;

        // Use safe traversal to change the mode
        if let Err(e) = dir_fd.chmod_at(entry_name, new_mode, symlink_behavior) {
            if self.verbose {
                println!(
                    "failed to change mode of {} to {new_mode:o}",
                    file_path.quote(),
                );
            }

            return Err(match e.kind() {
                std::io::ErrorKind::ReadOnlyFilesystem => {
                    ChmodError::ChangingPermissions(file_path.into(), e)
                }
                _ => ChmodError::PermissionDenied(file_path.into()),
            }
            .into());
        }

        // Report the change using the helper method
        self.report_permission_change(file_path, current_mode, new_mode);

        self.report_umask_conflict(file_path, new_mode, naively_expected_new_mode);

        Ok(())
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
                        Self::print_neither_changed(file.into())?;
                    }
                    Ok(()) // Skip dangling symlinks
                } else if err.kind() == std::io::ErrorKind::PermissionDenied {
                    Err(ChmodError::PermissionDenied(file.into()).into())
                } else {
                    Err(ChmodError::CannotStat(file.into(), err).into())
                };
            }
        };

        // Calculate the new mode using the helper method
        let (new_mode, naively_expected_new_mode) =
            self.calculate_new_mode(fperm, file.is_dir())?;

        // A symlink reached without dereferencing (for example one met while
        // walking a `-R` tree) must be left alone: chmod(2) follows the link,
        // so changing it would change the mode of the referent, which can live
        // outside the tree. On most Unix systems, symlink permissions are
        // ignored by the kernel anyway, so changing them has no effect.
        if file.is_symlink() && !dereference {
            if self.verbose {
                Self::print_neither_changed(file.into())?;
            }
            return Ok(());
        }

        self.change_file(fperm, self.fmode.unwrap_or(new_mode), file)?;

        self.report_umask_conflict(file, new_mode, naively_expected_new_mode);

        Ok(())
    }

    fn change_file(&self, fperm: u32, mode: u32, file: &Path) -> UResult<()> {
        // Always issue the chmod(2) call, even when the bits are unchanged: the
        // syscall can still fail (e.g. lacking permission on the file) and that
        // failure must be reported, matching GNU.
        if let Err(err) = fs::set_permissions(file, fs::Permissions::from_mode(mode)) {
            if self.verbose {
                println!(
                    "failed to change mode of file {} from {fperm:04o} ({}) to {mode:04o} ({})",
                    file.quote(),
                    display_permissions_unix(fperm, false),
                    display_permissions_unix(mode, false)
                );
            }
            return Err(ChmodError::ChangingPermissions(file.into(), err).into());
        }

        // Use the helper method for consistent reporting
        self.report_permission_change(file, fperm, mode);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "redox"))]
    #[test]
    fn test_reopen_directory_uses_cached_anchor_after_rename() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("child/leaf")).unwrap();
        let root_fd = DirFd::open(root, SymlinkBehavior::NoFollow).unwrap();
        let child_fd = root_fd
            .open_subdir("child".as_ref(), SymlinkBehavior::NoFollow)
            .unwrap();
        let leaf_fd = child_fd
            .open_subdir("leaf".as_ref(), SymlinkBehavior::NoFollow)
            .unwrap();
        let mut frames = vec![
            TraversalFrame {
                name: OsString::new(),
                info: FileInformation::from_file(&root_fd).unwrap(),
                entries: Vec::new().into_iter(),
                fd: None,
            },
            TraversalFrame {
                name: "child".into(),
                info: FileInformation::from_file(&child_fd).unwrap(),
                entries: Vec::new().into_iter(),
                fd: Some(child_fd),
            },
            TraversalFrame {
                name: "leaf".into(),
                info: FileInformation::from_file(&leaf_fd).unwrap(),
                entries: Vec::new().into_iter(),
                fd: None,
            },
        ];
        drop(leaf_fd);
        fs::rename(root.join("child"), root.join("saved")).unwrap();
        // A cached descriptor still names the original directory after a rename.
        let reopened = reopen_directory(&root_fd, &frames, SymlinkBehavior::NoFollow).unwrap();
        assert!(FileInformation::from_file(&reopened).unwrap() == frames[2].info);
        drop(reopened);
        // Once evicted, a renamed ancestor cannot be recovered through its old
        // name, and a new directory at that name must not redirect the walk.
        frames[1].fd = None;
        fs::create_dir_all(root.join("child/leaf")).unwrap();
        assert_eq!(
            reopen_directory(&root_fd, &frames, SymlinkBehavior::NoFollow)
                .err()
                .unwrap()
                .raw_os_error(),
            Some(uucore::libc::ESTALE)
        );
    }

    #[cfg(not(target_os = "redox"))]
    #[test]
    fn test_reopen_directory_symlink_policy_and_identity() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("child/leaf")).unwrap();
        let root_fd = DirFd::open(root, SymlinkBehavior::NoFollow).unwrap();
        let child_fd = root_fd
            .open_subdir("child".as_ref(), SymlinkBehavior::NoFollow)
            .unwrap();
        let leaf_fd = child_fd
            .open_subdir("leaf".as_ref(), SymlinkBehavior::NoFollow)
            .unwrap();
        let frames = vec![
            TraversalFrame {
                name: OsString::new(),
                info: FileInformation::from_file(&root_fd).unwrap(),
                entries: Vec::new().into_iter(),
                fd: None,
            },
            TraversalFrame {
                name: "child".into(),
                info: FileInformation::from_file(&child_fd).unwrap(),
                entries: Vec::new().into_iter(),
                fd: None,
            },
            TraversalFrame {
                name: "leaf".into(),
                info: FileInformation::from_file(&leaf_fd).unwrap(),
                entries: Vec::new().into_iter(),
                fd: None,
            },
        ];
        drop(leaf_fd);
        drop(child_fd);
        assert!(reopen_directory(&root_fd, &frames, SymlinkBehavior::NoFollow).is_ok());
        fs::rename(root.join("child"), root.join("saved")).unwrap();
        symlink("saved", root.join("child")).unwrap();
        // -P/-H must reject a swapped-in symlink, even to the original inode.
        assert!(reopen_directory(&root_fd, &frames, SymlinkBehavior::NoFollow).is_err());
        assert!(reopen_directory(&root_fd, &frames, SymlinkBehavior::Follow).is_ok());
        fs::remove_file(root.join("child")).unwrap();
        fs::create_dir(root.join("child")).unwrap();
        symlink("../saved/leaf", root.join("child/leaf")).unwrap();
        // Even if the leaf is unchanged, a replaced intermediate ancestor must
        // be rejected under both policies before traversing its contents.
        for policy in [SymlinkBehavior::NoFollow, SymlinkBehavior::Follow] {
            assert!(reopen_directory(&root_fd, &frames, policy).is_err());
        }
    }

    #[test]
    fn test_extract_negative_modes() {
        // "chmod -w -r file" becomes "chmod -w,-r file". clap does not accept "-w,-r" as MODE.
        // Therefore, "w" is added as pseudo mode to pass clap.
        let (c, a, i) = extract_negative_modes(["-w", "-r", "file"].iter().map(OsString::from));
        assert_eq!(c, Some("-w,-r".to_string()));
        assert_eq!(a, ["w", "file"]);
        assert_eq!(i, [0, 1]);

        // "chmod -w file -r" becomes "chmod -w,-r file". clap does not accept "-w,-r" as MODE.
        // Therefore, "w" is added as pseudo mode to pass clap.
        let (c, a, i) = extract_negative_modes(["-w", "file", "-r"].iter().map(OsString::from));
        assert_eq!(c, Some("-w,-r".to_string()));
        assert_eq!(a, ["w", "file"]);
        assert_eq!(i, [0, 2]);

        // "chmod -w -- -r file" becomes "chmod -w -r file", where "-r" is interpreted as file.
        // Again, "w" is needed as pseudo mode.
        let (c, a, i) = extract_negative_modes(["-w", "--", "-r", "f"].iter().map(OsString::from));
        assert_eq!(c, Some("-w".to_string()));
        assert_eq!(a, ["w", "--", "-r", "f"]);
        assert_eq!(i, [0]);

        // "chmod -- -r file" becomes "chmod -r file".
        let (c, a, i) = extract_negative_modes(["--", "-r", "file"].iter().map(OsString::from));
        assert_eq!(c, None);
        assert_eq!(a, ["--", "-r", "file"]);
        assert!(i.is_empty());
    }
}
