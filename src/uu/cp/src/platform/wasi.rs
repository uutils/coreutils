// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fs::Metadata;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rustix::fs::{AtFlags, CWD, FileType, Timespec, Timestamps, lstat, stat, utimensat};

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
        let stat = if dereference {
            stat(path).ok()?
        } else {
            lstat(path).ok()?
        };
        let file_type = FileType::from_raw_mode(stat.st_mode);
        if stat.st_dev != self.device
            || stat.st_ino != self.inode
            || file_type != self.file_type
            || !self.times.matches_stat(&stat, !self.file_type.is_symlink())
        {
            return None;
        }
        Some(self.times)
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
