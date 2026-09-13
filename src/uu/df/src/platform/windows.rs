// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Windows backend of `df`: volume usage probes and the `-i` notice.

use std::ffi::OsString;
use std::path::Path;

use clap::ArgMatches;
use uucore::error::UResult;
use uucore::fsext::{FsUsage, MountInfo};
use uucore::translate;

use crate::OPT_INODES;
use crate::filesystem::{Filesystem, FsError};

/// Windows has no call to flush every filesystem for `--sync`.
pub(crate) fn sync() {}

/// Usage of the filesystem at `mount_info`, `None` if it cannot be queried.
pub(crate) fn fs_usage(mount_info: &MountInfo) -> Option<FsUsage> {
    let stat_path = if mount_info.mount_dir.is_empty() {
        // On windows, we expect the volume id
        mount_info.dev_id.as_ref()
    } else {
        mount_info.mount_dir.as_os_str()
    };
    FsUsage::new(Path::new(stat_path)).ok()
}

/// Find and create the filesystem from the given mount.
pub(crate) fn filesystem_from_mount(
    _mounts: &[MountInfo],
    mount: &MountInfo,
    file: Option<OsString>,
) -> Result<Filesystem, FsError> {
    Filesystem::new(mount.clone(), file).ok_or(FsError::MountMissing)
}

/// Find and create the filesystem that contains `path` through the mount table.
pub(crate) fn filesystem_for_path<P>(
    mounts: &[MountInfo],
    _use_fallback: bool,
    path: P,
) -> Result<Filesystem, FsError>
where
    P: AsRef<Path>,
{
    Filesystem::from_path(mounts, path)
}

/// `-i` is not supported: say so and stop successfully.
pub(crate) fn maybe_unsupported_options(matches: &ArgMatches) -> Option<UResult<()>> {
    if matches.get_flag(OPT_INODES) {
        println!(
            "{}",
            translate!("df-error-inodes-not-supported-windows", "program" => "df")
        );
        return Some(Ok(()));
    }
    None
}
