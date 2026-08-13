// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fs::Metadata;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rustix::fs::{AtFlags, CWD, FileType, Timespec, Timestamps, lstat, stat, utimensat};

pub(crate) struct SourceTimestampSnapshot {
    timestamps: SourceTimestamps,
    device: u64,
    inode: u64,
    file_type: FileType,
}

impl SourceTimestampSnapshot {
    pub(crate) fn from_path(path: &Path) -> io::Result<Self> {
        let stat = lstat(path)?;
        Ok(Self {
            timestamps: SourceTimestamps::from_stat(&stat),
            device: stat.st_dev,
            inode: stat.st_ino,
            file_type: FileType::from_raw_mode(stat.st_mode),
        })
    }

    pub(crate) fn current_timestamps(
        &self,
        path: &Path,
        dereference: bool,
    ) -> Option<SourceTimestamps> {
        let stat = if dereference {
            stat(path).ok()?
        } else {
            lstat(path).ok()?
        };
        let file_type = FileType::from_raw_mode(stat.st_mode);
        if stat.st_dev != self.device
            || stat.st_ino != self.inode
            || file_type != self.file_type
            || !self
                .timestamps
                .matches_stat(&stat, !self.file_type.is_symlink())
        {
            return None;
        }
        Some(self.timestamps)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SourceTimestamps {
    accessed: SystemTime,
    modified: SystemTime,
}

#[derive(Clone, Copy)]
pub(crate) struct TimestampOptions {
    pub(crate) source: Option<SourceTimestamps>,
    pub(crate) no_follow: bool,
}

impl SourceTimestamps {
    pub(crate) fn from_metadata(metadata: &Metadata) -> io::Result<Self> {
        Ok(Self {
            accessed: metadata.accessed()?,
            modified: metadata.modified()?,
        })
    }

    fn from_stat(stat: &rustix::fs::Stat) -> Self {
        Self {
            accessed: UNIX_EPOCH
                + std::time::Duration::new(stat.st_atim.tv_sec as u64, stat.st_atim.tv_nsec as u32),
            modified: UNIX_EPOCH
                + std::time::Duration::new(stat.st_mtim.tv_sec as u64, stat.st_mtim.tv_nsec as u32),
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

pub(crate) fn set_timestamps(
    source_timestamps: SourceTimestamps,
    dest: &Path,
    no_follow: bool,
) -> io::Result<()> {
    let timestamps = Timestamps {
        last_access: to_timespec(source_timestamps.accessed)?,
        last_modification: to_timespec(source_timestamps.modified)?,
    };
    let flags = if no_follow {
        AtFlags::SYMLINK_NOFOLLOW
    } else {
        AtFlags::empty()
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
