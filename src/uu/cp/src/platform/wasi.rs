// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (vars) atim mtim

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use rustix::fs::{AtFlags, CWD, FileType, Timespec, Timestamps, lstat, stat, utimensat};
use uucore::buf_copy;
use uucore::display::Quotable;
use uucore::safe_copy::{create_dest_restrictive, open_source};
use uucore::translate;

use crate::{
    CopyDebug, CopyResult, CpError, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode,
};

#[derive(Clone, Copy)]
pub(crate) struct SourceTimesSnapshot {
    times: SourceTimes,
    device: u64,
    inode: u64,
    file_type: FileType,
}

impl SourceTimesSnapshot {
    pub(crate) fn from_path(path: &Path, dereference: bool) -> io::Result<Self> {
        let stat = if dereference {
            stat(path)?
        } else {
            lstat(path)?
        };
        Ok(Self {
            times: SourceTimes::from_stat(&stat),
            device: stat.st_dev,
            inode: stat.st_ino,
            file_type: FileType::from_raw_mode(stat.st_mode),
        })
    }

    pub(crate) fn times_if_unchanged(&self, path: &Path, dereference: bool) -> Option<SourceTimes> {
        self.times_if_matches(path, dereference, !self.file_type.is_symlink())
    }

    fn times_if_matches(
        &self,
        path: &Path,
        dereference: bool,
        compare_accessed: bool,
    ) -> Option<SourceTimes> {
        let stat = if dereference {
            stat(path).ok()?
        } else {
            lstat(path).ok()?
        };
        let file_type = FileType::from_raw_mode(stat.st_mode);
        if stat.st_dev != self.device
            || stat.st_ino != self.inode
            || file_type != self.file_type
            || !self.times.matches_stat(&stat, compare_accessed)
        {
            return None;
        }
        Some(self.times)
    }
}

/// Source directory timestamps captured before recursive traversal opens each directory.
pub(crate) struct DirectoryTimesTracker {
    root_dereference: bool,
    child_dereference: bool,
    snapshots: Rc<RefCell<HashMap<(u64, u64), SourceTimesSnapshot>>>,
}

pub(crate) struct DirectoryTimesSnapshot {
    source: SourceTimesSnapshot,
    dereference: bool,
}

impl DirectoryTimesSnapshot {
    pub(crate) fn times_if_unchanged(&self, path: &Path) -> Option<SourceTimes> {
        // Recursive traversal legitimately opens directories, so only identity,
        // type, and modification time determine whether the snapshot is stale.
        self.source.times_if_matches(path, self.dereference, false)
    }
}

impl DirectoryTimesTracker {
    pub(crate) fn new(root: &Path, root_dereference: bool, child_dereference: bool) -> Self {
        let tracker = Self {
            root_dereference,
            child_dereference,
            snapshots: Rc::new(RefCell::new(HashMap::new())),
        };
        tracker.capture(root, root_dereference);
        tracker
    }

    /// Create trackers that share the earliest snapshot of overlapping source trees.
    pub(crate) fn for_roots(
        roots: &[PathBuf],
        root_dereference: bool,
        child_dereference: bool,
    ) -> Vec<Option<Self>> {
        let snapshots = Rc::new(RefCell::new(HashMap::new()));
        roots
            .iter()
            .map(|root| {
                let tracker = Self {
                    root_dereference,
                    child_dereference,
                    snapshots: Rc::clone(&snapshots),
                };
                tracker.capture(root, root_dereference);
                Some(tracker)
            })
            .collect()
    }

    /// Return the directory's pre-traversal snapshot for final validation.
    pub(crate) fn take(&mut self, path: &Path, depth: usize) -> Option<DirectoryTimesSnapshot> {
        let dereference = if depth == 0 {
            self.root_dereference
        } else {
            self.child_dereference
        };
        let current = SourceTimesSnapshot::from_path(path, dereference).ok()?;
        let snapshot = self
            .snapshots
            .borrow()
            .get(&(current.device, current.inode))
            .copied()
            .unwrap_or(current);
        Some(DirectoryTimesSnapshot {
            source: snapshot,
            dereference,
        })
    }

    /// Capture direct child directories before the walker opens them.
    pub(crate) fn capture_children(&mut self, path: &Path) {
        let Ok(children) = fs::read_dir(path) else {
            return;
        };
        for child in children.flatten() {
            let child_path = child.path();
            if let Ok(snapshot) =
                SourceTimesSnapshot::from_path(&child_path, self.child_dereference)
                && snapshot.file_type.is_dir()
            {
                self.snapshots
                    .borrow_mut()
                    .entry((snapshot.device, snapshot.inode))
                    .or_insert(snapshot);
            }
        }
    }

    fn capture(&self, path: &Path, dereference: bool) {
        if let Ok(snapshot) = SourceTimesSnapshot::from_path(path, dereference) {
            self.snapshots
                .borrow_mut()
                .entry((snapshot.device, snapshot.inode))
                .or_insert(snapshot);
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SourceTimes {
    accessed: Timespec,
    modified: Timespec,
}

impl SourceTimes {
    pub(crate) fn from_metadata(metadata: &Metadata) -> io::Result<Self> {
        Ok(Self {
            accessed: to_timespec(metadata.accessed()?)?,
            modified: to_timespec(metadata.modified()?)?,
        })
    }

    fn from_stat(stat: &rustix::fs::Stat) -> Self {
        Self {
            accessed: Timespec {
                tv_sec: stat.st_atim.tv_sec,
                tv_nsec: stat.st_atim.tv_nsec,
            },
            modified: Timespec {
                tv_sec: stat.st_mtim.tv_sec,
                tv_nsec: stat.st_mtim.tv_nsec,
            },
        }
    }

    fn matches_stat(&self, stat: &rustix::fs::Stat, compare_accessed: bool) -> bool {
        let timestamps = Self::from_stat(stat);
        self.modified == timestamps.modified
            && (!compare_accessed || self.accessed == timestamps.accessed)
    }
}

pub(crate) fn create_symlink(source: &Path, dest: &Path) -> io::Result<()> {
    rustix::fs::symlink(source, dest).map_err(io::Error::from)
}

/// Copy a regular file while refusing source symlinks in no-dereference mode.
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
    nofollow: bool,
) -> CopyResult<CopyDebug> {
    if reflink_mode != ReflinkMode::Never {
        return Err(translate!("cp-error-reflink-not-supported")
            .to_string()
            .into());
    }
    if sparse_mode != SparseMode::Auto {
        return Err(translate!("cp-error-sparse-not-supported")
            .to_string()
            .into());
    }

    let mut source_file =
        open_source(source, nofollow).map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
    let mut dest_file = create_dest_restrictive(dest, false).map_err(|e| {
        CpError::IoErrContext(
            e,
            translate!("cp-error-cannot-create-regular-file", "path" => dest.quote()),
        )
    })?;
    buf_copy::copy_fast(&mut source_file, &mut dest_file)
        .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;

    Ok(CopyDebug {
        offload: OffloadReflinkDebug::Unsupported,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::Unsupported,
    })
}

pub(crate) fn is_optional_metadata_error(error: &io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(code) if code == libc::EOPNOTSUPP || code == libc::ENOSYS
    )
}

pub(crate) fn set_timestamps(
    source_times: SourceTimes,
    dest: &Path,
    follow_destination: bool,
) -> io::Result<()> {
    let timestamps = Timestamps {
        last_access: source_times.accessed,
        last_modification: source_times.modified,
    };
    let flags = if follow_destination {
        AtFlags::empty()
    } else {
        AtFlags::SYMLINK_NOFOLLOW
    };
    utimensat(CWD, dest, &timestamps, flags).map_err(io::Error::from)
}

fn to_timespec(time: SystemTime) -> io::Result<Timespec> {
    let duration = time
        .duration_since(UNIX_EPOCH)
        .map_err(|error| io::Error::new(io::ErrorKind::Unsupported, error))?;
    Ok(Timespec {
        tv_sec: duration.as_secs() as i64,
        tv_nsec: duration.subsec_nanos() as i32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn nofollow_copy_rejects_symlink_source() {
        let target = Path::new("cp-wasi-nofollow-target");
        let source = Path::new("cp-wasi-nofollow-source");
        let dest = Path::new("cp-wasi-nofollow-dest");
        for path in [source, target, dest] {
            fs::remove_file(path).ok();
        }
        File::create(target).unwrap();
        create_symlink(target, source).unwrap();

        let result = copy_on_write(
            source,
            dest,
            ReflinkMode::Never,
            SparseMode::Auto,
            "copy",
            true,
        );

        assert!(result.is_err());
        assert!(!dest.exists());
        fs::remove_file(source).unwrap();
        fs::remove_file(target).unwrap();
    }
}
