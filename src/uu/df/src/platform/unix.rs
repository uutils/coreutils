// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Unix backend of `df`: `statfs` usage probes, over-mount detection and the
//! fallback used when the mount table is unavailable.

use std::ffi::OsString;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use clap::ArgMatches;
use uucore::error::UResult;
use uucore::fsext::{FsMeta, FsUsage, MountInfo, pretty_fstype, statfs};

use crate::filesystem::{Filesystem, FsError};

/// Flush filesystem buffers for `--sync`.
pub(crate) fn sync() {
    #[cfg(not(target_os = "redox"))]
    rustix::fs::sync();
}

/// Usage of the filesystem at `mount_info`, `None` if it cannot be queried.
pub(crate) fn fs_usage(mount_info: &MountInfo) -> Option<FsUsage> {
    let stat_path = if mount_info.mount_dir.is_empty() {
        mount_info.dev_name.as_ref()
    } else {
        mount_info.mount_dir.as_os_str()
    };
    Some(FsUsage::new(statfs(stat_path).ok()?))
}

/// Check whether `mount` has been over-mounted.
///
/// `mount` is considered over-mounted if it there is an element in
/// `mounts` after mount that has the same `mount_dir`.
fn is_over_mounted(mounts: &[MountInfo], mount: &MountInfo) -> bool {
    let last_mount_for_dir = mounts.iter().rfind(|m| m.mount_dir == mount.mount_dir);

    if let Some(lmi) = last_mount_for_dir {
        lmi.dev_name != mount.dev_name
    } else {
        // Should be unreachable if `mount` is in `mounts`
        false
    }
}

/// Find and create the filesystem from the given mount
/// after checking that the it hasn't been over-mounted
pub(crate) fn filesystem_from_mount(
    mounts: &[MountInfo],
    mount: &MountInfo,
    file: Option<OsString>,
) -> Result<Filesystem, FsError> {
    if is_over_mounted(mounts, mount) {
        Err(FsError::OverMounted)
    } else {
        Filesystem::new(mount.clone(), file).ok_or(FsError::MountMissing)
    }
}

/// Find and create the filesystem that contains `path`, through the mount
/// table or, when that could not be read, through `statfs` alone.
pub(crate) fn filesystem_for_path<P>(
    mounts: &[MountInfo],
    use_fallback: bool,
    path: P,
) -> Result<Filesystem, FsError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if use_fallback {
        from_path_direct(path)
    } else {
        Filesystem::from_path(mounts, path).or_else(|_| from_path_direct_with_mounts(mounts, path))
    }
}

/// Find mount point by walking up the directory tree until device ID changes.
fn find_mount_point<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let mut current = path.as_ref().canonicalize()?;
    let current_dev = current.metadata()?.dev();

    while let Some(parent) = current.parent().filter(|p| !p.as_os_str().is_empty()) {
        let parent_dev = parent.metadata()?.dev();
        if parent_dev != current_dev || parent == current {
            return Ok(current);
        }

        current = parent.to_path_buf();
    }
    Ok(current)
}

/// Fallback using statfs with a mount table available.
fn from_path_direct_with_mounts<P>(mounts: &[MountInfo], path: P) -> Result<Filesystem, FsError>
where
    P: AsRef<Path>,
{
    let file = path.as_ref().as_os_str().to_owned();

    let canonical_path = path
        .as_ref()
        .canonicalize()
        .map_err(|_| FsError::InvalidPath)?;

    let stat_result = statfs(canonical_path.as_os_str()).map_err(|_| FsError::MountMissing)?;

    // GNU coreutils always appear to return the last match
    let mut last_match = None;
    for mount in mounts {
        if let Ok(stat_result_mount) = statfs(&mount.mount_dir)
            && stat_result_mount.fsid() == stat_result.fsid()
        {
            last_match = Some(mount);
        }
    }

    last_match
        .ok_or(FsError::MountMissing)
        .and_then(|mount_info| filesystem_from_mount(mounts, mount_info, Some(file)))
}

/// Fallback using statfs when mount table is unavailable.
fn from_path_direct<P>(path: P) -> Result<Filesystem, FsError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let file = path.as_os_str().to_owned();

    let canonical_path = path.canonicalize().map_err(|_| FsError::InvalidPath)?;

    let stat_result = statfs(canonical_path.as_os_str()).map_err(|_| FsError::MountMissing)?;
    let mount_dir = find_mount_point(&canonical_path).map_err(|_| FsError::MountMissing)?;
    let fs_type = pretty_fstype(stat_result.fs_type()).into_owned();

    let mount_info = MountInfo {
        dev_id: String::new(),
        dev_name: "-".to_string(),
        fs_type,
        mount_dir: mount_dir.into_os_string(),
        mount_option: String::new(),
        mount_root: OsString::new(),
        remote: false,
        dummy: false,
    };

    let usage = FsUsage::new(stat_result);

    Ok(Filesystem {
        file: Some(file),
        mount_info,
        usage,
    })
}

/// Every option is supported on unix.
pub(crate) fn maybe_unsupported_options(_matches: &ArgMatches) -> Option<UResult<()>> {
    None
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{filesystem_from_mount, is_over_mounted};
    use crate::filesystem::FsError;
    use uucore::fsext::MountInfo;

    fn mount_info_with_dev_name(mount_dir: &str, dev_name: Option<&str>) -> MountInfo {
        MountInfo {
            dev_id: String::default(),
            dev_name: dev_name.map(String::from).unwrap_or_default(),
            fs_type: String::default(),
            mount_dir: OsString::from(mount_dir),
            mount_option: String::default(),
            mount_root: OsString::default(),
            remote: Default::default(),
            dummy: Default::default(),
        }
    }

    #[test]
    fn test_over_mount() {
        let mount_info1 = mount_info_with_dev_name("/foo", Some("dev_name_1"));
        let mount_info2 = mount_info_with_dev_name("/foo", Some("dev_name_2"));
        let mounts = [mount_info1, mount_info2];
        assert!(is_over_mounted(&mounts, &mounts[0]));
    }

    #[test]
    fn test_over_mount_not_over_mounted() {
        let mount_info1 = mount_info_with_dev_name("/foo", Some("dev_name_1"));
        let mount_info2 = mount_info_with_dev_name("/foo", Some("dev_name_2"));
        let mounts = [mount_info1, mount_info2];
        assert!(!is_over_mounted(&mounts, &mounts[1]));
    }

    #[test]
    fn test_from_mount_over_mounted() {
        let mount_info1 = mount_info_with_dev_name("/foo", Some("dev_name_1"));
        let mount_info2 = mount_info_with_dev_name("/foo", Some("dev_name_2"));

        let mounts = [mount_info1, mount_info2];

        assert_eq!(
            filesystem_from_mount(&mounts, &mounts[0], None).unwrap_err(),
            FsError::OverMounted
        );
    }
}
