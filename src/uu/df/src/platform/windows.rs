// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore SUBST

//! Windows backend of `df`: volume usage probes, path resolution and the `-i` notice.

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
    FsUsage::new(Path::new(&mount_info.mount_dir)).ok()
}

/// Find and create the filesystem from the given mount.
pub(crate) fn filesystem_from_mount(
    _mounts: &[MountInfo],
    mount: &MountInfo,
    file: Option<OsString>,
) -> Result<Filesystem, FsError> {
    Filesystem::new(mount.clone(), file).ok_or(FsError::MountMissing)
}

/// Find and create the filesystem that contains `path`: the mount with the
/// longest directory prefixing it, or, when the mount table does not list it
/// (UNC paths, unreadable table), the volume root the OS reports for it.
pub(crate) fn filesystem_for_path<P>(
    mounts: &[MountInfo],
    _use_fallback: bool,
    path: P,
) -> Result<Filesystem, FsError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let file = path.as_os_str().to_owned();
    // Not `canonicalize`: it resolves SUBST drives and junctions away and
    // yields `\\?\` prefixes that never match a mount directory.
    let absolute = std::path::absolute(path).map_err(|_| FsError::InvalidPath)?;
    absolute.metadata().map_err(|_| FsError::InvalidPath)?;
    let longest = mounts
        .iter()
        .filter(|m| absolute.starts_with(&m.mount_dir))
        .max_by_key(|m| m.mount_dir.len());
    let mount_info = if let Some(mount_info) = longest {
        mount_info.clone()
    } else {
        let root = uucore::fs::volume_path_name(&absolute).map_err(|_| FsError::MountMissing)?;
        MountInfo::from_mount_dir(root.into_os_string())
    };
    Filesystem::new(mount_info, Some(file)).ok_or(FsError::MountMissing)
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
