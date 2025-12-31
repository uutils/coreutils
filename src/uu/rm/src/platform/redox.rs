// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Redox-specific stub implementations for the rm utility
//
// Redox OS does not support the safe_traversal module from uucore due to
// missing support for certain Unix syscalls (fstatat, etc.). This module
// provides stub implementations that signal safe traversal is unavailable,
// allowing rm to fall back to standard filesystem operations.

use indicatif::ProgressBar;
use std::path::Path;

use super::super::Options;

/// Remove a single file using safe traversal (STUB for Redox)
///
/// On Redox, safe traversal is not available because the platform lacks support
/// for the fstatat and related syscalls used by uucore::safe_traversal.
///
/// Returns None to signal that safe traversal is unavailable, which causes
/// the caller to fall back to std::fs::remove_file.
pub fn safe_remove_file(
    _path: &Path,
    _options: &Options,
    _progress_bar: Option<&ProgressBar>,
) -> Option<bool> {
    None
}

/// Remove an empty directory using safe traversal (STUB for Redox)
///
/// On Redox, safe traversal is not available because the platform lacks support
/// for the fstatat and related syscalls used by uucore::safe_traversal.
///
/// Returns None to signal that safe traversal is unavailable, which causes
/// the caller to fall back to std::fs::remove_dir.
pub fn safe_remove_empty_dir(
    _path: &Path,
    _options: &Options,
    _progress_bar: Option<&ProgressBar>,
) -> Option<bool> {
    None
}

/// Recursively remove a directory (STUB for Redox)
///
/// On Redox, safe traversal is not available because the platform lacks support
/// for the fstatat and related syscalls used by uucore::safe_traversal.
///
/// This stub returns true (error) to indicate that safe recursive removal is not
/// supported on Redox. A full implementation using standard filesystem operations
/// will be provided in a future PR.
///
/// Note: Unlike safe_remove_file and safe_remove_empty_dir which return Option<bool>,
/// this function returns bool directly because it's called without Option handling.
pub fn safe_remove_dir_recursive(
    _path: &Path,
    _options: &Options,
    _progress_bar: Option<&ProgressBar>,
) -> bool {
    // Return true to indicate error - safe traversal not supported
    // TODO: Implement full recursive removal using standard fs operations
    true
}
