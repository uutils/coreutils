//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
//! Provides a summary representation of a filesystem.
//!
//! A [`Filesystem`] struct represents a device containing a
//! filesystem mounted at a particular directory. It also includes
//! information on amount of space available and amount of space used.
// spell-checker:ignore canonicalized
use std::path::Path;

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
    pub file: Option<String>,

    /// Information about the mounted device, mount directory, and related options.
    pub mount_info: MountInfo,

    /// Information about the amount of space used on the filesystem.
    pub usage: FsUsage,
}

#[derive(Debug, PartialEq)]
pub(crate) enum FsError {
    OverMounted,
    InvalidPath,
    MountMissing,
}

/// Check whether `mount` has been over-mounted.
///
/// `mount` is considered over-mounted if it there is an element in
/// `mounts` after mount that has the same mount_dir.
fn is_over_mounted(mounts: &[MountInfo], mount: &MountInfo) -> bool {
    let last_mount_for_dir = mounts
        .iter()
        .filter(|m| m.mount_dir == mount.mount_dir)
        .last();

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
/// [`MountInfo`] with the longest mount directory is returned.
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

    let maybe_mount_point = mounts
        .iter()
        .find(|mi| mi.dev_name.eq(&path.to_string_lossy()));

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
    pub(crate) fn new(mount_info: MountInfo, file: Option<String>) -> Option<Self> {
        let _stat_path = if !mount_info.mount_dir.is_empty() {
            mount_info.mount_dir.clone()
        } else {
            #[cfg(unix)]
            {
                mount_info.dev_name.clone()
            }
            #[cfg(windows)]
            {
                // On windows, we expect the volume id
                mount_info.dev_id.clone()
            }
        };
        #[cfg(unix)]
        let usage = FsUsage::new(statfs(_stat_path).ok()?);
        #[cfg(windows)]
        let usage = FsUsage::new(Path::new(&_stat_path));
        Some(Self {
            mount_info,
            usage,
            file,
        })
    }

    /// Find and create the filesystem from the given mount
    /// after checking that the it hasn't been over-mounted
    pub(crate) fn from_mount(
        mounts: &[MountInfo],
        mount: &MountInfo,
        file: Option<String>,
    ) -> Result<Self, FsError> {
        if is_over_mounted(mounts, mount) {
            Err(FsError::OverMounted)
        } else {
            Self::new(mount.clone(), file).ok_or(FsError::MountMissing)
        }
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
        let file = path.as_ref().display().to_string();
        let canonicalize = true;

        mount_info_from_path(mounts, path, canonicalize)
            .and_then(|mount_info| Self::from_mount(mounts, mount_info, Some(file)))
    }
}

#[cfg(test)]
mod tests {
    use uucore::fsext::MountInfo;

    fn mount_info_with_dev_name(mount_dir: &str, dev_name: Option<&str>) -> MountInfo {
        MountInfo {
            dev_id: Default::default(),
            dev_name: dev_name.map(|s| String::from(s)).unwrap_or_default(),
            fs_type: Default::default(),
            mount_dir: String::from(mount_dir),
            mount_option: Default::default(),
            mount_root: Default::default(),
            remote: Default::default(),
            dummy: Default::default(),
        }
    }
    // Create a fake `MountInfo` with the given directory name.
    fn mount_info(mount_dir: &str) -> MountInfo {
        mount_info_with_dev_name(mount_dir, None)
    }

    mod mount_info_from_path {

        use uucore::fsext::MountInfo;

        use crate::filesystem::{mount_info_from_path, tests::mount_info, FsError};

        // Check whether two `MountInfo` instances are equal.
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
                mount_info_from_path(&[], "/ksjhdkljflksdjfklsdjfksdjlkfjsd", true).unwrap_err(),
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
            let mut mount_info = mount_info("/foo");
            mount_info.dev_name = "/dev/sda2".to_string();
            let mounts = [mount_info];
            let actual = mount_info_from_path(&mounts, "/dev/sda2", false).unwrap();
            assert!(mount_info_eq(actual, &mounts[0]));
        }
    }

    mod over_mount {

        use crate::filesystem::{
            is_over_mounted, tests::mount_info_with_dev_name, Filesystem, FsError,
        };

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
