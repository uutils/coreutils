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

#[cfg(unix)]
use uucore::fsext::statfs;
use uucore::fsext::{FsUsage, MountInfo};

/// Summary representation of a filesystem.
///
/// A [`Filesystem`] struct represents a device containing a
/// filesystem mounted at a particular directory. The
/// [`Filesystem::mount_info`] field exposes that information. The
/// [`Filesystem::usage`] field provides information on the amount of
/// space available on the filesystem and the amount of space used.
#[derive(Debug, Clone)]
pub(crate) struct Filesystem {
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

/// Check whether `mount` has been over-mounted.
///
/// `mount` is considered over-mounted if it there is an element in
/// `mounts` after mount that has the same `mount_dir`.
#[cfg(not(windows))]
fn is_over_mounted(mounts: &[MountInfo], mount: &MountInfo) -> bool {
    let last_mount_for_dir = mounts.iter().rfind(|m| m.mount_dir == mount.mount_dir);

    if let Some(lmi) = last_mount_for_dir {
        lmi.dev_name != mount.dev_name
    } else {
        // Should be unreachable if `mount` is in `mounts`
        false
    }
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
        .filter(|m| m.1.is_ok())
        .map(|m| (m.0, m.1.ok().unwrap()))
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
        let _stat_path = if mount_info.mount_dir.is_empty() {
            #[cfg(unix)]
            {
                mount_info.dev_name.clone().into()
            }
            #[cfg(windows)]
            {
                // On windows, we expect the volume id
                mount_info.dev_id.clone().into()
            }
        } else {
            mount_info.mount_dir.clone()
        };
        #[cfg(unix)]
        let usage = FsUsage::new(statfs(&_stat_path).ok()?);
        #[cfg(windows)]
        let usage = FsUsage::new(Path::new(&_stat_path)).ok()?;
        Some(Self {
            file,
            mount_info,
            usage,
        })
    }

    /// Find and create the filesystem from the given mount
    /// after checking that the it hasn't been over-mounted
    #[cfg(not(windows))]
    pub(crate) fn from_mount(
        mounts: &[MountInfo],
        mount: &MountInfo,
        file: Option<OsString>,
    ) -> Result<Self, FsError> {
        if is_over_mounted(mounts, mount) {
            Err(FsError::OverMounted)
        } else {
            Self::new(mount.clone(), file).ok_or(FsError::MountMissing)
        }
    }

    /// Find and create the filesystem from the given mount.
    #[cfg(windows)]
    pub(crate) fn from_mount(mount: &MountInfo, file: Option<OsString>) -> Result<Self, FsError> {
        Self::new(mount.clone(), file).ok_or(FsError::MountMissing)
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
        #[cfg(windows)]
        return result.and_then(|mount_info| Self::from_mount(mount_info, Some(file)));
        #[cfg(not(windows))]
        return result.and_then(|mount_info| Self::from_mount(mounts, mount_info, Some(file)));
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
        // clippy::assigning_clones added with Rust 1.78
        // Rust version = 1.76 on OpenBSD stable/7.5
        #[cfg_attr(not(target_os = "openbsd"), allow(clippy::assigning_clones))]
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

    #[cfg(not(windows))]
    mod over_mount {
        use std::ffi::OsString;

        use crate::filesystem::{Filesystem, FsError, is_over_mounted};
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
                Filesystem::from_mount(&mounts, &mounts[0], None).unwrap_err(),
                FsError::OverMounted
            );
        }
    }
}
