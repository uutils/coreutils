// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fstatat unlinkat statx behaviour automount

// Unix-specific implementations for the rm utility

use indicatif::ProgressBar;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{IsTerminal, stdin};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, strip_errno};
use uucore::prompt_yes;
use uucore::safe_traversal::{DirFd, FileStat, SymlinkBehavior};
use uucore::show_error;
use uucore::translate;

use super::super::{
    InteractiveMode, Options, is_dir_empty, is_readable_metadata, prompt_descend, remove_file,
    report_verbose_write_error, show_permission_denied_error, show_removal_error,
    verbose_removed_directory, verbose_removed_file,
};

#[inline]
fn mode_readable(mode: libc::mode_t) -> bool {
    (mode & libc::S_IRUSR) != 0
}

#[inline]
fn mode_writable(mode: libc::mode_t) -> bool {
    (mode & libc::S_IWUSR) != 0
}

/// File prompt that reuses existing stat data to avoid extra statx calls
fn prompt_file_with_stat(path: &Path, stat: &FileStat, options: &Options) -> bool {
    if options.interactive == InteractiveMode::Never {
        return true;
    }

    let is_symlink = ((stat.st_mode as libc::mode_t) & libc::S_IFMT) == libc::S_IFLNK;
    let writable = mode_writable(stat.st_mode as libc::mode_t);
    let len = stat.st_size as u64;
    let stdin_ok = options.__presume_input_tty.unwrap_or(false) || stdin().is_terminal();

    // Match original behaviour:
    // - Interactive::Always: always prompt; use non-protected wording when writable,
    //   otherwise fall through to protected wording.
    if options.interactive == InteractiveMode::Always {
        if is_symlink {
            return prompt_yes!("remove symbolic link {}?", path.quote());
        }
        if writable {
            return if len == 0 {
                prompt_yes!("remove regular empty file {}?", path.quote())
            } else {
                prompt_yes!("remove file {}?", path.quote())
            };
        }
        // Not writable: use protected wording below
    }

    // Interactive::Once or ::PromptProtected (and non-writable Always) paths
    match (stdin_ok, writable, len == 0) {
        (false, _, _) if options.interactive == InteractiveMode::PromptProtected => true,
        (_, true, _) => true,
        (_, false, true) => prompt_yes!(
            "remove write-protected regular empty file {}?",
            path.quote()
        ),
        _ => prompt_yes!("remove write-protected regular file {}?", path.quote()),
    }
}

/// Directory prompt that reuses existing stat data to avoid extra statx calls
fn prompt_dir_with_mode(path: &Path, mode: libc::mode_t, options: &Options) -> bool {
    if options.interactive == InteractiveMode::Never {
        return true;
    }

    let readable = mode_readable(mode as libc::mode_t);
    let writable = mode_writable(mode as libc::mode_t);
    let stdin_ok = options.__presume_input_tty.unwrap_or(false) || stdin().is_terminal();

    match (stdin_ok, readable, writable, options.interactive) {
        (false, _, _, InteractiveMode::PromptProtected)
        | (false, false, false, InteractiveMode::Never) => true,
        (_, false, false, _) => prompt_yes!(
            "attempt removal of inaccessible directory {}?",
            path.quote()
        ),
        (_, false, true, InteractiveMode::Always) => {
            prompt_yes!(
                "attempt removal of inaccessible directory {}?",
                path.quote()
            )
        }
        (_, true, false, _) => prompt_yes!("remove write-protected directory {}?", path.quote()),
        (_, _, _, InteractiveMode::Always) => prompt_yes!("remove directory {}?", path.quote()),
        (_, _, _, _) => true,
    }
}

/// Whether the given file or directory is readable.
pub fn is_readable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| is_readable_metadata(&metadata))
}

/// Remove a single file using safe traversal
pub fn safe_remove_file(
    path: &Path,
    options: &Options,
    progress_bar: Option<&ProgressBar>,
) -> Option<bool> {
    // If there is no parent (path is directly under cwd), unlinkat relative to "."
    let parent = path.parent().unwrap_or(Path::new("."));
    let file_name = path.file_name()?;

    let dir_fd = DirFd::open(parent, SymlinkBehavior::Follow).ok()?;

    match dir_fd.unlink_at(file_name, false) {
        Ok(_) => {
            // Update progress bar for file removal
            if let Some(pb) = progress_bar {
                pb.inc(1);
            }
            report_verbose_write_error(verbose_removed_file(path, options));
            Some(false)
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                show_error!("cannot remove {}: {}", path.quote(), strip_errno(&e));
            } else {
                let _ = show_removal_error(e, path);
            }
            Some(true)
        }
    }
}

/// Remove an empty directory using safe traversal
pub fn safe_remove_empty_dir(
    path: &Path,
    options: &Options,
    progress_bar: Option<&ProgressBar>,
) -> Option<bool> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let dir_name = path.file_name()?;

    let dir_fd = DirFd::open(parent, SymlinkBehavior::Follow).ok()?;

    match dir_fd.unlink_at(dir_name, true) {
        Ok(_) => {
            // Update progress bar for directory removal
            if let Some(pb) = progress_bar {
                pb.inc(1);
            }
            report_verbose_write_error(verbose_removed_directory(path, options));
            Some(false)
        }
        Err(e) => {
            let e =
                e.map_err_context(|| translate!("rm-error-cannot-remove", "file" => path.quote()));
            show_error!("{e}");
            Some(true)
        }
    }
}

/// Helper to handle errors with force mode consideration
fn handle_error_with_force(e: std::io::Error, path: &Path, options: &Options) -> bool {
    // Permission denied errors should be shown even in force mode
    // This matches GNU rm behavior
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        show_permission_denied_error(path);
        return true;
    }

    if !options.force {
        let e = e.map_err_context(|| translate!("rm-error-cannot-remove", "file" => path.quote()));
        show_error!("{e}");
    }
    !options.force
}

/// Helper to handle permission denied errors
fn handle_permission_denied(
    dir_fd: &DirFd,
    entry_name: &OsStr,
    entry_path: &Path,
    options: &Options,
) -> bool {
    // When we can't open a subdirectory due to permission denied,
    // try to remove it directly (it might be empty).
    // This matches GNU rm behavior with -f flag.
    if let Err(_remove_err) = dir_fd.unlink_at(entry_name, true) {
        // The directory is not empty (or another error) and we can't read it
        // to remove its contents. Report the original permission denied error.
        // This matches GNU rm behavior — the real problem is we lack
        // permission to traverse the directory.
        show_permission_denied_error(entry_path);
        return true;
    }
    // Successfully removed empty directory
    report_verbose_write_error(verbose_removed_directory(entry_path, options));
    false
}

/// Helper to handle unlink operation with error reporting
fn handle_unlink(
    dir_fd: &DirFd,
    entry_name: &OsStr,
    entry_path: &Path,
    is_dir: bool,
    options: &Options,
) -> bool {
    if let Err(e) = dir_fd.unlink_at(entry_name, is_dir) {
        let e = e
            .map_err_context(|| translate!("rm-error-cannot-remove", "file" => entry_path.quote()));
        show_error!("{e}");
        true
    } else {
        report_verbose_write_error(if is_dir {
            verbose_removed_directory(entry_path, options)
        } else {
            verbose_removed_file(entry_path, options)
        });
        false
    }
}

/// Helper function to remove directory handling special cases
pub fn remove_dir_with_special_cases(path: &Path, options: &Options, error_occurred: bool) -> bool {
    match fs::remove_dir(path) {
        Err(_) if !error_occurred && !is_readable(path) => {
            // For compatibility with GNU test case
            // `tests/rm/unread2.sh`, show "Permission denied" in this
            // case instead of "Directory not empty".
            show_permission_denied_error(path);
            true
        }
        Err(_) if !error_occurred && path.read_dir().is_err() => {
            // For compatibility with GNU test case on Linux
            // Check if directory is readable by attempting to read it
            show_permission_denied_error(path);
            true
        }
        Err(e) if !error_occurred => show_removal_error(e, path),
        Err(_) => {
            // If we already had errors while
            // trying to remove the children, then there is no need to
            // show another error message as we return from each level
            // of the recursion.
            error_occurred
        }
        Ok(_) => {
            report_verbose_write_error(verbose_removed_directory(path, options));
            false
        }
    }
}

/// `None` when `path` has no parent (the filesystem root). A directory whose
/// own device differs from this is a mount point, which `--preserve-root=all`
/// refuses to cross.
fn parent_device(path: &Path) -> Option<u64> {
    let parent = match path.parent()? {
        // A bare name like "b" has an empty parent, meaning the current dir.
        p if p.as_os_str().is_empty() => Path::new("."),
        p => p,
    };
    fs::metadata(parent).ok().map(|m| m.dev())
}

/// GNU prints two lines, not one, when `--preserve-root=all` stops at a device
/// boundary.
fn show_preserve_root_all_skip(path: &Path) {
    show_error!(
        "{}",
        translate!("rm-error-skipping-different-device", "file" => path.quote())
    );
    show_error!("{}", translate!("rm-error-and-preserve-root-all-in-effect"));
}

pub fn safe_remove_dir_recursive(
    path: &Path,
    options: &Options,
    progress_bar: Option<&ProgressBar>,
) -> bool {
    // Base case 1: this is a file or a symbolic link.
    // Use lstat to avoid race condition between check and use
    let (initial_mode, root_dev, root_ino) = match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_dir() => {
            return remove_file(path, options, progress_bar);
        }
        // root_dev is the tree-root device, captured once and compared against
        // every subdirectory for --one-file-system (not recomputed per level).
        Ok(metadata) => (
            metadata.permissions().mode(),
            metadata.dev(),
            metadata.ino(),
        ),
        Err(e) => {
            return show_removal_error(e, path);
        }
    };

    // A directory named directly on the command line is itself a mount point
    // when its device differs from its parent's; the recursion below only ever
    // sees its children, so this boundary has to be caught here.
    if options.preserve_root_all && parent_device(path).is_some_and(|dev| dev != root_dev) {
        show_preserve_root_all_skip(path);
        return true;
    }

    // Open the directory with DirFd for secure traversal. The lstat above
    // already established that the operand is a real directory, so a symlink
    // here can only have been swapped in since, and following it would land on
    // a tree we never named. GNU refuses the same way, opening directories
    // O_NOFOLLOW under FTS_PHYSICAL. The descent was already hardened this way;
    // this is the entry point.
    let dir_fd = match DirFd::open(path, SymlinkBehavior::NoFollow) {
        Ok(fd) => fd,
        Err(e) => {
            // If we can't open the directory for safe traversal,
            // handle the error appropriately and try to remove if possible
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                // Try to remove the directory directly if it's empty
                if fs::remove_dir(path).is_ok() {
                    report_verbose_write_error(verbose_removed_directory(path, options));
                    return false;
                }
                // If we can't read the directory AND can't remove it,
                // show permission denied error for GNU compatibility
                return show_permission_denied_error(path);
            }
            return show_removal_error(e, path);
        }
    };

    // O_NOFOLLOW only rejects a symlink as the *final* component, and a trailing
    // slash ("dir/") makes the link stop being final, so it still resolves. Pin
    // the result down by confirming the fd we hold is the inode we checked.
    match dir_fd.metadata() {
        Ok(m) if m.dev() == root_dev && m.ino() == root_ino => {}
        Ok(_) => {
            // Not necessarily a symlink: a directory swapped for another
            // directory, or an automount that only lstat failed to trigger,
            // lands here too, so don't claim ELOOP.
            show_error!(
                "{}",
                translate!("rm-error-cannot-remove-changed", "file" => path.quote())
            );
            return true;
        }
        Err(e) => {
            return show_removal_error(e, path);
        }
    }

    let error = safe_remove_dir_recursive_impl(path, dir_fd, options, root_dev, root_ino);

    // After processing all children, remove the directory itself
    if error {
        error
    } else {
        // Ask user permission if needed
        if options.interactive == InteractiveMode::Always
            && !prompt_dir_with_mode(path, initial_mode as libc::mode_t, options)
        {
            return false;
        }

        // Before trying to remove the directory, check if it's actually empty
        // This handles the case where some children weren't removed due to user "no" responses
        if !is_dir_empty(path) {
            // Directory is not empty, so we can't/shouldn't remove it
            // In interactive mode, this might be expected if user said "no" to some children
            // In non-interactive mode, this indicates an error (some children couldn't be removed)
            if options.interactive == InteractiveMode::Always {
                return false;
            }
            // Try to remove the directory anyway and let the system tell us why it failed
            // Use false for error_occurred since this is the main error we want to report
            return remove_dir_with_special_cases(path, options, false);
        }

        // Directory is empty and user approved removal
        if let Some(result) = safe_remove_empty_dir(path, options, progress_bar) {
            result
        } else {
            remove_dir_with_special_cases(path, options, error)
        }
    }
}

/// A directory suspended while one of its entries is being emptied. Together
/// with the directory being walked, only the deepest [`OPEN_DIR_FDS`] keep
/// their descriptor; the others get it back through ".." on the way up, so
/// depth costs memory, not file descriptors.
#[cfg(not(target_os = "redox"))]
struct Frame {
    dir_fd: Option<DirFd>,
    dev: u64,
    ino: u64,
    entries: std::vec::IntoIter<OsString>,
    error: bool,
    /// The entry being emptied, removed once that is done.
    entry_name: OsString,
    entry_mode: libc::mode_t,
    /// Length of this directory's own path, to cut the entry back off with.
    path_len: usize,
}

#[cfg(not(target_os = "redox"))]
const OPEN_DIR_FDS: usize = 16;

/// The path of the directory being walked, kept as raw bytes so that stepping
/// into an entry and back out again is a truncation rather than a fresh
/// allocation: a tree deep enough to need this walk has paths long enough that
/// copying one per entry dominates the removal.
#[cfg(not(target_os = "redox"))]
fn path_of(buf: &[u8]) -> &Path {
    Path::new(OsStr::from_bytes(buf))
}

#[cfg(not(target_os = "redox"))]
fn path_push(buf: &mut Vec<u8>, name: &OsStr) {
    if !buf.is_empty() && buf.last() != Some(&b'/') {
        buf.push(b'/');
    }
    buf.extend_from_slice(name.as_bytes());
}

/// Whether `name` is an empty directory in `dir_fd`, asked through the
/// descriptor rather than the path: past PATH_MAX a path-based check just
/// fails, and a failure counts as non-empty, so rm would prompt before
/// descending into an empty directory and leave it behind when that prompt is
/// declined. An unreadable directory still counts as non-empty.
#[cfg(not(target_os = "redox"))]
fn is_subdir_empty(dir_fd: &DirFd, name: &OsStr) -> bool {
    dir_fd
        .open_subdir(name, SymlinkBehavior::NoFollow)
        .and_then(|fd| fd.read_dir())
        .is_ok_and(|entries| entries.is_empty())
}

/// Reopen the parent of `child_fd` through "..", checking it is still the
/// directory we descended from and not one swapped in mid-walk.
#[cfg(not(target_os = "redox"))]
fn reopen_parent(child_fd: &DirFd, dev: u64, ino: u64) -> std::io::Result<DirFd> {
    let parent_fd = child_fd.open_subdir(OsStr::new(".."), SymlinkBehavior::NoFollow)?;
    let info = parent_fd.metadata()?.file_info();
    if info.device() == dev && info.inode() == ino {
        Ok(parent_fd)
    } else {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    }
}

#[cfg(not(target_os = "redox"))]
pub fn safe_remove_dir_recursive_impl(
    path: &Path,
    mut cur_fd: DirFd,
    options: &Options,
    root_dev: u64,
    root_ino: u64,
) -> bool {
    // Read directory entries using safe traversal
    let entries = match cur_fd.read_dir() {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            if !options.force {
                show_permission_denied_error(path);
            }
            return !options.force;
        }
        Err(e) => {
            return handle_error_with_force(e, path, options);
        }
    };

    let mut error = false;
    let mut entries = entries.into_iter();
    let mut path_buf = path.as_os_str().as_bytes().to_vec();
    // Entries of the root directory have the root itself as their parent.
    let (mut parent_dev, mut ino) = (root_dev, root_ino);
    // Walk with an explicit stack: recursing once per level overflows the real
    // stack on a tree tens of thousands of levels deep.
    let mut stack: Vec<Frame> = Vec::new();

    // Process each entry
    loop {
        let Some(entry_name) = entries.next() else {
            // This directory is done: resume its parent, which removes it.
            let Some(parent) = stack.pop() else {
                return error;
            };
            cur_fd = if let Some(fd) = parent.dir_fd {
                fd
            } else {
                match reopen_parent(&cur_fd, parent.dev, parent.ino) {
                    Ok(fd) => fd,
                    Err(e) => {
                        // Name the parent, not the entry we came up from.
                        path_buf.truncate(parent.path_len);
                        return show_removal_error(e, path_of(&path_buf));
                    }
                }
            };
            let child_error = error;
            (parent_dev, ino, entries) = (parent.dev, parent.ino, parent.entries);
            error = parent.error | child_error;

            // Ask user permission if needed for this subdirectory, then remove
            // the now-empty subdirectory using safe unlinkat.
            if !child_error
                && (options.interactive != InteractiveMode::Always
                    || prompt_dir_with_mode(path_of(&path_buf), parent.entry_mode, options))
            {
                error |= handle_unlink(
                    &cur_fd,
                    &parent.entry_name,
                    path_of(&path_buf),
                    true,
                    options,
                );
            }
            path_buf.truncate(parent.path_len);
            continue;
        };

        // Build the entry's path in place: a tree deep enough to need this
        // walk also has paths too long to reallocate once per entry.
        let parent_len = path_buf.len();
        path_push(&mut path_buf, &entry_name);
        let descended = 'entry: {
            // Get metadata for the entry using fstatat
            let entry_stat = match cur_fd.stat_at(&entry_name, SymlinkBehavior::NoFollow) {
                Ok(stat) => stat,
                Err(e) => {
                    error |= handle_error_with_force(e, path_of(&path_buf), options);
                    break 'entry false;
                }
            };

            // Check if it's a directory
            let is_dir = ((entry_stat.st_mode as libc::mode_t) & libc::S_IFMT) == libc::S_IFDIR;
            if !is_dir {
                // Remove file - check if user wants to remove it first
                if prompt_file_with_stat(path_of(&path_buf), &entry_stat, options) {
                    error |=
                        handle_unlink(&cur_fd, &entry_name, path_of(&path_buf), false, options);
                }
                break 'entry false;
            }

            // st_dev's type varies by platform (i32 on macOS, u64 on Linux).
            #[allow(clippy::unnecessary_cast)]
            let entry_dev = entry_stat.st_dev as u64;
            #[allow(clippy::unnecessary_cast)]
            let entry_ino = entry_stat.st_ino as u64;

            if options.one_fs && entry_dev != root_dev {
                show_error!(
                    "{}",
                    translate!("rm-error-skipping-different-device", "file" => path_of(&path_buf).quote())
                );
                error = true;
                break 'entry false;
            }

            // --preserve-root=all compares against the immediate parent rather
            // than the tree root, so a mount nested anywhere in the tree is
            // caught even when --one-file-system is not in effect.
            if options.preserve_root_all && entry_dev != parent_dev {
                show_preserve_root_all_skip(path_of(&path_buf));
                error = true;
                break 'entry false;
            }

            // Ask user if they want to descend into this directory
            if options.interactive == InteractiveMode::Always
                && !is_subdir_empty(&cur_fd, &entry_name)
                && !prompt_descend(path_of(&path_buf))
            {
                break 'entry false;
            }

            // Recursively remove subdirectory using safe traversal. rm never
            // follows symlinks during recursion, so open with NoFollow: if an
            // attacker swaps this just-stat'd directory for a symlink before the
            // open, O_NOFOLLOW makes openat fail instead of descending off-tree
            // and deleting unrelated files.
            let child_dir_fd = match cur_fd.open_subdir(&entry_name, SymlinkBehavior::NoFollow) {
                Ok(fd) => fd,
                Err(e) => {
                    // If we can't open the subdirectory for safe traversal,
                    // try to handle it as best we can with safe operations
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        error |= handle_permission_denied(
                            &cur_fd,
                            &entry_name,
                            path_of(&path_buf),
                            options,
                        );
                    } else {
                        error |= handle_error_with_force(e, path_of(&path_buf), options);
                    }
                    break 'entry false;
                }
            };

            let child_entries = match child_dir_fd.read_dir() {
                Ok(child_entries) => (child_entries.into_iter(), false),
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    if !options.force {
                        show_permission_denied_error(path_of(&path_buf));
                    }
                    (Vec::new().into_iter(), !options.force)
                }
                Err(e) => (
                    Vec::new().into_iter(),
                    handle_error_with_force(e, path_of(&path_buf), options),
                ),
            };

            // Suspend this directory and empty the subdirectory first.
            stack.push(Frame {
                dir_fd: Some(std::mem::replace(&mut cur_fd, child_dir_fd)),
                dev: parent_dev,
                ino,
                entries: std::mem::replace(&mut entries, child_entries.0),
                error: std::mem::replace(&mut error, child_entries.1),
                entry_name,
                entry_mode: entry_stat.st_mode as libc::mode_t,
                path_len: parent_len,
            });
            if let Some(closable) = stack.len().checked_sub(OPEN_DIR_FDS) {
                stack[closable].dir_fd = None;
            }
            parent_dev = entry_dev;
            ino = entry_ino;
            true
        };
        if !descended {
            path_buf.truncate(parent_len);
        }
    }
}

#[cfg(target_os = "redox")]
pub fn safe_remove_dir_recursive_impl(
    _path: &Path,
    _dir_fd: DirFd,
    _options: &Options,
    _root_dev: u64,
    _root_ino: u64,
) -> bool {
    // safe_traversal stat_at is not supported on Redox
    // This shouldn't be called on Redox, but provide a stub for compilation
    true // Return error
}
