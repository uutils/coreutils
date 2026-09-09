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
        for mount_dir in sys::volume_mount_paths(&volume).unwrap_or_default() {
            mounts.push(MountInfo::from_mount_dir(mount_dir));
        }
    }
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
        let remote = sys::is_remote_drive(&mount_dir);
        let dev_name = remote
            .then(|| sys::remote_name(&mount_dir))
            .flatten()
            .unwrap_or_else(|| mount_dir.to_string_lossy().into_owned());
        let (dev_id, fs_type) = match sys::volume_information(&mount_dir) {
            Ok(info) => (info.serial.to_string(), info.fs_type),
            Err(_) => (dev_name.clone(), String::new()),
        };
        Self {
            dev_id,
            dev_name,
            fs_type,
            mount_root: OsString::new(),
            mount_dir,
            mount_option: String::new(),
            remote,
            dummy: false,
        }
    }
}

impl FsUsage {
    /// Usage of the volume mounted at `root`; Windows reports no inode counts.
    pub fn new(root: &Path) -> io::Result<Self> {
        let _quiet = sys::ErrorMode::fail_critical_errors();
        let root = root.as_os_str();
        let space = sys::disk_space(root)?;
        let blocksize = sys::cluster_size(root).unwrap_or(1).max(1);
        Ok(Self {
            blocksize,
            blocks: space.total / blocksize,
            bfree: space.free / blocksize,
            bavail: space.available / blocksize,
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
    use std::ptr;

    use crate::wide::{FromWide, ToWide};
    use windows_sys::Win32::Foundation::{
        ERROR_MORE_DATA, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE, MAX_PATH, NO_ERROR,
    };
    use windows_sys::Win32::NetworkManagement::WNet::WNetGetConnectionW;
    use windows_sys::Win32::Storage::FileSystem::{
        FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetDiskFreeSpaceExW, GetDiskFreeSpaceW,
        GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW, GetVolumePathNamesForVolumeNameW,
    };
    use windows_sys::Win32::System::Diagnostics::Debug::{
        SEM_FAILCRITICALERRORS, SetThreadErrorMode,
    };
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_REMOTE;
    use windows_sys::core::BOOL;

    const BUF_LEN: usize = MAX_PATH as usize + 1;

    fn cvt(result: BOOL) -> io::Result<()> {
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

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

    pub struct VolumeInformation {
        pub fs_type: String,
        pub serial: u32,
    }

    pub fn volume_information(root: &OsStr) -> io::Result<VolumeInformation> {
        let root = root.to_wide_null();
        let mut serial = 0;
        let mut fs_type = [0u16; BUF_LEN];
        // SAFETY: `root` is NUL-terminated; `serial` and `fs_type` are valid
        // out-buffers and the remaining out-pointers may be null.
        cvt(unsafe {
            GetVolumeInformationW(
                root.as_ptr(),
                ptr::null_mut(),
                0,
                &raw mut serial,
                ptr::null_mut(),
                ptr::null_mut(),
                fs_type.as_mut_ptr(),
                fs_type.len() as u32,
            )
        })?;
        Ok(VolumeInformation {
            fs_type: String::from_wide_null(&fs_type),
            serial,
        })
    }

    pub fn is_remote_drive(root: &OsStr) -> bool {
        let root = root.to_wide_null();
        // SAFETY: `root` is NUL-terminated.
        unsafe { GetDriveTypeW(root.as_ptr()) == DRIVE_REMOTE }
    }

    /// The UNC name a drive letter is mapped to, `\\server\share`.
    pub fn remote_name(root: &OsStr) -> Option<String> {
        // `X:` only; the API rejects the trailing separator.
        let local: Vec<u16> = root.to_wide().into_iter().take(2).chain([0]).collect();
        let mut remote = [0u16; BUF_LEN];
        let mut len = remote.len() as u32;
        // SAFETY: `local` is NUL-terminated; `remote` is a valid buffer of `len` units.
        let status =
            unsafe { WNetGetConnectionW(local.as_ptr(), remote.as_mut_ptr(), &raw mut len) };
        (status == NO_ERROR).then(|| String::from_wide_null(&remote))
    }

    pub struct DiskSpace {
        pub total: u64,
        pub free: u64,
        pub available: u64,
    }

    /// Byte counts of the volume at `root`; `available` honours quotas.
    pub fn disk_space(root: &OsStr) -> io::Result<DiskSpace> {
        let root = root.to_wide_null();
        let mut space = DiskSpace {
            total: 0,
            free: 0,
            available: 0,
        };
        // SAFETY: `root` is NUL-terminated; the three out-pointers are valid.
        cvt(unsafe {
            GetDiskFreeSpaceExW(
                root.as_ptr(),
                &raw mut space.available,
                &raw mut space.total,
                &raw mut space.free,
            )
        })?;
        Ok(space)
    }

    /// Bytes per allocation unit of the volume at `root`.
    pub fn cluster_size(root: &OsStr) -> io::Result<u64> {
        let root = root.to_wide_null();
        let mut sectors_per_cluster = 0;
        let mut bytes_per_sector = 0;
        // SAFETY: `root` is NUL-terminated; the two out-pointers are valid and
        // the remaining ones may be null.
        cvt(unsafe {
            GetDiskFreeSpaceW(
                root.as_ptr(),
                &raw mut sectors_per_cluster,
                &raw mut bytes_per_sector,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        })?;
        Ok(u64::from(sectors_per_cluster) * u64::from(bytes_per_sector))
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
                .all(|m| m.mount_dir.to_string_lossy().ends_with('\\'))
        );
        let system = mounts.iter().find(|m| m.mount_dir == system_drive).unwrap();
        assert!(!system.fs_type.is_empty());
        assert_eq!(system.dev_name, system_drive.to_string_lossy());
    }
}
