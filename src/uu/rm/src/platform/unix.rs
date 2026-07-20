// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Unix-specific implementations for the rm utility

// spell-checker:ignore fstatat unlinkat statx behaviour NOFILE PATH_MAX

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

/// Soft cap on simultaneously open directory FDs during recursive rm.
///
/// Shallow trees (CodSpeed `rm_recursive_tree`, depth 5) use the same recursive
/// walk as before and stay under this budget, so parent `DirFd`s remain open and
/// there is no `openat("..")` churn. Past the budget we switch the remaining
/// subtree to an O(1)-FD iterative walk (#7995) that closes before descend and
/// restores parents with `openat(child, "..")` + `O_NOFOLLOW` (GNU `rm/deep-2`).
/// Keep well below the low-NOFILE regression soft limit (32) after stdio.
const DIR_FD_BUDGET: usize = 16;

/// Frame for the deep (past-budget) iterative walk.
struct DirWalkFrame {
    path: std::path::PathBuf,
    dir_fd: Option<DirFd>,
    dir_dev: u64,
    /// Identity of this directory when first opened (device + inode).
    /// Checked after `openat(child, "..")` so a moved parent is not unlinked into.
    dir_ino: u64,
    pending: Vec<std::ffi::OsString>,
    error: bool,
    mode: libc::mode_t,
    name_in_parent: std::ffi::OsString,
}

/// Re-open the parent of `child` without using the absolute path.
///
/// Deep trees (GNU `rm/deep-2`) exceed `PATH_MAX`, so path reopen fails with
/// "file name too long". `openat(child, "..")` stays relative.
///
/// Use `NoFollow`: the security harness requires every relative directory
/// `openat` (not the top-level command path) to carry `O_NOFOLLOW`.
fn reopen_parent_from_child(child: &DirFd) -> std::io::Result<DirFd> {
    child.open_subdir(OsStr::new(".."), SymlinkBehavior::NoFollow)
}

/// Restore the parent directory FD and confirm it is still the same directory.
///
/// Closing the parent before a deep descend leaves a window where that directory
/// can be renamed/moved. After `openat(child, "..")` (or a path reopen), `fstat`
/// the result and require the same device + inode we recorded on the way down.
fn reopen_parent_checked(
    child: Option<&DirFd>,
    parent_path: &Path,
    expected_dev: u64,
    expected_ino: u64,
) -> std::io::Result<DirFd> {
    let parent_fd = match child {
        Some(fd) => reopen_parent_from_child(fd)?,
        None => DirFd::open(parent_path, SymlinkBehavior::NoFollow)?,
    };
    let st = parent_fd.fstat()?;
    #[allow(clippy::unnecessary_cast)]
    let got_dev = st.st_dev as u64;
    #[allow(clippy::unnecessary_cast)]
    let got_ino = st.st_ino as u64;
    if got_dev != expected_dev || got_ino != expected_ino {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "directory changed while removing",
        ));
    }
    Ok(parent_fd)
}

#[cfg(not(target_os = "redox"))]
pub fn safe_remove_dir_recursive_impl(
    path: &Path,
    dir_fd: &DirFd,
    options: &Options,
    root_dev: u64,
    parent_dev: u64,
) -> bool {
    safe_remove_dir_recursive_impl_depth(path, dir_fd, options, root_dev, parent_dev, 0)
}

/// Recursive walk matching pre-#7995 structure while `depth < DIR_FD_BUDGET`.
///
/// Once the next level would exceed the budget, the remaining subtree is removed
/// with [`safe_remove_dir_deep_o1`] so open directory FDs stay bounded.
#[cfg(not(target_os = "redox"))]
fn safe_remove_dir_recursive_impl_depth(
    path: &Path,
    dir_fd: &DirFd,
    options: &Options,
    root_dev: u64,
    parent_dev: u64,
    depth: usize,
) -> bool {
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

    let mut error = false;

    for entry_name in entries {
        let entry_path = path.join(&entry_name);

        let entry_stat = match dir_fd.stat_at(&entry_name, SymlinkBehavior::NoFollow) {
            Ok(stat) => stat,
            Err(e) => {
                error |= handle_error_with_force(e, &entry_path, options);
                continue;
            }
        };

        let is_dir = ((entry_stat.st_mode as libc::mode_t) & libc::S_IFMT) == libc::S_IFDIR;

        if is_dir {
            #[allow(clippy::unnecessary_cast)]
            let entry_dev = entry_stat.st_dev as u64;

            if options.one_fs && entry_dev != root_dev {
                show_error!(
                    "{}",
                    translate!("rm-error-skipping-different-device", "file" => entry_path.quote())
                );
                error = true;
                continue;
            }

            if options.preserve_root_all && entry_dev != parent_dev {
                show_preserve_root_all_skip(&entry_path);
                error = true;
                continue;
            }

            if options.interactive == InteractiveMode::Always
                && !is_dir_empty(&entry_path)
                && !prompt_descend(&entry_path)
            {
                continue;
            }

            let child_dir_fd = match dir_fd.open_subdir(&entry_name, SymlinkBehavior::NoFollow) {
                Ok(fd) => fd,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        error |= handle_permission_denied(
                            dir_fd,
                            entry_name.as_ref(),
                            &entry_path,
                            options,
                        );
                    } else {
                        error |= handle_error_with_force(e, &entry_path, options);
                    }
                    continue;
                }
            };

            // Shallow: same recursive shape as main (parent DirFd stays open).
            // Deep: iterative O(1) FD walk for the remaining subtree only.
            let child_error = if depth + 1 < DIR_FD_BUDGET {
                safe_remove_dir_recursive_impl_depth(
                    &entry_path,
                    &child_dir_fd,
                    options,
                    root_dev,
                    entry_dev,
                    depth + 1,
                )
            } else {
                safe_remove_dir_deep_o1(
                    &entry_path,
                    child_dir_fd,
                    options,
                    root_dev,
                    entry_dev,
                    entry_stat.st_mode as libc::mode_t,
                    entry_name.clone(),
                )
            };
            error |= child_error;

            if !child_error
                && options.interactive == InteractiveMode::Always
                && !prompt_dir_with_mode(&entry_path, entry_stat.st_mode as libc::mode_t, options)
            {
                continue;
            }

            if !child_error {
                error |= handle_unlink(dir_fd, entry_name.as_ref(), &entry_path, true, options);
            }
        } else if prompt_file_with_stat(&entry_path, &entry_stat, options) {
            error |= handle_unlink(dir_fd, entry_name.as_ref(), &entry_path, false, options);
        }
    }

    error
}

/// Iterative remove for subtrees that already sit at [`DIR_FD_BUDGET`].
///
/// Always closes the parent before descending and restores it with
/// `openat(child, "..")` plus a device/inode check so open directory FDs stay
/// O(1) for pathological depth without following a moved parent.
#[cfg(not(target_os = "redox"))]
fn safe_remove_dir_deep_o1(
    path: &Path,
    root_fd: DirFd,
    options: &Options,
    root_dev: u64,
    dir_dev: u64,
    mode: libc::mode_t,
    name_in_parent: std::ffi::OsString,
) -> bool {
    let root_entries = match root_fd.read_dir() {
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

    let mut root_pending = root_entries;
    root_pending.reverse();

    let root_ino = match root_fd.fstat() {
        Ok(st) => {
            #[allow(clippy::unnecessary_cast)]
            {
                st.st_ino as u64
            }
        }
        Err(e) => {
            return handle_error_with_force(e, path, options);
        }
    };

    let mut stack = vec![DirWalkFrame {
        path: path.to_path_buf(),
        dir_fd: Some(root_fd),
        dir_dev,
        dir_ino: root_ino,
        pending: root_pending,
        error: false,
        mode,
        name_in_parent,
    }];

    let mut had_error = false;

    while !stack.is_empty() {
        let frame_idx = stack.len() - 1;

        if stack[frame_idx].pending.is_empty() {
            let frame = stack.pop().unwrap();
            let child_error = frame.error;
            let child_path = frame.path;
            let child_mode = frame.mode;
            let child_name = frame.name_in_parent;
            let mut child_fd = frame.dir_fd;

            if stack.is_empty() {
                // Subtree root: caller unlinks this directory from its parent.
                had_error = child_error;
                break;
            }

            let parent_idx = stack.len() - 1;
            // Always closed parent on descend in deep walk — restore via ".." and
            // check the recorded parent identity (device + inode) after fstat.
            if stack[parent_idx].dir_fd.is_none() {
                let expected_dev = stack[parent_idx].dir_dev;
                let expected_ino = stack[parent_idx].dir_ino;
                let parent_path = stack[parent_idx].path.clone();
                match reopen_parent_checked(
                    child_fd.as_ref(),
                    &parent_path,
                    expected_dev,
                    expected_ino,
                ) {
                    Ok(parent_fd) => stack[parent_idx].dir_fd = Some(parent_fd),
                    Err(e) => {
                        stack[parent_idx].error |=
                            handle_error_with_force(e, &stack[parent_idx].path, options);
                        continue;
                    }
                }
            }
            child_fd = None;
            drop(child_fd);

            if child_error {
                stack[parent_idx].error = true;
            } else if options.interactive == InteractiveMode::Always
                && !prompt_dir_with_mode(&child_path, child_mode, options)
            {
                continue;
            } else if let Some(parent_fd) = stack[parent_idx].dir_fd.as_ref() {
                stack[parent_idx].error |=
                    handle_unlink(parent_fd, child_name.as_ref(), &child_path, true, options);
            } else {
                stack[parent_idx].error = true;
            }
            continue;
        }

        let Some(entry_name) = stack[frame_idx].pending.pop() else {
            continue;
        };
        let entry_path = stack[frame_idx].path.join(&entry_name);

        if stack[frame_idx].dir_fd.is_none() {
            match DirFd::open(&stack[frame_idx].path, SymlinkBehavior::NoFollow) {
                Ok(fd) => match fd.fstat() {
                    Ok(st) => {
                        #[allow(clippy::unnecessary_cast)]
                        let got_dev = st.st_dev as u64;
                        #[allow(clippy::unnecessary_cast)]
                        let got_ino = st.st_ino as u64;
                        if got_dev != stack[frame_idx].dir_dev
                            || got_ino != stack[frame_idx].dir_ino
                        {
                            stack[frame_idx].error |= handle_error_with_force(
                                std::io::Error::new(
                                    std::io::ErrorKind::NotFound,
                                    "directory changed while removing",
                                ),
                                &stack[frame_idx].path,
                                options,
                            );
                            stack[frame_idx].pending.clear();
                            continue;
                        }
                        stack[frame_idx].dir_fd = Some(fd);
                    }
                    Err(e) => {
                        stack[frame_idx].error |=
                            handle_error_with_force(e, &stack[frame_idx].path, options);
                        stack[frame_idx].pending.clear();
                        continue;
                    }
                },
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

            if options.preserve_root_all && entry_dev != parent_dev_for_child {
                show_preserve_root_all_skip(&entry_path);
                stack[frame_idx].error = true;
                continue;
            }

            if options.interactive == InteractiveMode::Always
                && !is_dir_empty(&entry_path)
                && !prompt_descend(&entry_path)
            {
                continue;
            }

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

            // Always close parent before deep descend (already past budget).
            stack[frame_idx].dir_fd = None;

            let mut child_pending = child_entries;
            child_pending.reverse();

            #[allow(clippy::unnecessary_cast)]
            let entry_ino = entry_stat.st_ino as u64;

            stack.push(DirWalkFrame {
                path: entry_path,
                dir_fd: Some(child_dir_fd),
                dir_dev: entry_dev,
                dir_ino: entry_ino,
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
