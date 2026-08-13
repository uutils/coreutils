// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fs::Metadata;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};

pub(crate) fn create_symlink(source: &Path, dest: &Path) -> io::Result<()> {
    rustix::fs::symlink(source, dest).map_err(io::Error::from)
}

pub(crate) fn set_timestamps(source_metadata: &Metadata, dest: &Path) -> io::Result<()> {
    let timestamps = Timestamps {
        last_access: to_timespec(source_metadata.accessed()?)?,
        last_modification: to_timespec(source_metadata.modified()?)?,
    };
    utimensat(CWD, dest, &timestamps, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)
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
