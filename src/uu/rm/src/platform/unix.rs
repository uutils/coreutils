// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Unix-specific implementations for the rm utility

// spell-checker:ignore fstatat unlinkat statx behaviour

use indicatif::ProgressBar;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, IsTerminal, stdin};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::FromIo;
use uucore::prompt_yes;
use uucore::safe_traversal::{DirEntry, DirEntryType, DirFd, DirIter, SymlinkBehavior};
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
            if e.kind() == io::ErrorKind::PermissionDenied {
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
fn handle_error_with_force(e: io::Error, path: &Path, options: &Options) -> bool {
    // Permission denied errors should be shown even in force mode
    // This matches GNU rm behavior
    if e.kind() == io::ErrorKind::PermissionDenied {
        show_permission_denied_error(path);
        return true;
    }

    if !options.force {
        let e = e.map_err_context(|| translate!("rm-error-cannot-remove", "file" => path.quote()));
        show_error!("{e}");
    }
    !options.force
}

/// Helper to handle permission denied errors.
///
/// `unlink_result` is the result of a prior `unlink_at` attempt on the entry.
/// The caller must perform the unlink before calling this function.
fn handle_permission_denied(
    unlink_result: io::Result<()>,
    entry_path: &Path,
    options: &Options,
    progress_bar: Option<&ProgressBar>,
) -> bool {
    // When we can't open a subdirectory due to permission denied,
    // try to remove it directly (it might be empty).
    // This matches GNU rm behavior with -f flag.
    if unlink_result.is_err() {
        // The directory is not empty (or another error) and we can't read it
        // to remove its contents. Report the original permission denied error.
        // This matches GNU rm behavior — the real problem is we lack
        // permission to traverse the directory.
        show_permission_denied_error(entry_path);
        return true;
    }
    // Successfully removed empty directory
    if let Some(pb) = progress_bar {
        pb.inc(1);
    }
    verbose_removed_directory(entry_path, options);
    false
}

/// Helper to handle unlink operation with error reporting.
///
/// `unlink_result` is the result of a prior `unlink_at` attempt on the entry.
/// The caller must perform the unlink before calling this function.
fn handle_unlink(
    unlink_result: io::Result<()>,
    entry_path: &Path,
    is_dir: bool,
    options: &Options,
    progress_bar: Option<&ProgressBar>,
) -> bool {
    if let Err(e) = unlink_result {
        let e = e
            .map_err_context(|| translate!("rm-error-cannot-remove", "file" => entry_path.quote()));
        show_error!("{e}");
        true
    } else {
        if let Some(pb) = progress_bar {
            pb.inc(1);
        }
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
    let initial_mode = match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_dir() => {
            return remove_file(path, options, progress_bar);
        }
        Ok(metadata) => metadata.permissions().mode(),
        Err(e) => {
            return show_removal_error(e, path);
        }
    };

    // Try to open the directory using DirFd for secure traversal
    let dir_fd = match DirFd::open(path, SymlinkBehavior::Follow) {
        Ok(fd) => fd,
        Err(e) => {
            // If we can't open the directory for safe traversal,
            // handle the error appropriately and try to remove if possible
            if e.kind() == io::ErrorKind::PermissionDenied {
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

    let mut current_path = path.to_path_buf();
    let error = safe_remove_dir_recursive_impl(&mut current_path, dir_fd, options, progress_bar);

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

#[cfg(not(target_os = "redox"))]
/// Returns `true` if the I/O error is EMFILE or ENFILE (too many open files).
fn is_emfile(e: &io::Error) -> bool {
    matches!(e.raw_os_error(), Some(libc::EMFILE) | Some(libc::ENFILE))
}

#[cfg(not(target_os = "redox"))]
/// Source of directory entries for a [`StackFrame`].
///
/// A frame starts as `Live` (fd open, lazy iteration via `DirIter`).  When
/// the process runs out of file descriptors — which limits tree depth to
/// approximately `RLIMIT_NOFILE` (≈ 1 024) — `try_reclaim_fd` demotes the
/// oldest live frame to `Drained` by materialising its remaining entries into
/// a `Vec` and closing the fd.  Subsequent entry-yielding uses the vec; fd
/// operations (stat, unlink, open-child) temporarily re-open the directory
/// from `StackFrame::dir_path`.
enum FrameIter {
    /// The directory fd is open.  All operations go through the [`DirIter`].
    Live(DirIter),
    /// The fd was closed to reclaim it for a deeper level.
    /// Remaining entries are pre-collected.  Operations that need a directory
    /// fd temporarily re-open `StackFrame::dir_path`.
    Drained(std::vec::IntoIter<DirEntry>),
}

#[cfg(not(target_os = "redox"))]
/// One level of the directory stack used by the iterative traversal.
///
/// # File-descriptor budget
///
/// Each frame ordinarily holds exactly **one** open file descriptor: `iter`
/// owns the fd for the directory being traversed and exposes it for
/// `stat_at`, `open_child_iter`, and `unlink_at` operations as well.
///
/// When `RLIMIT_NOFILE` is exhausted, [`try_reclaim_fd`] demotes the oldest
/// `Live` frame to [`FrameIter::Drained`], closing its fd.  Operations on a
/// `Drained` frame re-open the directory from `dir_path` on demand; this
/// costs one extra `openat` per entry in that frame but allows trees of
/// arbitrary depth.
struct StackFrame {
    /// Entry source and directory-operation handle.
    iter: FrameIter,
    /// Accumulates whether any child removal failed.
    had_error: bool,
    /// Permission bits of this directory, stored for the interactive prompt
    /// that fires when the directory is about to be unlinked from its parent.
    /// Not meaningful for the root frame (handled by `safe_remove_dir_recursive`).
    mode: libc::mode_t,
    /// Name of this directory inside its parent, used for `unlinkat` once all
    /// children have been processed.  Empty string for the root frame.
    entry_name: OsString,
    /// Full path to this directory.  Used to re-open the directory when the
    /// frame has been demoted to [`FrameIter::Drained`] and a fd-requiring
    /// operation (stat, unlink, open-child) is needed.
    ///
    /// `None` for child frames that have never been demoted; populated lazily
    /// by [`try_reclaim_fd`] at demotion time, so no path allocation is
    /// incurred for frames that remain `Live` for their entire lifetime.
    /// The root frame always has `Some(path)` set at construction time.
    dir_path: Option<PathBuf>,
}

#[cfg(not(target_os = "redox"))]
/// Demote the oldest `Live` frame (excluding the top) to `Drained`, freeing
/// one file descriptor.
///
/// Returns `true` if a frame was demoted; `false` if all non-top frames are
/// already `Drained` (which means we are genuinely out of fds).
///
/// Materialising the oldest frame first minimises the chance of collecting a
/// large sibling list: for deep linear chains the bottom frames have already
/// exhausted their children before a new level is pushed.
///
/// # Security note
///
/// Once a frame is demoted to `Drained`, subsequent fd-requiring operations
/// (`frame_stat_at`, `frame_open_child`, `frame_unlink_at`) re-open the
/// directory via its stored `dir_path` with `O_NOFOLLOW`.  This protects the
/// **final path component** against a concurrent symlink swap.  Intermediate
/// components of the path are resolved normally — the same limitation present
/// in GNU `rm` under fd pressure.  The window only exists in the EMFILE
/// fallback path, never during normal traversal.
fn try_reclaim_fd(stack: &mut [StackFrame]) -> bool {
    // Leave the top frame (currently active) alone.
    let limit = stack.len().saturating_sub(1);
    for i in 0..limit {
        if !matches!(stack[i].iter, FrameIter::Live(_)) {
            continue;
        }
        // Reconstruct this frame's full path from the root frame's stored
        // path plus the `entry_name` of every intermediate frame.  This is
        // computed before the mutable borrow of `stack[i]` below so that the
        // immutable borrows of `stack[0..=i]` are released first.
        let dir_path = {
            // stack[0] is the root frame and always has `dir_path` set.
            let root = stack[0]
                .dir_path
                .as_ref()
                .expect("root StackFrame always has dir_path set");
            let mut p = root.clone();
            for frame in &stack[1..=i] {
                p.push(&frame.entry_name);
            }
            p
        };
        // Drain the remaining entries into a Vec so the fd can be closed.
        // If readdir itself fails mid-drain we cannot enumerate the rest of
        // this directory's children.  Report the error and mark the frame as
        // failed so the directory is not unlinked (it would be non-empty).
        let mut remaining = Vec::new();
        let mut had_error = false;
        if let FrameIter::Live(ref mut di) = stack[i].iter {
            for result in di.by_ref() {
                match result {
                    Ok(entry) => remaining.push(entry),
                    Err(e) => {
                        show_error!(
                            "{}",
                            e.map_err_context(|| translate!(
                                "rm-error-cannot-remove",
                                "file" => dir_path.quote()
                            ))
                        );
                        had_error = true;
                        break;
                    }
                }
            }
        }
        stack[i].had_error |= had_error;
        stack[i].iter = FrameIter::Drained(remaining.into_iter());
        stack[i].dir_path = Some(dir_path);
        return true;
    }
    false
}

#[cfg(not(target_os = "redox"))]
/// Stat an entry relative to the given frame's directory.
///
/// If the frame is [`FrameIter::Live`] the existing fd is used.  If it is
/// [`FrameIter::Drained`] the directory is temporarily re-opened from
/// `frame.dir_path` with `O_NOFOLLOW`, which protects the final path
/// component against a concurrent symlink swap.
fn frame_stat_at(frame: &StackFrame, name: &OsStr) -> io::Result<libc::stat> {
    match &frame.iter {
        FrameIter::Live(di) => di.stat_at(name, SymlinkBehavior::NoFollow),
        FrameIter::Drained(_) => DirFd::open(
            frame
                .dir_path
                .as_deref()
                .expect("Drained frame always has dir_path set"),
            SymlinkBehavior::NoFollow,
        )
        .and_then(|fd| fd.stat_at(name, SymlinkBehavior::NoFollow)),
    }
}

#[cfg(not(target_os = "redox"))]
/// Open a child directory relative to the given frame's directory, returning
/// a new [`DirIter`] that owns exactly one fd.
///
/// If the frame is [`FrameIter::Live`] the existing fd is used via
/// `open_child_iter` (no dup).  If it is [`FrameIter::Drained`] the parent
/// directory is temporarily re-opened from `frame.dir_path` with `O_NOFOLLOW`,
/// which protects the final path component against a concurrent symlink swap.
///
/// In both cases the child is opened with `O_NOFOLLOW` to prevent symlink
/// substitution attacks.
fn frame_open_child(frame: &StackFrame, name: &OsStr) -> io::Result<DirIter> {
    match &frame.iter {
        FrameIter::Live(di) => di.open_child_iter(name, SymlinkBehavior::NoFollow),
        FrameIter::Drained(_) => DirFd::open(
            frame
                .dir_path
                .as_deref()
                .expect("Drained frame always has dir_path set"),
            SymlinkBehavior::NoFollow,
        )
        .and_then(|fd| fd.open_subdir(name, SymlinkBehavior::NoFollow))
        .and_then(DirFd::into_iter_dir),
    }
}

#[cfg(not(target_os = "redox"))]
/// Unlink an entry relative to the given frame's directory.
///
/// If the frame is [`FrameIter::Live`] the existing fd is used.  If it is
/// [`FrameIter::Drained`] the directory is temporarily re-opened from
/// `frame.dir_path` with `O_NOFOLLOW`, which protects the final path
/// component against a concurrent symlink swap.
fn frame_unlink_at(frame: &StackFrame, name: &OsStr, is_dir: bool) -> io::Result<()> {
    match &frame.iter {
        FrameIter::Live(di) => di.unlink_at(name, is_dir),
        FrameIter::Drained(_) => DirFd::open(
            frame
                .dir_path
                .as_deref()
                .expect("Drained frame always has dir_path set"),
            SymlinkBehavior::NoFollow,
        )
        .and_then(|fd| fd.unlink_at(name, is_dir)),
    }
}

#[cfg(not(target_os = "redox"))]
pub(super) fn safe_remove_dir_recursive_impl(
    current_path: &mut PathBuf,
    dir_fd: DirFd,
    options: &Options,
    progress_bar: Option<&ProgressBar>,
) -> bool {
    // Iterative, allocation-minimising implementation.
    //
    // Design goals (see https://github.com/uutils/coreutils/issues/11222):
    //   • One shared `PathBuf` for the entire traversal instead of one `join()`
    //     allocation per child entry.
    //   • Explicit `Vec<StackFrame>` work-stack instead of recursive call frames,
    //     removing one heap allocation per directory level and preventing stack
    //     overflow on deep trees.
    //   • Lazy `DirIter` (backed by `nix::dir::OwningIter`) instead of the eager
    //     `read_dir()` helper, avoiding the intermediate `Vec<OsString>` per level.
    //   • Single fd per stack frame: `DirFd::into_iter_dir` transfers ownership
    //     directly into the iterator without `dup(2)`.
    //
    // Fd budget: each `Live` frame holds exactly one open fd.  When the process
    // runs out of file descriptors (`EMFILE`/`ENFILE`), `try_reclaim_fd` demotes
    // the oldest live frame to `Drained` — its remaining entries are materialised
    // into a `Vec<OsString>` and its fd is closed.  Subsequent fd-requiring
    // operations on a `Drained` frame re-open the directory from `dir_path` on
    // demand.  This allows traversal of trees of arbitrary depth at the cost of
    // one extra `openat(2)` per entry in a drained frame.

    // Obtain the initial iterator via into_iter_dir (consuming, no dup).
    let root_iter = match dir_fd.into_iter_dir() {
        Ok(it) => it,
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            if !options.force {
                show_permission_denied_error(current_path.as_path());
            }
            return !options.force;
        }
        Err(e) => return handle_error_with_force(e, current_path.as_path(), options),
    };

    // Pre-size to a reasonable depth to avoid early reallocations.
    let mut stack: Vec<StackFrame> = Vec::with_capacity(32);
    stack.push(StackFrame {
        iter: FrameIter::Live(root_iter),
        had_error: false,
        mode: 0, // root mode is not used here; handled by safe_remove_dir_recursive
        entry_name: OsString::new(),
        dir_path: Some(current_path.clone()),
    });

    loop {
        // Pull the next child from the top frame's iterator.
        // The mutable borrow of `stack` is released as soon as `next()` returns
        // an owned value, so subsequent borrows within the match arms are allowed.
        let entry = match stack.last_mut() {
            None => unreachable!(
                "stack is non-empty at every iteration: the root frame is only \
                 removed in the `None` arm below, which returns immediately"
            ),
            Some(frame) => match &mut frame.iter {
                FrameIter::Live(di) => di.next(),
                FrameIter::Drained(it) => it.next().map(Ok),
            },
        };

        match entry {
            // ── All children of the current directory have been processed ──────
            None => {
                let completed = stack.pop().unwrap();

                if stack.is_empty() {
                    // Root frame exhausted — return its error flag to the caller.
                    // current_path is still the root path; no pop needed.
                    return completed.had_error;
                }

                // current_path == path of the completed (child) directory here.
                let child_error = completed.had_error;
                stack.last_mut().unwrap().had_error |= child_error;

                // Only unlink this directory if all its children were removed.
                if !child_error {
                    if options.interactive == InteractiveMode::Always
                        && !prompt_dir_with_mode(current_path.as_path(), completed.mode, options)
                    {
                        // User declined this subdirectory.  We intentionally do
                        // NOT propagate an error to the parent frame here: GNU rm
                        // only fails when an *unlink* syscall fails, not when the
                        // user explicitly skips a removal.  If the parent itself
                        // cannot be removed later (because it is now non-empty),
                        // the caller `safe_remove_dir_recursive` detects the
                        // non-empty condition via `is_dir_empty` and silently
                        // skips it — matching GNU rm's exit-0 behaviour in that
                        // situation.
                        current_path.pop();
                        continue;
                    }
                    // Unlink the completed child directory from its parent.
                    // `stack.last()` is now the parent frame; `frame_unlink_at`
                    // handles both Live (fd already open) and Drained (re-open
                    // from dir_path) cases.
                    let unlink_result =
                        frame_unlink_at(stack.last().unwrap(), completed.entry_name.as_ref(), true);
                    let unlink_err = handle_unlink(
                        unlink_result,
                        current_path.as_path(),
                        true,
                        options,
                        progress_bar,
                    );
                    stack.last_mut().unwrap().had_error |= unlink_err;
                }

                // Restore current_path to the parent directory.
                current_path.pop();
            }

            // ── readdir returned an I/O error mid-iteration ───────────────────
            Some(Err(e)) => {
                // current_path is the directory being iterated; no child was pushed.
                let err = handle_error_with_force(e, current_path.as_path(), options);
                stack.last_mut().unwrap().had_error |= err;
            }

            // ── Normal child entry ────────────────────────────────────────────
            Some(Ok(entry)) => {
                // Extend the shared path accumulator in-place — amortised O(1),
                // no heap allocation if there is spare capacity.
                current_path.push(&entry.name);

                // Determine whether this entry is a directory and, if needed,
                // obtain its full stat.
                //
                // Fast path (non-interactive, d_type known): use the file-type
                // hint from `getdents`/`d_type` to skip the `fstatat` syscall.
                // On modern Linux filesystems (ext4, xfs, btrfs, tmpfs, …) this
                // is always available, eliminating one syscall per entry in the
                // common case.
                //
                // Fallback: stat when in interactive mode (need mode/size for
                // the prompt) or when the filesystem reports DT_UNKNOWN.
                let needs_stat = options.interactive != InteractiveMode::Never
                    || entry.file_type.is_none();

                let (is_dir, stat_opt) = if needs_stat {
                    match frame_stat_at(stack.last().unwrap(), entry.name.as_ref()) {
                        Ok(s) => {
                            let is_dir = ((s.st_mode as libc::mode_t) & libc::S_IFMT)
                                == libc::S_IFDIR;
                            (is_dir, Some(s))
                        }
                        Err(e) => {
                            let err =
                                handle_error_with_force(e, current_path.as_path(), options);
                            stack.last_mut().unwrap().had_error |= err;
                            current_path.pop();
                            continue;
                        }
                    }
                } else {
                    (
                        matches!(entry.file_type, Some(DirEntryType::Directory)),
                        None,
                    )
                };

                if is_dir {
                    // Interactive: ask whether to descend into this directory.
                    //
                    // Note: is_dir_empty resolves `current_path` through a fresh
                    // path-based read_dir(2) call, then closes the fd before
                    // open_child_iter opens the directory again below.  There is a
                    // narrow TOCTOU window between these two operations where a
                    // concurrent process could modify the directory's contents,
                    // causing the prompt to be shown (or skipped) incorrectly.
                    // This is pre-existing behaviour inherited from GNU rm, which
                    // performs the same non-atomic check.  The window is limited to
                    // interactive mode and carries no security consequence: even if
                    // the directory is swapped, open_child_iter uses O_NOFOLLOW so
                    // symlink substitution is rejected by the kernel.
                    if options.interactive == InteractiveMode::Always
                        && !is_dir_empty(current_path.as_path())
                        && !prompt_descend(current_path.as_path())
                    {
                        current_path.pop();
                        continue;
                    }

                    // Open the child directory for safe traversal.
                    // Use NoFollow: the entry was already confirmed to be a real
                    // directory (not a symlink) by stat_at(NoFollow) above, or by
                    // d_type == DT_DIR (which the kernel sets from the inode, not
                    // the symlink target).  Using Follow would re-introduce a TOCTOU
                    // window — a concurrent process could replace the directory with
                    // a symlink between the stat and the open, causing rm to traverse
                    // an unintended target.
                    //
                    // EMFILE / ENFILE recovery: if the process has run out of file
                    // descriptors, demote the oldest Live frame to Drained (freeing
                    // one fd) and retry.  This allows trees deeper than RLIMIT_NOFILE
                    // to be traversed at the cost of one extra openat(2) per entry in
                    // the drained frame.
                    let child_iter =
                        match frame_open_child(stack.last().unwrap(), entry.name.as_ref()) {
                            Ok(it) => it,
                            Err(e) if is_emfile(&e) => {
                                if try_reclaim_fd(&mut stack) {
                                    // Retry with the freed fd.
                                    match frame_open_child(
                                        stack.last().unwrap(),
                                        entry.name.as_ref(),
                                    ) {
                                        Ok(it) => it,
                                        Err(e) => {
                                            let err = handle_error_with_force(
                                                e,
                                                current_path.as_path(),
                                                options,
                                            );
                                            stack.last_mut().unwrap().had_error |= err;
                                            current_path.pop();
                                            continue;
                                        }
                                    }
                                } else {
                                    // No frames to reclaim — genuinely out of fds.
                                    let err =
                                        handle_error_with_force(e, current_path.as_path(), options);
                                    stack.last_mut().unwrap().had_error |= err;
                                    current_path.pop();
                                    continue;
                                }
                            }
                            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                                // Attempt to remove it directly in case it is empty.
                                let unlink_result = frame_unlink_at(
                                    stack.last().unwrap(),
                                    entry.name.as_ref(),
                                    true,
                                );
                                let err = handle_permission_denied(
                                    unlink_result,
                                    current_path.as_path(),
                                    options,
                                    progress_bar,
                                );
                                stack.last_mut().unwrap().had_error |= err;
                                current_path.pop();
                                continue;
                            }
                            Err(e) => {
                                let err =
                                    handle_error_with_force(e, current_path.as_path(), options);
                                stack.last_mut().unwrap().had_error |= err;
                                current_path.pop();
                                continue;
                            }
                        };

                    // Push the child frame.  current_path now ends with entry_name
                    // and will be popped when this frame is exhausted (None arm).
                    // dir_path is None — populated lazily by try_reclaim_fd only
                    // if this frame is ever demoted to Drained (EMFILE fallback).
                    // mode is 0 when stat_opt is None (non-interactive path); it is
                    // only consumed by prompt_dir_with_mode which is guarded by
                    // `interactive == Always`, so the zero is never observed.
                    stack.push(StackFrame {
                        iter: FrameIter::Live(child_iter),
                        had_error: false,
                        mode: stat_opt.map_or(0, |s| s.st_mode as libc::mode_t),
                        entry_name: entry.name,
                        dir_path: None,
                    });
                } else {
                    // File or symlink: prompt then unlink.
                    // In Never mode stat_opt is None and we always remove.
                    let should_remove = match stat_opt {
                        Some(ref s) => {
                            prompt_file_with_stat(current_path.as_path(), s, options)
                        }
                        None => true,
                    };
                    if should_remove {
                        let unlink_result =
                            frame_unlink_at(stack.last().unwrap(), entry.name.as_ref(), false);
                        let err = handle_unlink(
                            unlink_result,
                            current_path.as_path(),
                            false,
                            options,
                            progress_bar,
                        );
                        stack.last_mut().unwrap().had_error |= err;
                    }
                    // Restore path after processing the file/symlink.
                    current_path.pop();
                }
            }
        }
    }
}

#[cfg(target_os = "redox")]
pub(super) fn safe_remove_dir_recursive_impl(
    _current_path: &mut PathBuf,
    _dir_fd: DirFd,
    _options: &Options,
    _progress_bar: Option<&ProgressBar>,
) -> bool {
    // safe_traversal stat_at is not supported on Redox
    // This shouldn't be called on Redox, but provide a stub for compilation
    true // Return error
}
