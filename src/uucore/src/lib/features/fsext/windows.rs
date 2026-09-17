// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore WNet FAILCRITICALERRORS SUBST

//! Windows backend of `fsext`: volume enumeration and usage probes.

use std::ffi::OsString;
use std::io;
use std::path::Path;

use super::{FsUsage, MountInfo};
use crate::error::UResult;

/// Every mount path of every volume, then the drive letters that are not
/// volume mount points (mapped network drives, SUBST drives).
pub(super) fn read_fs_list() -> UResult<Vec<MountInfo>> {
    let _quiet = sys::ErrorMode::fail_critical_errors();
    let mut mounts = Vec::new();
    for volume in sys::volumes()? {
        let paths = sys::volume_mount_paths(&volume).unwrap_or_default();

        let mut mount = MountInfo::from_mount_dir(OsString::from(&volume));

        if paths.is_empty() {
            mount.mount_dir.clear();
            mounts.push(mount);
        } else {
            for mount_dir in paths {
                mounts.push(MountInfo {
                    mount_dir,
                    ..mount.clone()
                });
            }
        }
    }

    // Add mapped network drives, SUBST drives, etc., to the list.
    for drive in sys::logical_drives() {
        if !mounts.iter().any(|m| m.mount_dir == drive) {
            mounts.push(MountInfo::from_mount_dir(drive));
        }
    }

    Ok(mounts)
}

impl MountInfo {
    /// The filesystem mounted at `mount_dir` (`C:\`, `C:\mount\`, `\\server\share\`).
    pub fn from_mount_dir(mount_dir: OsString) -> Self {
        let info = sys::volume_information(&mount_dir).unwrap_or_default();
        Self {
            dev_id: info.dev_id,
            dev_name: info.dev_name,
            fs_type: info.fs_type,
            mount_root: OsString::new(),
            mount_dir,
            mount_option: String::new(),
            remote: info.remote,
            dummy: false,
        }
    }
}

impl FsUsage {
    /// Usage of the filesystem at the NT path `root`; Windows reports no inode counts.
    pub fn new(root: &Path) -> io::Result<Self> {
        let _quiet = sys::ErrorMode::fail_critical_errors();
        let info = sys::disk_space(root)?;
        let blocksize = info.SectorsPerAllocationUnit as u64 * info.BytesPerSector as u64;
        let blocks = info.TotalAllocationUnits.max(0) as u64;
        let bfree = info.ActualAvailableAllocationUnits.max(0) as u64;
        let bavail = info.CallerAvailableAllocationUnits.max(0) as u64;
        Ok(Self {
            blocksize,
            blocks,
            bfree,
            bavail,
            bavail_top_bit_set: false,
            files: 0,
            ffree: 0,
        })
    }
}

/// Safe wrappers around the Win32 calls; every `unsafe` lives here.
mod sys {
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::os::windows::ffi::OsStringExt;
    use std::path::Path;
    use std::ptr;

    use crate::features::nt;
    use crate::wide::{FromWide, ToWide};
    use windows_sys::Win32::Foundation::{
        ERROR_MORE_DATA, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE, MAX_PATH,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetLogicalDrives,
        GetVolumePathNamesForVolumeNameW,
    };
    use windows_sys::Win32::System::Diagnostics::Debug::{
        SEM_FAILCRITICALERRORS, SetThreadErrorMode,
    };

    const BUF_LEN: usize = MAX_PATH as usize + 1;

    /// Keeps the "no disk in drive" dialog away while probing drives without
    /// media; the previous mode is restored on drop.
    pub struct ErrorMode(u32);

    impl ErrorMode {
        pub fn fail_critical_errors() -> Self {
            let mut previous = 0;
            // SAFETY: `previous` is a valid out-pointer.
            unsafe { SetThreadErrorMode(SEM_FAILCRITICALERRORS, &raw mut previous) };
            Self(previous)
        }
    }

    impl Drop for ErrorMode {
        fn drop(&mut self) {
            // SAFETY: a null out-pointer is allowed.
            unsafe { SetThreadErrorMode(self.0, ptr::null_mut()) };
        }
    }

    struct FindVolume(HANDLE);

    impl Drop for FindVolume {
        fn drop(&mut self) {
            // SAFETY: the handle came from `FindFirstVolumeW` and is closed once.
            unsafe { FindVolumeClose(self.0) };
        }
    }

    /// The `\\?\Volume{...}\` name of every volume.
    pub fn volumes() -> io::Result<Vec<String>> {
        let mut name = [0u16; BUF_LEN];
        // SAFETY: `name` is a valid buffer of `name.len()` units.
        let handle = unsafe { FindFirstVolumeW(name.as_mut_ptr(), name.len() as u32) };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let handle = FindVolume(handle);
        let mut volumes = vec![String::from_wide_null(&name)];
        loop {
            // SAFETY: `handle` is open; `name` is a valid buffer of `name.len()` units.
            if unsafe { FindNextVolumeW(handle.0, name.as_mut_ptr(), name.len() as u32) } == 0 {
                let err = io::Error::last_os_error();
                return if err.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                    Ok(volumes)
                } else {
                    Err(err)
                };
            }
            volumes.push(String::from_wide_null(&name));
        }
    }

    /// The drive letters and mounted folders `volume` is reachable at, each
    /// with a trailing `\`.
    pub fn volume_mount_paths(volume: &str) -> io::Result<Vec<OsString>> {
        let volume = volume.to_wide_null();
        let mut paths = vec![0u16; BUF_LEN];
        loop {
            let mut len = 0;
            // SAFETY: `volume` is NUL-terminated; `paths` is a valid buffer of
            // `paths.len()` units and `len` a valid out-pointer.
            let ok = unsafe {
                GetVolumePathNamesForVolumeNameW(
                    volume.as_ptr(),
                    paths.as_mut_ptr(),
                    paths.len() as u32,
                    &raw mut len,
                )
            };
            if ok != 0 {
                return Ok(paths[..len as usize]
                    .split(|&c| c == 0)
                    .filter(|p| !p.is_empty())
                    .map(OsString::from_wide)
                    .collect());
            }
            let err = io::Error::last_os_error();
            if err.raw_os_error() != Some(ERROR_MORE_DATA as i32) {
                return Err(err);
            }
            paths.resize(len as usize, 0);
        }
    }

    /// The root of every drive letter in use, `A:\` to `Z:\`.
    pub fn logical_drives() -> impl Iterator<Item = OsString> {
        // SAFETY: no preconditions.
        let mask = unsafe { GetLogicalDrives() };
        (b'A'..=b'Z')
            .filter(move |letter| mask & (1 << (letter - b'A')) != 0)
            .map(|letter| OsString::from(format!("{}:\\", letter as char)))
    }

    #[derive(Default)]
    pub struct VolumeInformation {
        pub dev_id: String,
        pub dev_name: String,
        pub fs_type: String,
        pub remote: bool,
    }

    pub fn volume_information(root: &OsStr) -> io::Result<VolumeInformation> {
        let handle = nt::open_file_win32(
            Path::new(root),
            nt::SYNCHRONIZE,
            nt::FILE_SYNCHRONOUS_IO_NONALERT | nt::FILE_DIRECTORY_FILE,
        )?;

        // This returns a path such as "\Device\HarddiskVolume3\".
        let dev_id = nt::query_nt_path(&handle)?.to_string_lossy().into_owned();

        // It's more pleasant to read without the trailing backslash.
        let dev_name = dev_id.trim_end_matches('\\').to_owned();

        let fs_type = nt::query_filesystem_name(&handle).unwrap_or_default();

        // SAFETY: The information class matches FILE_FS_DEVICE_INFORMATION.
        let remote = unsafe {
            nt::query_volume_information::<nt::FILE_FS_DEVICE_INFORMATION>(
                &handle,
                nt::FileFsDeviceInformation,
            )
        }
        .is_ok_and(|info| info.Characteristics & nt::FILE_REMOTE_DEVICE != 0);

        Ok(VolumeInformation {
            dev_id,
            dev_name,
            fs_type,
            remote,
        })
    }

    /// `FILE_FS_FULL_SIZE_INFORMATION` for the filesystem at the NT path `root`.
    pub fn disk_space(root: &Path) -> io::Result<nt::FILE_FS_FULL_SIZE_INFORMATION> {
        let handle = nt::open_file_nt(
            root,
            nt::SYNCHRONIZE,
            nt::FILE_SYNCHRONOUS_IO_NONALERT
                | nt::FILE_DIRECTORY_FILE
                | nt::FILE_OPEN_FOR_FREE_SPACE_QUERY,
        )?;
        // SAFETY: The information class matches FILE_FS_FULL_SIZE_INFORMATION.
        unsafe { nt::query_volume_information(&handle, nt::FileFsFullSizeInformation) }
    }
}

#[cfg(test)]
mod tests {
    use super::read_fs_list;
    use std::ffi::OsString;

    #[test]
    fn test_read_fs_list_has_system_drive() {
        let system_drive = OsString::from(std::env::var("SystemDrive").unwrap() + "\\");
        let mounts = read_fs_list().unwrap();
        assert!(
            mounts
                .iter()
                .all(|m| m.mount_dir.is_empty() || m.mount_dir.to_string_lossy().ends_with('\\'))
        );
        let system = mounts.iter().find(|m| m.mount_dir == system_drive).unwrap();
        assert_ne!(system.fs_type, "");
        assert!(system.dev_name.starts_with("\\Device\\"));
    }
}
