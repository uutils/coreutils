// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Unix-specific implementations for the rm utility

// spell-checker:ignore fstatat unlinkat statx behaviour

use indicatif::ProgressBar;
use std::ffi::OsStr;
use std::fs;
use std::io::{IsTerminal, stdin};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::FromIo;
use uucore::prompt_yes;
use uucore::safe_traversal::{DirFd, SymlinkBehavior};
use uucore::show_error;
use uucore::translate;

use super::super::{
    InteractiveMode, Options, is_dir_empty, is_readable_metadata, prompt_descend, remove_file,
    show_permission_denied_error, show_removal_error, verbose_removed_directory,
    verbose_removed_file,
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
fn prompt_file_with_stat(path: &Path, stat: &libc::stat, options: &Options) -> bool {
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
        (false, _, _, InteractiveMode::PromptProtected) => true,
        (false, false, false, InteractiveMode::Never) => true,
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
            verbose_removed_file(path, options);
            Some(false)
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                show_error!("cannot remove {}: Permission denied", path.quote());
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
            verbose_removed_directory(path, options);
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
    verbose_removed_directory(entry_path, options);
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
        if is_dir {
            verbose_removed_directory(entry_path, options);
        } else {
            verbose_removed_file(entry_path, options);
        }
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
            verbose_removed_directory(path, options);
            false
        }
    }
}

/// `None` when `path` has no parent (the filesystem root). A directory whose
/// own device differs from this is a mount point, which `--preserve-root=all`
/// refuses to cross.
fn parent_device(path: &Path) -> Option<u64> {
    let parent = match path.parent() {
        // A bare name like "b" has an empty parent, meaning the current dir.
        Some(p) if p.as_os_str().is_empty() => Path::new("."),
        Some(p) => p,
        None => return None,
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
    let (initial_mode, root_dev) = match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_dir() => {
            return remove_file(path, options, progress_bar);
        }
        // root_dev is the tree-root device, captured once and compared against
        // every subdirectory for --one-file-system (not recomputed per level).
        Ok(metadata) => (metadata.permissions().mode(), metadata.dev()),
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

    // Try to open the directory using DirFd for secure traversal
    let dir_fd = match DirFd::open(path, SymlinkBehavior::Follow) {
        Ok(fd) => fd,
        Err(e) => {
            // If we can't open the directory for safe traversal,
            // handle the error appropriately and try to remove if possible
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                // Try to remove the directory directly if it's empty
                if fs::remove_dir(path).is_ok() {
                    verbose_removed_directory(path, options);
                    return false;
                }
                // If we can't read the directory AND can't remove it,
                // show permission denied error for GNU compatibility
                return show_permission_denied_error(path);
            }
            return show_removal_error(e, path);
        }
    };

    // Entries of the root directory have the root itself as their parent.
    let error = safe_remove_dir_recursive_impl(path, &dir_fd, options, root_dev, root_dev);

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

/// Frame on the explicit directory-walk stack used by
/// [`safe_remove_dir_recursive_impl`].
///
/// Only the active directory stays open. Before descending we close the parent
/// `DirFd` and restore it later with `openat(child, "..")`, so open-file use
/// stays O(1) with depth and we never re-open a path longer than `PATH_MAX`
/// (#7995, GNU `tests/rm/deep-2`). Children are still unlinked with `unlinkat`
/// relative to an open parent.
struct DirWalkFrame {
    path: std::path::PathBuf,
    dir_fd: Option<DirFd>,
    dir_dev: u64,
    pending: Vec<std::ffi::OsString>,
    error: bool,
    mode: libc::mode_t,
    name_in_parent: std::ffi::OsString,
}

/// Re-open the parent of `child` without using the absolute path.
///
/// Deep trees (GNU `rm/deep-2`) build paths longer than `PATH_MAX`, so
/// `DirFd::open(path)` fails with `ENAMETOOLONG`. `openat(child, "..")` stays
/// relative and keeps FD use O(1) when paired with close-before-descend.
fn reopen_parent_from_child(child: &DirFd) -> std::io::Result<DirFd> {
    child.open_subdir(OsStr::new(".."), SymlinkBehavior::Follow)
}

#[cfg(not(target_os = "redox"))]
pub fn safe_remove_dir_recursive_impl(
    path: &Path,
    dir_fd: &DirFd,
    options: &Options,
    root_dev: u64,
    parent_dev: u64,
) -> bool {
    // Snapshot entries from the caller's DirFd, then re-open so the walk owns
    // every descriptor on the stack (parent FDs are closed while descended).
    let root_entries = match dir_fd.read_dir() {
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

    let root_fd = match DirFd::open(path, SymlinkBehavior::Follow) {
        Ok(fd) => fd,
        Err(e) => return handle_error_with_force(e, path, options),
    };

    // parent_dev is unused on the root frame itself; the caller passes
    // parent_dev == root_dev for the initial invocation.
    let _ = parent_dev;

    // Reverse so pop() yields entries in the original read order.
    let mut root_pending = root_entries;
    root_pending.reverse();

    let mut stack = vec![DirWalkFrame {
        path: path.to_path_buf(),
        dir_fd: Some(root_fd),
        dir_dev: root_dev,
        pending: root_pending,
        error: false,
        mode: 0,
        name_in_parent: std::ffi::OsString::new(),
    }];

    let mut had_error = false;

    while !stack.is_empty() {
        let frame_idx = stack.len() - 1;

        if stack[frame_idx].pending.is_empty() {
            let child_error = stack[frame_idx].error;
            let child_path = stack[frame_idx].path.clone();
            let child_mode = stack[frame_idx].mode;
            let child_name = stack[frame_idx].name_in_parent.clone();

            if stack.len() == 1 {
                // Root frame: caller removes the top directory.
                stack[frame_idx].dir_fd = None;
                had_error = child_error;
                stack.pop();
                break;
            }

            let parent_idx = frame_idx - 1;
            // Restore the parent FD from the child via ".." before dropping the
            // child FD. Do not DirFd::open(parent_path): deep trees exceed PATH_MAX.
            if stack[parent_idx].dir_fd.is_none() {
                let child_fd = stack[frame_idx].dir_fd.take();
                match child_fd.as_ref() {
                    Some(fd) => match reopen_parent_from_child(fd) {
                        Ok(parent_fd) => stack[parent_idx].dir_fd = Some(parent_fd),
                        Err(e) => {
                            stack[parent_idx].error |=
                                handle_error_with_force(e, &stack[parent_idx].path, options);
                            stack.pop();
                            continue;
                        }
                    },
                    None => {
                        // Defensive fallback for short paths only; long paths fail.
                        match DirFd::open(&stack[parent_idx].path, SymlinkBehavior::NoFollow) {
                            Ok(fd) => stack[parent_idx].dir_fd = Some(fd),
                            Err(e) => {
                                stack[parent_idx].error |=
                                    handle_error_with_force(e, &stack[parent_idx].path, options);
                                stack.pop();
                                continue;
                            }
                        }
                    }
                }
            } else {
                stack[frame_idx].dir_fd = None;
            }

            if child_error {
                stack[parent_idx].error = true;
            } else if options.interactive == InteractiveMode::Always
                && !prompt_dir_with_mode(&child_path, child_mode, options)
            {
                stack.pop();
                continue;
            } else if let Some(parent_fd) = stack[parent_idx].dir_fd.as_ref() {
                stack[parent_idx].error |=
                    handle_unlink(parent_fd, child_name.as_ref(), &child_path, true, options);
            } else {
                stack[parent_idx].error = true;
            }

            stack.pop();
            continue;
        }

        let Some(entry_name) = stack[frame_idx].pending.pop() else {
            continue;
        };
        let entry_path = stack[frame_idx].path.join(&entry_name);

        // Parent FD is restored via openat("..") when a child finishes. Path
        // reopen is only a last resort for short paths (PATH_MAX trees break).
        if stack[frame_idx].dir_fd.is_none() {
            match DirFd::open(&stack[frame_idx].path, SymlinkBehavior::NoFollow) {
                Ok(fd) => stack[frame_idx].dir_fd = Some(fd),
                Err(e) => {
                    stack[frame_idx].error |=
                        handle_error_with_force(e, &stack[frame_idx].path, options);
                    stack[frame_idx].pending.clear();
                    continue;
                }
            }
        }

        let parent_dev_for_child = stack[frame_idx].dir_dev;
        let entry_stat = {
            let Some(dir_fd) = stack[frame_idx].dir_fd.as_ref() else {
                stack[frame_idx].error = true;
                continue;
            };
            match dir_fd.stat_at(&entry_name, SymlinkBehavior::NoFollow) {
                Ok(stat) => stat,
                Err(e) => {
                    stack[frame_idx].error |= handle_error_with_force(e, &entry_path, options);
                    continue;
                }
            }
        };

        let is_dir = ((entry_stat.st_mode as libc::mode_t) & libc::S_IFMT) == libc::S_IFDIR;

        if is_dir {
            // st_dev's type varies by platform (i32 on macOS, u64 on Linux).
            #[allow(clippy::unnecessary_cast)]
            let entry_dev = entry_stat.st_dev as u64;

            if options.one_fs && entry_dev != root_dev {
                show_error!(
                    "{}",
                    translate!("rm-error-skipping-different-device", "file" => entry_path.quote())
                );
                stack[frame_idx].error = true;
                continue;
            }

            // --preserve-root=all compares against the immediate parent rather
            // than the tree root, so a mount nested anywhere in the tree is
            // caught even when --one-file-system is not in effect.
            if options.preserve_root_all && entry_dev != parent_dev_for_child {
                show_preserve_root_all_skip(&entry_path);
                stack[frame_idx].error = true;
                continue;
            }

            // Ask user if they want to descend into this directory
            if options.interactive == InteractiveMode::Always
                && !is_dir_empty(&entry_path)
                && !prompt_descend(&entry_path)
            {
                continue;
            }

            // Open the subdirectory while the parent is still open, then close
            // the parent before pushing the child frame so FD use stays O(1)
            // with depth (GNU rm closes as it descends).
            //
            // rm never follows symlinks during recursion, so open with
            // NoFollow: if an attacker swaps this just-stat'd directory for a
            // symlink before the open, O_NOFOLLOW makes openat fail instead of
            // descending off-tree and deleting unrelated files.
            let child_dir_fd = {
                let Some(dir_fd) = stack[frame_idx].dir_fd.as_ref() else {
                    stack[frame_idx].error = true;
                    continue;
                };
                match dir_fd.open_subdir(&entry_name, SymlinkBehavior::NoFollow) {
                    Ok(fd) => fd,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            stack[frame_idx].error |= handle_permission_denied(
                                dir_fd,
                                entry_name.as_ref(),
                                &entry_path,
                                options,
                            );
                        } else {
                            stack[frame_idx].error |=
                                handle_error_with_force(e, &entry_path, options);
                        }
                        continue;
                    }
                }
            };

            let child_entries = match child_dir_fd.read_dir() {
                Ok(entries) => entries,
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    if !options.force {
                        show_permission_denied_error(&entry_path);
                    }
                    drop(child_dir_fd);
                    stack[frame_idx].error |= !options.force;
                    continue;
                }
                Err(e) => {
                    drop(child_dir_fd);
                    stack[frame_idx].error |= handle_error_with_force(e, &entry_path, options);
                    continue;
                }
            };

            // Close parent descriptor before descending.
            stack[frame_idx].dir_fd = None;

            let mut child_pending = child_entries;
            child_pending.reverse();

            stack.push(DirWalkFrame {
                path: entry_path,
                dir_fd: Some(child_dir_fd),
                dir_dev: entry_dev,
                pending: child_pending,
                error: false,
                mode: entry_stat.st_mode as libc::mode_t,
                name_in_parent: entry_name,
            });
        } else if prompt_file_with_stat(&entry_path, &entry_stat, options) {
            if let Some(dir_fd) = stack[frame_idx].dir_fd.as_ref() {
                stack[frame_idx].error |=
                    handle_unlink(dir_fd, entry_name.as_ref(), &entry_path, false, options);
            } else {
                stack[frame_idx].error = true;
            }
        }
    }

    had_error
}

#[cfg(target_os = "redox")]
pub fn safe_remove_dir_recursive_impl(
    _path: &Path,
    _dir_fd: &DirFd,
    _options: &Options,
    _root_dev: u64,
    _parent_dev: u64,
) -> bool {
    // safe_traversal stat_at is not supported on Redox
    // This shouldn't be called on Redox, but provide a stub for compilation
    true // Return error
}
