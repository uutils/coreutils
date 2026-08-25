// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Provides a summary representation of a filesystem.
//!
//! A [`Filesystem`] struct represents a device containing a
//! filesystem mounted at a particular directory. It also includes
//! information on amount of space available and amount of space used.
// spell-checker:ignore canonicalized
use std::{ffi::OsString, path::Path};

use uucore::fsext::{FsUsage, MountInfo};

use crate::platform;

/// Summary representation of a filesystem.
///
/// A [`Filesystem`] struct represents a device containing a
/// filesystem mounted at a particular directory. The
/// [`Filesystem::mount_info`] field exposes that information. The
/// [`Filesystem::usage`] field provides information on the amount of
/// space available on the filesystem and the amount of space used.
#[derive(Debug, Clone)]
pub struct Filesystem {
    /// The file given on the command line if any.
    ///
    /// When invoking `df` with a positional argument, it displays
    /// usage information for the filesystem that contains the given
    /// file. If given, this field contains that filename.
    pub file: Option<OsString>,

    /// Information about the mounted device, mount directory, and related options.
    pub mount_info: MountInfo,

    /// Information about the amount of space used on the filesystem.
    pub usage: FsUsage,
}

#[derive(Debug, PartialEq)]
pub(crate) enum FsError {
    #[cfg(not(windows))]
    OverMounted,
    InvalidPath,
    MountMissing,
}

/// Find the mount info that best matches a given filesystem path.
///
/// This function returns the element of `mounts` on which `path` is
/// mounted. If there are no matches, this function returns
/// [`None`]. If there are two or more matches, then the single
/// [`MountInfo`] with the device name corresponding to the entered path.
///
/// If `canonicalize` is `true`, then the `path` is canonicalized
/// before checking whether it matches any mount directories.
///
/// # See also
///
/// * [`Path::canonicalize`]
/// * [`MountInfo::mount_dir`]
fn mount_info_from_path<P>(
    mounts: &[MountInfo],
    path: P,
    // This is really only used for testing purposes.
    canonicalize: bool,
) -> Result<&MountInfo, FsError>
where
    P: AsRef<Path>,
{
    // TODO Refactor this function with `Stater::find_mount_point()`
    // in the `stat` crate.
    let path = if canonicalize {
        path.as_ref()
            .canonicalize()
            .map_err(|_| FsError::InvalidPath)?
    } else {
        path.as_ref().to_path_buf()
    };

    // Find the potential mount point that matches entered path
    let maybe_mount_point = mounts
        .iter()
        // Create pair MountInfo, canonicalized device name
        // TODO Abstract from accessing real filesystem to
        // make code more testable
        .map(|m| (m, std::fs::canonicalize(&m.dev_name)))
        // Ignore non existing paths
        .filter_map(|m| m.1.ok().map(|m1| (m.0, m1)))
        // Try to find canonicalized device name corresponding to entered path
        .find(|m| m.1.eq(&path))
        .map(|m| m.0);

    maybe_mount_point
        .or_else(|| {
            mounts
                .iter()
                .filter(|mi| path.starts_with(&mi.mount_dir))
                .max_by_key(|mi| mi.mount_dir.len())
        })
        .ok_or(FsError::MountMissing)
}

impl Filesystem {
    // TODO: resolve uuid in `mount_info.dev_name` if exists
    pub(crate) fn new(mount_info: MountInfo, file: Option<OsString>) -> Option<Self> {
        let usage = platform::fs_usage(&mount_info)?;
        Some(Self {
            file,
            mount_info,
            usage,
        })
    }

    /// Find and create the filesystem that best matches a given path.
    ///
    /// This function returns a new `Filesystem` derived from the
    /// element of `mounts` on which `path` is mounted. If there are
    /// no matches, this function returns [`None`]. If there are two
    /// or more matches, then the single [`Filesystem`] with the
    /// longest mount directory is returned.
    ///
    /// The `path` is canonicalized before checking whether it matches
    /// any mount directories.
    ///
    /// # See also
    ///
    /// * [`Path::canonicalize`]
    /// * [`MountInfo::mount_dir`]
    ///
    pub(crate) fn from_path<P>(mounts: &[MountInfo], path: P) -> Result<Self, FsError>
    where
        P: AsRef<Path>,
    {
        let file = path.as_ref().as_os_str().to_owned();
        let canonicalize = true;

        let result = mount_info_from_path(mounts, path, canonicalize);
        result
            .and_then(|mount_info| platform::filesystem_from_mount(mounts, mount_info, Some(file)))
    }
}

#[cfg(test)]
mod tests {

    mod mount_info_from_path {

        use std::ffi::OsString;

        use uucore::fsext::MountInfo;

        use crate::filesystem::{FsError, mount_info_from_path};

        /// Create a fake `MountInfo` with the given directory name.
        fn mount_info(mount_dir: &str) -> MountInfo {
            MountInfo {
                dev_id: String::default(),
                dev_name: String::default(),
                fs_type: String::default(),
                mount_dir: OsString::from(mount_dir),
                mount_option: String::default(),
                mount_root: OsString::default(),
                remote: Default::default(),
                dummy: Default::default(),
            }
        }

        /// Check whether two `MountInfo` instances are equal.
        fn mount_info_eq(m1: &MountInfo, m2: &MountInfo) -> bool {
            m1.dev_id == m2.dev_id
                && m1.dev_name == m2.dev_name
                && m1.fs_type == m2.fs_type
                && m1.mount_dir == m2.mount_dir
                && m1.mount_option == m2.mount_option
                && m1.mount_root == m2.mount_root
                && m1.remote == m2.remote
                && m1.dummy == m2.dummy
        }

        #[test]
        fn test_empty_mounts() {
            assert_eq!(
                mount_info_from_path(&[], "/", false).unwrap_err(),
                FsError::MountMissing
            );
        }

        #[test]
        fn test_bad_path() {
            assert_eq!(
                // This path better not exist....
                mount_info_from_path(&[], "/non-existent-path", true).unwrap_err(),
                FsError::InvalidPath
            );
        }

        #[test]
        fn test_exact_match() {
            let mounts = [mount_info("/foo")];
            let actual = mount_info_from_path(&mounts, "/foo", false).unwrap();
            assert!(mount_info_eq(actual, &mounts[0]));
        }

        #[test]
        fn test_prefix_match() {
            let mounts = [mount_info("/foo")];
            let actual = mount_info_from_path(&mounts, "/foo/bar", false).unwrap();
            assert!(mount_info_eq(actual, &mounts[0]));
        }

        #[test]
        fn test_multiple_matches() {
            let mounts = [mount_info("/foo"), mount_info("/foo/bar")];
            let actual = mount_info_from_path(&mounts, "/foo/bar", false).unwrap();
            assert!(mount_info_eq(actual, &mounts[1]));
        }

        #[test]
        fn test_no_match() {
            let mounts = [mount_info("/foo")];
            assert_eq!(
                mount_info_from_path(&mounts, "/bar", false).unwrap_err(),
                FsError::MountMissing
            );
        }

        #[test]
        fn test_partial_match() {
            let mounts = [mount_info("/foo/bar")];
            assert_eq!(
                mount_info_from_path(&mounts, "/foo/baz", false).unwrap_err(),
                FsError::MountMissing
            );
        }

        #[test]
        fn test_dev_name_match() {
            let tmp = tempfile::TempDir::new().expect("Failed to create temp dir");
            let dev_name = std::fs::canonicalize(tmp.path())
                .expect("Failed to canonicalize tmp path")
                .to_string_lossy()
                .to_string();

            let mut mount_info = mount_info("/foo");
            mount_info.dev_name = dev_name.clone();
            let mounts = [mount_info];
            let actual = mount_info_from_path(&mounts, dev_name, false).unwrap();
            assert!(mount_info_eq(actual, &mounts[0]));
        }
    }
}
