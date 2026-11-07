// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore hardlinked

//! Hardlink preservation utilities for mv operations
//!
//! This module provides functionality to preserve hardlink relationships
//! when moving files across different filesystems/partitions.

use rustc_hash::FxHashMap;
use std::cell::OnceCell;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use uucore::display::Quotable;
use uucore::translate;

/// Tracks hardlinks during cross-partition moves to preserve them
#[derive(Debug, Default)]
pub struct HardlinkTracker {
    /// Maps (device, inode) -> destination path for the first occurrence
    inode_map: FxHashMap<(u64, u64), PathBuf>,
}

type HardlinkGroups = FxHashMap<(u64, u64), Vec<PathBuf>>;

/// Groups the files being moved by hardlink, scanning them on first use
#[derive(Debug, Default)]
pub struct HardlinkGroupScanner {
    /// Maps (device, inode) -> list of source paths that are hardlinked together.
    /// Only filled when a copy needs it, so a move that is just a rename never
    /// walks the source tree.
    hardlink_groups: OnceCell<HardlinkGroups>,
    /// List of source files/directories being moved
    source_files: Vec<PathBuf>,
    verbose: bool,
}

/// Configuration options for hardlink preservation
#[derive(Debug, Clone, Default)]
pub struct HardlinkOptions {
    /// Whether to show verbose output about hardlink operations
    pub verbose: bool,
}

/// Errors that can occur during hardlink operations
#[derive(Debug, thiserror::Error)]
pub enum HardlinkError {
    #[error("{}", translate!("mv-error-hardlink-io", "error" => .0))]
    Io(#[from] io::Error),
    #[error("{}", translate!("mv-error-hardlink-scan", "error" => .0))]
    Scan(String),
    #[error("{}", translate!("mv-error-hardlink-preserve", "source" => .source_path.quote(), "target" => .target_path.quote()))]
    Preservation {
        source_path: PathBuf,
        target_path: PathBuf,
    },
    #[error("{}", translate!("mv-error-hardlink-metadata", "path" => .path.quote(), "error" => .error))]
    Metadata {
        path: PathBuf,
        #[source]
        error: io::Error,
    },
}

impl From<HardlinkError> for io::Error {
    fn from(error: HardlinkError) -> Self {
        match error {
            HardlinkError::Io(e) => e,
            HardlinkError::Scan(msg) => Self::other(msg),
            other => Self::other(other.to_string()),
        }
    }
}

impl HardlinkTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a file is a hardlink we've seen before, and return the target path if so
    pub fn check_hardlink(
        &mut self,
        source: &Path,
        dest: &Path,
        scanner: &HardlinkGroupScanner,
        options: &HardlinkOptions,
    ) -> Option<PathBuf> {
        use std::os::unix::fs::MetadataExt;

        // Scan now, before this first copy removes its source, so that every
        // member of a hardlink group is still in place.
        let groups = scanner.groups();

        let metadata = match source.symlink_metadata() {
            Ok(meta) => meta,
            Err(e) => {
                // Gracefully handle metadata errors by logging and continuing without hardlink tracking
                if options.verbose {
                    let _ = writeln!(
                        io::stderr(),
                        "warning: cannot get metadata for {}: {e}",
                        source.quote()
                    );
                }
                return None;
            }
        };

        let key = (metadata.dev(), metadata.ino());

        // Check if we've already processed a file with this inode
        if let Some(existing_path) = self.inode_map.get(&key) {
            // Check if this file is part of a hardlink group from the scanner
            let has_hardlinks = groups.get(&key).is_some_and(|group| group.len() > 1);

            if has_hardlinks {
                if options.verbose {
                    let _ = writeln!(
                        io::stderr(),
                        "preserving hardlink {} -> {} (hardlinked)",
                        source.quote(),
                        existing_path.quote()
                    );
                }
                return Some(existing_path.clone());
            }
        }

        // This is the first time we see this file, record its destination
        self.inode_map.insert(key, dest.to_path_buf());

        None
    }
}

impl HardlinkGroupScanner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remember the files to scan. The scan itself runs the first time the
    /// groups are needed, which only happens when `rename` fails and the
    /// files have to be copied.
    pub fn scan_files(&mut self, files: &[PathBuf], options: &HardlinkOptions) {
        self.source_files = files.to_vec();
        self.verbose = options.verbose;
    }

    fn groups(&self) -> &HardlinkGroups {
        self.hardlink_groups.get_or_init(|| self.scan())
    }

    /// Scan the source files and group them by hardlinks, including recursive directory scanning
    fn scan(&self) -> HardlinkGroups {
        let mut groups = HardlinkGroups::default();

        for file in &self.source_files {
            match Self::scan_single_path(file, &mut groups) {
                // A source that is gone was already renamed by this mv, or
                // never existed, which mv reports on its own.
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                // Only show warnings for verbose mode. Otherwise continue
                // silently: we lose hardlink info for this file but can still
                // preserve hardlinks for other files.
                Err(e) if self.verbose => {
                    let _ = writeln!(
                        io::stderr(),
                        "warning: failed to scan {}: {e}",
                        file.quote()
                    );
                }
                _ => {}
            }
        }

        if self.verbose {
            let stats = ScannerStats::of(&groups);
            if stats.total_groups > 0 {
                let _ = writeln!(
                    io::stderr(),
                    "found {} hardlink groups with {} total files",
                    stats.total_groups,
                    stats.total_files
                );
            }
        }

        groups
    }

    /// Scan a single path (file or directory)
    fn scan_single_path(path: &Path, groups: &mut HardlinkGroups) -> io::Result<()> {
        use std::os::unix::fs::MetadataExt;

        let metadata = path.symlink_metadata()?;
        if metadata.is_dir() {
            // Recursively scan directory contents
            Self::scan_directory_recursive(path, groups)?;
        } else if metadata.is_file() && metadata.nlink() > 1 {
            let key = (metadata.dev(), metadata.ino());
            groups.entry(key).or_default().push(path.to_path_buf());
        }
        Ok(())
    }

    /// Recursively scan a directory for hardlinked files
    fn scan_directory_recursive(dir: &Path, groups: &mut HardlinkGroups) -> io::Result<()> {
        use std::os::unix::fs::MetadataExt;

        let entries = std::fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;

            if entry.file_type()?.is_dir() {
                Self::scan_directory_recursive(&entry.path(), groups)?;
            } else {
                let metadata = entry.metadata()?;
                if metadata.is_file() && metadata.nlink() > 1 {
                    let key = (metadata.dev(), metadata.ino());
                    groups.entry(key).or_default().push(entry.path());
                }
            }
        }
        Ok(())
    }

    #[cfg(not(unix))]
    pub fn stats(&self) -> ScannerStats {
        ScannerStats {
            total_groups: 0,
            total_files: 0,
        }
    }

    /// Get statistics about scanned hardlinks
    #[cfg(test)]
    pub fn stats(&self) -> ScannerStats {
        ScannerStats::of(self.groups())
    }
}

/// Statistics about hardlink scanning
#[derive(Debug, Clone)]
pub struct ScannerStats {
    pub total_groups: usize,
    pub total_files: usize,
}

impl ScannerStats {
    fn of(groups: &HardlinkGroups) -> Self {
        Self {
            total_groups: groups.len(),
            total_files: groups.values().map(Vec::len).sum(),
        }
    }
}

/// Create a new hardlink tracker and scanner pair
pub fn create_hardlink_context() -> (HardlinkTracker, HardlinkGroupScanner) {
    (HardlinkTracker::new(), HardlinkGroupScanner::new())
}

/// Convenient function to execute operations with proper hardlink context handling
pub fn with_optional_hardlink_context<F, R>(
    tracker: Option<&mut HardlinkTracker>,
    scanner: Option<&HardlinkGroupScanner>,
    operation: F,
) -> R
where
    F: FnOnce(&mut HardlinkTracker, &HardlinkGroupScanner) -> R,
{
    if let (Some(tracker), Some(scanner)) = (tracker, scanner) {
        operation(tracker, scanner)
    } else {
        let (mut dummy_tracker, dummy_scanner) = create_hardlink_context();
        operation(&mut dummy_tracker, &dummy_scanner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardlink_tracker_creation() {
        let _tracker = HardlinkTracker::new();
        // Just test that creation works
    }

    #[test]
    fn test_scanner_creation() {
        let scanner = HardlinkGroupScanner::new();
        let stats = scanner.stats();
        assert_eq!(stats.total_groups, 0);
        assert_eq!(stats.total_files, 0);
    }

    #[test]
    fn test_create_hardlink_context() {
        let (_tracker, scanner) = create_hardlink_context();
        let stats = scanner.stats();
        assert_eq!(stats.total_groups, 0);
        assert_eq!(stats.total_files, 0);
    }
}
