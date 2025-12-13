// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Linux-specific implementations for the rm utility

// spell-checker:ignore fstatat unlinkat

use indicatif::ProgressBar;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::FromIo;
use uucore::safe_traversal::{DirFd, clear_errno, take_errno};
use uucore::show_error;
use uucore::translate;

use super::super::{
    InteractiveMode, Options, is_dir_empty, is_readable_metadata, prompt_descend, prompt_dir,
    prompt_file, remove_file, show_permission_denied_error, show_removal_error,
    verbose_removed_directory, verbose_removed_file,
};

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
    let parent = path.parent()?;
    let file_name = path.file_name()?;

    let dir_fd = DirFd::open(parent).ok()?;

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
    let parent = path.parent()?;
    let dir_name = path.file_name()?;

    let dir_fd = DirFd::open(parent).ok()?;

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
    if let Err(remove_err) = dir_fd.unlink_at(entry_name, true) {
        // Failed to remove - show appropriate error
        if remove_err.kind() == std::io::ErrorKind::PermissionDenied {
            // Permission denied errors are always shown, even with force
            show_permission_denied_error(entry_path);
            return true;
        } else if !options.force {
            let remove_err = remove_err.map_err_context(
                || translate!("rm-error-cannot-remove", "file" => entry_path.quote()),
            );
            show_error!("{remove_err}");
            return true;
        }
        // With force mode, suppress non-permission errors
        return !options.force;
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

pub fn safe_remove_dir_recursive(
    path: &Path,
    options: &Options,
    progress_bar: Option<&ProgressBar>,
) -> bool {
    // Base case 1: this is a file or a symbolic link.
    // Use lstat to avoid race condition between check and use
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_dir() => {
            return remove_file(path, options, progress_bar);
        }
        Ok(_) => {}
        Err(e) => {
            return show_removal_error(e, path);
        }
    }

    // Try to open the directory using DirFd for secure traversal
    let dir_fd = match DirFd::open(path) {
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

    let error = safe_remove_dir_recursive_impl(path, &dir_fd, options);

    // After processing all children, remove the directory itself
    if error {
        error
    } else {
        // Ask user permission if needed
        if options.interactive == InteractiveMode::Always && !prompt_dir(path, options) {
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
        remove_dir_with_special_cases(path, options, error)
    }
}

pub fn safe_remove_dir_recursive_impl(path: &Path, dir_fd: &DirFd, options: &Options) -> bool {
    // Read directory entries using safe traversal
    clear_errno();
    let entries = match dir_fd.read_dir() {
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

    // Check if readdir failed partway through (partial read)
    if let Some(err) = take_errno() {
        if !entries.is_empty() {
            show_error!(
                "{}: {}",
                translate!("rm-error-traversal-failed", "path" => path.display()),
                err
            );
            return true;
        }
    }

    let mut error = false;

    // Process each entry
    for entry_name in entries {
        let entry_path = path.join(&entry_name);

        // Get metadata for the entry using fstatat
        let entry_stat = match dir_fd.stat_at(&entry_name, false) {
            Ok(stat) => stat,
            Err(e) => {
                error = handle_error_with_force(e, &entry_path, options);
                continue;
            }
        };

        // Check if it's a directory
        let is_dir = (entry_stat.st_mode & libc::S_IFMT) == libc::S_IFDIR;

        if is_dir {
            // Ask user if they want to descend into this directory
            if options.interactive == InteractiveMode::Always
                && !is_dir_empty(&entry_path)
                && !prompt_descend(&entry_path)
            {
                continue;
            }

            // Recursively remove subdirectory using safe traversal
            let child_dir_fd = match dir_fd.open_subdir(&entry_name) {
                Ok(fd) => fd,
                Err(e) => {
                    // If we can't open the subdirectory for safe traversal,
                    // try to handle it as best we can with safe operations
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        error = handle_permission_denied(
                            dir_fd,
                            entry_name.as_ref(),
                            &entry_path,
                            options,
                        );
                    } else {
                        error = handle_error_with_force(e, &entry_path, options);
                    }
                    continue;
                }
            };

            let child_error = safe_remove_dir_recursive_impl(&entry_path, &child_dir_fd, options);
            error = error || child_error;

            // Ask user permission if needed for this subdirectory
            if !child_error
                && options.interactive == InteractiveMode::Always
                && !prompt_dir(&entry_path, options)
            {
                continue;
            }

            // Remove the now-empty subdirectory using safe unlinkat
            if !child_error {
                error = handle_unlink(dir_fd, entry_name.as_ref(), &entry_path, true, options);
            }
        } else {
            // Remove file - check if user wants to remove it first
            if prompt_file(&entry_path, options) {
                error = handle_unlink(dir_fd, entry_name.as_ref(), &entry_path, false, options);
            }
        }
    }

    error
}
