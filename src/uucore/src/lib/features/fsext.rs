// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Set of functions to manage file systems

// spell-checker:ignore DATETIME getmntinfo subsecond (fs) cifs smbfs

#[cfg(any(target_os = "linux", target_os = "android"))]
const LINUX_MTAB: &str = "/etc/mtab";
#[cfg(any(target_os = "linux", target_os = "android"))]
const LINUX_MOUNTINFO: &str = "/proc/self/mountinfo";
#[cfg(all(unix, not(any(target_os = "aix", target_os = "redox"))))]
static MOUNT_OPT_BIND: &str = "bind";
#[cfg(windows)]
const MAX_PATH: usize = 266;
#[cfg(windows)]
static EXIT_ERR: i32 = 1;

#[cfg(any(
    target_os = "freebsd",
    target_vendor = "apple",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use crate::os_str_from_bytes;
#[cfg(windows)]
use crate::show_warning;

use std::ffi::OsStr;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{
        FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetDiskFreeSpaceW, GetDriveTypeW,
        GetVolumeInformationW, GetVolumePathNamesForVolumeNameW, QueryDosDeviceW,
    },
    System::WindowsProgramming::DRIVE_REMOTE,
};

#[cfg(windows)]
#[allow(non_snake_case)]
fn LPWSTR2String(buf: &[u16]) -> String {
    let len = buf.iter().position(|&n| n == 0).unwrap();
    String::from_utf16(&buf[..len]).unwrap()
}

#[cfg(windows)]
fn to_nul_terminated_wide_string(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<u16>>()
}

#[cfg(unix)]
use libc::{
    S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK, mode_t, strerror,
};
#[cfg(unix)]
use std::ffi::{CStr, CString};
use std::io::Error as IOError;
#[cfg(unix)]
use std::mem;
#[cfg(windows)]
use std::path::Path;
use std::time::SystemTime;
#[cfg(not(windows))]
use std::time::UNIX_EPOCH;
use std::{borrow::Cow, ffi::OsString};

use std::fs::Metadata;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::time::Duration;

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd"
))]
pub use libc::statfs as StatFs;
#[cfg(any(
    target_os = "aix",
    target_os = "netbsd",
    target_os = "dragonfly",
    target_os = "illumos",
    target_os = "solaris",
    target_os = "redox"
))]
pub use libc::statvfs as StatFs;

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd",
))]
pub use libc::statfs as statfs_fn;
#[cfg(any(
    target_os = "aix",
    target_os = "netbsd",
    target_os = "illumos",
    target_os = "solaris",
    target_os = "dragonfly",
    target_os = "redox"
))]
pub use libc::statvfs as statfs_fn;

#[derive(Debug, Copy, Clone)]
pub enum MetadataTimeField {
    Modification,
    Access,
    Change,
    Birth,
}

impl From<&str> for MetadataTimeField {
    /// Get a `MetadataTimeField` from a string, we expect the value
    /// to come from clap, and be constrained there (e.g. if Modification is
    /// not supported), and the default branch should not be reached.
    fn from(value: &str) -> Self {
        match value {
            "ctime" | "status" => Self::Change,
            "access" | "atime" | "use" => Self::Access,
            "mtime" | "modification" => Self::Modification,
            "birth" | "creation" => Self::Birth,
            // below should never happen as clap already restricts the values.
            _ => unreachable!("Invalid metadata time field."),
        }
    }
}

#[cfg(unix)]
fn metadata_get_change_time(md: &Metadata) -> Option<SystemTime> {
    let mut st = UNIX_EPOCH;
    let (secs, nsecs) = (md.ctime(), md.ctime_nsec());
    if secs >= 0 {
        st += Duration::from_secs(secs as u64);
    } else {
        st -= Duration::from_secs(-secs as u64);
    }
    if nsecs >= 0 {
        st += Duration::from_nanos(nsecs as u64);
    } else {
        // Probably never the case, but cover just in case.
        st -= Duration::from_nanos(-nsecs as u64);
    }
    Some(st)
}

#[cfg(not(unix))]
fn metadata_get_change_time(_md: &Metadata) -> Option<SystemTime> {
    // Not available.
    None
}

pub fn metadata_get_time(md: &Metadata, md_time: MetadataTimeField) -> Option<SystemTime> {
    match md_time {
        MetadataTimeField::Change => metadata_get_change_time(md),
        MetadataTimeField::Modification => md.modified().ok(),
        MetadataTimeField::Access => md.accessed().ok(),
        MetadataTimeField::Birth => md.created().ok(),
    }
}

// TODO: Types for this struct are probably mostly wrong. Possibly, most of them
// should be OsString.
#[derive(Debug, Clone)]
pub struct MountInfo {
    /// Stores `volume_name` in windows platform and `dev_id` in unix platform
    pub dev_id: String,
    pub dev_name: String,
    pub fs_type: String,
    pub mount_root: OsString,
    pub mount_dir: OsString,
    /// We only care whether this field contains "bind"
    pub mount_option: String,
    pub remote: bool,
    pub dummy: bool,
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn replace_special_chars(s: &[u8]) -> Vec<u8> {
    use bstr::ByteSlice;

    // Replace
    //
    // * ASCII space with a regular space character,
    // * \011 ASCII horizontal tab with a tab character,
    // * ASCII backslash with an actual backslash character.
    //
    s.replace(r#"\040"#, " ")
        .replace(r#"\011"#, "	")
        .replace(r#"\134"#, r#"\"#)
}

impl MountInfo {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn new(file_name: &str, raw: &[&[u8]]) -> Option<Self> {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::ffi::OsStringExt;

        let dev_name;
        let fs_type;
        let mount_root;
        let mount_dir;
        let mount_option;

        match file_name {
            // spell-checker:ignore (word) noatime
            // Format: 36 35 98:0 /mnt1 /mnt2 rw,noatime master:1 - ext3 /dev/root rw,errors=continue
            // "man proc" for more details
            LINUX_MOUNTINFO => {
                const FIELDS_OFFSET: usize = 6;
                let after_fields = raw[FIELDS_OFFSET..]
                    .iter()
                    .position(|c| *c == b"-")
                    .unwrap()
                    + FIELDS_OFFSET
                    + 1;
                dev_name = String::from_utf8_lossy(raw[after_fields + 1]).to_string();
                fs_type = String::from_utf8_lossy(raw[after_fields]).to_string();
                mount_root = OsStr::from_bytes(raw[3]).to_owned();
                mount_dir = OsString::from_vec(replace_special_chars(raw[4]));
                mount_option = String::from_utf8_lossy(raw[5]).to_string();
            }
            LINUX_MTAB => {
                dev_name = String::from_utf8_lossy(raw[0]).to_string();
                fs_type = String::from_utf8_lossy(raw[2]).to_string();
                mount_root = OsString::new();
                mount_dir = OsString::from_vec(replace_special_chars(raw[1]));
                mount_option = String::from_utf8_lossy(raw[3]).to_string();
            }
            _ => return None,
        }

        let dev_id = mount_dev_id(&mount_dir);
        let dummy = is_dummy_filesystem(&fs_type, &mount_option);
        let remote = is_remote_filesystem(&dev_name, &fs_type);

        Some(Self {
            dev_id,
            dev_name,
            fs_type,
            mount_root,
            mount_dir,
            mount_option,
            remote,
            dummy,
        })
    }

    #[cfg(windows)]
    fn new(mut volume_name: String) -> Option<Self> {
        let mut dev_name_buf = [0u16; MAX_PATH];
        volume_name.pop();
        unsafe {
            QueryDosDeviceW(
                OsStr::new(&volume_name)
                    .encode_wide()
                    .chain(Some(0))
                    .skip(4)
                    .collect::<Vec<u16>>()
                    .as_ptr(),
                dev_name_buf.as_mut_ptr(),
                dev_name_buf.len() as u32,
            )
        };
        volume_name.push('\\');
        let dev_name = LPWSTR2String(&dev_name_buf);

        let mut mount_root_buf = [0u16; MAX_PATH];
        let success = unsafe {
            let volume_name = to_nul_terminated_wide_string(&volume_name);
            GetVolumePathNamesForVolumeNameW(
                volume_name.as_ptr(),
                mount_root_buf.as_mut_ptr(),
                mount_root_buf.len() as u32,
                ptr::null_mut(),
            )
        };
        if 0 == success {
            // TODO: support the case when `GetLastError()` returns `ERROR_MORE_DATA`
            return None;
        }
        // TODO: This should probably call `OsString::from_wide`, but unclear if
        // terminating zeros need to be striped first.
        let mount_root = LPWSTR2String(&mount_root_buf);

        let mut fs_type_buf = [0u16; MAX_PATH];
        let success = unsafe {
            let mount_root = to_nul_terminated_wide_string(&mount_root);
            GetVolumeInformationW(
                mount_root.as_ptr(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                fs_type_buf.as_mut_ptr(),
                fs_type_buf.len() as u32,
            )
        };
        let fs_type = if 0 == success {
            None
        } else {
            Some(LPWSTR2String(&fs_type_buf))
        };
        let remote = DRIVE_REMOTE
            == unsafe {
                let mount_root = to_nul_terminated_wide_string(&mount_root);
                GetDriveTypeW(mount_root.as_ptr())
            };
        Some(Self {
            dev_id: volume_name,
            dev_name,
            fs_type: fs_type.unwrap_or_default(),
            mount_root: mount_root.into(), // TODO: We should figure out how to keep an OsString here.
            mount_dir: OsString::new(),
            mount_option: String::new(),
            remote,
            dummy: false,
        })
    }
}

#[cfg(any(
    target_os = "freebsd",
    target_vendor = "apple",
    target_os = "netbsd",
    target_os = "openbsd",
))]
impl From<StatFs> for MountInfo {
    fn from(statfs: StatFs) -> Self {
        let dev_name = unsafe {
            // spell-checker:disable-next-line
            CStr::from_ptr(&statfs.f_mntfromname[0])
                .to_string_lossy()
                .into_owned()
        };
        let fs_type = unsafe {
            // spell-checker:disable-next-line
            CStr::from_ptr(&statfs.f_fstypename[0])
                .to_string_lossy()
                .into_owned()
        };
        let mount_dir_bytes = unsafe {
            // spell-checker:disable-next-line
            CStr::from_ptr(&statfs.f_mntonname[0]).to_bytes()
        };
        let mount_dir = os_str_from_bytes(mount_dir_bytes).unwrap().into_owned();

        let dev_id = mount_dev_id(&mount_dir);
        let dummy = is_dummy_filesystem(&fs_type, "");
        let remote = is_remote_filesystem(&dev_name, &fs_type);

        Self {
            dev_id,
            dev_name,
            fs_type,
            mount_dir,
            mount_root: OsString::new(),
            mount_option: String::new(),
            remote,
            dummy,
        }
    }
}

#[cfg(all(unix, not(any(target_os = "aix", target_os = "redox"))))]
fn is_dummy_filesystem(fs_type: &str, mount_option: &str) -> bool {
    // spell-checker:disable
    match fs_type {
        "autofs" | "proc" | "subfs"
        // for Linux 2.6/3.x
        | "debugfs" | "devpts" | "fusectl" | "mqueue" | "rpc_pipefs" | "sysfs"
        // FreeBSD, Linux 2.4
        | "devfs"
        // for NetBSD 3.0
        | "kernfs"
        // for Irix 6.5
        | "ignore" => true,
        _ => fs_type == "none"
            && !mount_option.contains(MOUNT_OPT_BIND)
    }
    // spell-checker:enable
}

#[cfg(all(unix, not(any(target_os = "aix", target_os = "redox"))))]
fn is_remote_filesystem(dev_name: &str, fs_type: &str) -> bool {
    dev_name.find(':').is_some()
        || (dev_name.starts_with("//") && fs_type == "smbfs" || fs_type == "cifs")
        || dev_name == "-hosts"
}

#[cfg(all(unix, not(any(target_os = "aix", target_os = "redox"))))]
fn mount_dev_id(mount_dir: &OsStr) -> String {
    use std::os::unix::fs::MetadataExt;

    if let Ok(stat) = std::fs::metadata(mount_dir) {
        // Why do we cast this to i32?
        (stat.dev() as i32).to_string()
    } else {
        String::new()
    }
}

#[cfg(any(
    target_os = "freebsd",
    target_vendor = "apple",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use libc::c_int;
#[cfg(any(
    target_os = "freebsd",
    target_vendor = "apple",
    target_os = "netbsd",
    target_os = "openbsd"
))]
unsafe extern "C" {
    #[cfg(all(target_vendor = "apple", target_arch = "x86_64"))]
    #[link_name = "getmntinfo$INODE64"]
    fn get_mount_info(mount_buffer_p: *mut *mut StatFs, flags: c_int) -> c_int;

    #[cfg(any(
        target_os = "netbsd",
        target_os = "openbsd",
        all(target_vendor = "apple", target_arch = "aarch64")
    ))]
    #[link_name = "getmntinfo"]
    fn get_mount_info(mount_buffer_p: *mut *mut StatFs, flags: c_int) -> c_int;

    // Rust on FreeBSD uses 11.x ABI for filesystem metadata syscalls.
    // Call the right version of the symbol for getmntinfo() result to
    // match libc StatFS layout.
    #[cfg(target_os = "freebsd")]
    #[link_name = "getmntinfo@FBSD_1.0"]
    fn get_mount_info(mount_buffer_p: *mut *mut StatFs, flags: c_int) -> c_int;
}

use crate::error::UResult;
#[cfg(any(
    target_os = "freebsd",
    target_vendor = "apple",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "windows"
))]
use crate::error::USimpleError;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs::File;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::io::{BufRead, BufReader};
#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "windows",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use std::ptr;
#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use std::slice;

/// Read file system list.
pub fn read_fs_list() -> UResult<Vec<MountInfo>> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let (file_name, f) = File::open(LINUX_MOUNTINFO)
            .map(|f| (LINUX_MOUNTINFO, f))
            .or_else(|_| File::open(LINUX_MTAB).map(|f| (LINUX_MTAB, f)))?;
        let reader = BufReader::new(f);
        Ok(reader
            .split(b'\n')
            .map_while(Result::ok)
            .filter_map(|line| {
                let raw_data = line.split(|c| *c == b' ').collect::<Vec<&[u8]>>();
                MountInfo::new(file_name, &raw_data)
            })
            .collect::<Vec<_>>())
    }
    #[cfg(any(
        target_os = "freebsd",
        target_vendor = "apple",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let mut mount_buffer_ptr: *mut StatFs = ptr::null_mut();
        let len = unsafe { get_mount_info(&mut mount_buffer_ptr, 1_i32) };
        if len < 0 {
            return Err(USimpleError::new(1, "get_mount_info() failed"));
        }
        let mounts = unsafe { slice::from_raw_parts(mount_buffer_ptr, len as usize) };
        Ok(mounts
            .iter()
            .map(|m| MountInfo::from(*m))
            .collect::<Vec<_>>())
    }
    #[cfg(windows)]
    {
        let mut volume_name_buf = [0u16; MAX_PATH];
        // As recommended in the MS documentation, retrieve the first volume before the others
        let find_handle =
            unsafe { FindFirstVolumeW(volume_name_buf.as_mut_ptr(), volume_name_buf.len() as u32) };
        if INVALID_HANDLE_VALUE == find_handle {
            let os_err = IOError::last_os_error();
            let msg = format!("FindFirstVolumeW failed: {os_err}");
            return Err(USimpleError::new(EXIT_ERR, msg));
        }
        let mut mounts = Vec::<MountInfo>::new();
        loop {
            let volume_name = LPWSTR2String(&volume_name_buf);
            if !volume_name.starts_with("\\\\?\\") || !volume_name.ends_with('\\') {
                show_warning!("A bad path was skipped: {volume_name}");
                continue;
            }
            if let Some(m) = MountInfo::new(volume_name) {
                mounts.push(m);
            }
            if 0 == unsafe {
                FindNextVolumeW(
                    find_handle,
                    volume_name_buf.as_mut_ptr(),
                    volume_name_buf.len() as u32,
                )
            } {
                let err = IOError::last_os_error();
                if err.raw_os_error() != Some(ERROR_NO_MORE_FILES as i32) {
                    let msg = format!("FindNextVolumeW failed: {err}");
                    return Err(USimpleError::new(EXIT_ERR, msg));
                }
                break;
            }
        }
        unsafe {
            FindVolumeClose(find_handle);
        }
        Ok(mounts)
    }
    #[cfg(any(
        target_os = "aix",
        target_os = "redox",
        target_os = "illumos",
        target_os = "solaris"
    ))]
    {
        // No method to read mounts, yet
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct FsUsage {
    pub blocksize: u64,
    pub blocks: u64,
    pub bfree: u64,
    pub bavail: u64,
    pub bavail_top_bit_set: bool,
    pub files: u64,
    pub ffree: u64,
}

impl FsUsage {
    #[cfg(unix)]
    pub fn new(statvfs: StatFs) -> Self {
        {
            #[cfg(all(
                not(any(target_os = "freebsd", target_os = "openbsd")),
                target_pointer_width = "64"
            ))]
            return Self {
                blocksize: statvfs.f_bsize as u64, // or `statvfs.f_frsize` ?
                blocks: statvfs.f_blocks,
                bfree: statvfs.f_bfree,
                bavail: statvfs.f_bavail,
                bavail_top_bit_set: ((statvfs.f_bavail) & (1u64.rotate_right(1))) != 0,
                files: statvfs.f_files,
                ffree: statvfs.f_ffree,
            };
            #[cfg(all(
                not(any(target_os = "freebsd", target_os = "openbsd")),
                not(target_pointer_width = "64")
            ))]
            return Self {
                blocksize: statvfs.f_bsize as u64, // or `statvfs.f_frsize` ?
                blocks: statvfs.f_blocks.into(),
                bfree: statvfs.f_bfree.into(),
                bavail: statvfs.f_bavail.into(),
                bavail_top_bit_set: ((statvfs.f_bavail as u64) & (1u64.rotate_right(1))) != 0,
                files: statvfs.f_files.into(),
                ffree: statvfs.f_ffree.into(),
            };
            #[cfg(target_os = "freebsd")]
            return Self {
                blocksize: statvfs.f_bsize, // or `statvfs.f_frsize` ?
                blocks: statvfs.f_blocks,
                bfree: statvfs.f_bfree,
                bavail: statvfs.f_bavail.try_into().unwrap(),
                bavail_top_bit_set: ((std::convert::TryInto::<u64>::try_into(statvfs.f_bavail)
                    .unwrap())
                    & (1u64.rotate_right(1)))
                    != 0,
                files: statvfs.f_files,
                ffree: statvfs.f_ffree.try_into().unwrap(),
            };
            #[cfg(target_os = "openbsd")]
            return Self {
                blocksize: statvfs.f_bsize.into(),
                blocks: statvfs.f_blocks,
                bfree: statvfs.f_bfree,
                bavail: statvfs.f_bavail.try_into().unwrap(),
                bavail_top_bit_set: ((std::convert::TryInto::<u64>::try_into(statvfs.f_bavail)
                    .unwrap())
                    & (1u64.rotate_right(1)))
                    != 0,
                files: statvfs.f_files,
                ffree: statvfs.f_ffree,
            };
        }
    }
    #[cfg(windows)]
    pub fn new(path: &Path) -> UResult<Self> {
        let mut root_path = [0u16; MAX_PATH];
        let success = unsafe {
            let path = to_nul_terminated_wide_string(path);
            GetVolumePathNamesForVolumeNameW(
                //path_utf8.as_ptr(),
                path.as_ptr(),
                root_path.as_mut_ptr(),
                root_path.len() as u32,
                ptr::null_mut(),
            )
        };
        if 0 == success {
            let msg = format!(
                "GetVolumePathNamesForVolumeNameW failed: {}",
                IOError::last_os_error()
            );
            return Err(USimpleError::new(EXIT_ERR, msg));
        }

        let mut sectors_per_cluster = 0;
        let mut bytes_per_sector = 0;
        let mut number_of_free_clusters = 0;
        let mut total_number_of_clusters = 0;

        unsafe {
            let path = to_nul_terminated_wide_string(path);
            GetDiskFreeSpaceW(
                path.as_ptr(),
                &mut sectors_per_cluster,
                &mut bytes_per_sector,
                &mut number_of_free_clusters,
                &mut total_number_of_clusters,
            );
        }

        let bytes_per_cluster = sectors_per_cluster as u64 * bytes_per_sector as u64;
        Ok(Self {
            // f_bsize      File system block size.
            blocksize: bytes_per_cluster,
            // f_blocks - Total number of blocks on the file system, in units of f_frsize.
            // frsize =     Fundamental file system block size (fragment size).
            blocks: total_number_of_clusters as u64,
            //  Total number of free blocks.
            bfree: number_of_free_clusters as u64,
            //  Total number of free blocks available to non-privileged processes.
            bavail: 0,
            bavail_top_bit_set: ((bytes_per_sector as u64) & (1u64.rotate_right(1))) != 0,
            // Total number of file nodes (inodes) on the file system.
            files: 0, // Not available on windows
            // Total number of free file nodes (inodes).
            ffree: 0, // Meaningless on Windows
        })
    }
}

#[cfg(unix)]
pub trait FsMeta {
    fn fs_type(&self) -> i64;
    fn io_size(&self) -> u64;
    fn block_size(&self) -> i64;
    fn total_blocks(&self) -> u64;
    fn free_blocks(&self) -> u64;
    fn avail_blocks(&self) -> u64;
    fn total_file_nodes(&self) -> u64;
    fn free_file_nodes(&self) -> u64;
    fn fsid(&self) -> u64;
    fn namelen(&self) -> u64;
}

#[cfg(unix)]
impl FsMeta for StatFs {
    fn block_size(&self) -> i64 {
        #[cfg(all(
            not(target_env = "musl"),
            not(target_vendor = "apple"),
            not(target_os = "aix"),
            not(target_os = "android"),
            not(target_os = "freebsd"),
            not(target_os = "netbsd"),
            not(target_os = "openbsd"),
            not(target_os = "illumos"),
            not(target_os = "solaris"),
            not(target_os = "redox"),
            not(target_arch = "s390x"),
            target_pointer_width = "64"
        ))]
        return self.f_bsize;
        #[cfg(all(
            not(target_env = "musl"),
            not(target_os = "freebsd"),
            not(target_os = "netbsd"),
            not(target_os = "redox"),
            any(
                target_arch = "s390x",
                target_vendor = "apple",
                all(target_os = "android", target_pointer_width = "32"),
                target_os = "openbsd",
                not(target_pointer_width = "64")
            )
        ))]
        return self.f_bsize.into();
        #[cfg(any(
            target_env = "musl",
            target_os = "aix",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "illumos",
            target_os = "solaris",
            target_os = "redox",
            all(target_os = "android", target_pointer_width = "64"),
        ))]
        return self.f_bsize.try_into().unwrap();
    }
    fn total_blocks(&self) -> u64 {
        #[cfg(target_pointer_width = "64")]
        return self.f_blocks;
        #[cfg(not(target_pointer_width = "64"))]
        return self.f_blocks.into();
    }
    fn free_blocks(&self) -> u64 {
        #[cfg(target_pointer_width = "64")]
        return self.f_bfree;
        #[cfg(not(target_pointer_width = "64"))]
        return self.f_bfree.into();
    }
    fn avail_blocks(&self) -> u64 {
        #[cfg(all(
            not(target_os = "freebsd"),
            not(target_os = "openbsd"),
            target_pointer_width = "64"
        ))]
        return self.f_bavail;
        #[cfg(all(
            not(target_os = "freebsd"),
            not(target_os = "openbsd"),
            not(target_pointer_width = "64")
        ))]
        return self.f_bavail.into();
        #[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
        return self.f_bavail.try_into().unwrap();
    }
    fn total_file_nodes(&self) -> u64 {
        #[cfg(target_pointer_width = "64")]
        return self.f_files;
        #[cfg(not(target_pointer_width = "64"))]
        return self.f_files.into();
    }
    fn free_file_nodes(&self) -> u64 {
        #[cfg(all(not(target_os = "freebsd"), target_pointer_width = "64"))]
        return self.f_ffree;
        #[cfg(all(not(target_os = "freebsd"), not(target_pointer_width = "64")))]
        return self.f_ffree.into();
        #[cfg(target_os = "freebsd")]
        return self.f_ffree.try_into().unwrap();
    }
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_vendor = "apple",
        target_os = "freebsd"
    ))]
    fn fs_type(&self) -> i64 {
        #[cfg(all(
            not(target_env = "musl"),
            not(target_vendor = "apple"),
            not(target_os = "android"),
            not(target_os = "freebsd"),
            not(target_arch = "s390x"),
            target_pointer_width = "64"
        ))]
        return self.f_type;
        #[cfg(all(
            not(target_env = "musl"),
            any(
                target_vendor = "apple",
                all(target_os = "android", target_pointer_width = "32"),
                target_os = "freebsd",
                target_arch = "s390x",
                not(target_pointer_width = "64")
            )
        ))]
        return self.f_type.into();
        #[cfg(any(
            target_env = "musl",
            all(target_os = "android", target_pointer_width = "64"),
        ))]
        return self.f_type.try_into().unwrap();
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_vendor = "apple",
        target_os = "freebsd"
    )))]
    fn fs_type(&self) -> i64 {
        // FIXME: statvfs doesn't have an equivalent, so we need to do something else
        unimplemented!()
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn io_size(&self) -> u64 {
        self.f_frsize as u64
    }
    #[cfg(any(target_vendor = "apple", target_os = "freebsd", target_os = "netbsd"))]
    fn io_size(&self) -> u64 {
        #[cfg(target_os = "freebsd")]
        return self.f_iosize;
        #[cfg(not(target_os = "freebsd"))]
        return self.f_iosize as u64;
    }
    // XXX: dunno if this is right
    #[cfg(not(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "android",
        target_os = "netbsd"
    )))]
    fn io_size(&self) -> u64 {
        self.f_bsize as u64
    }

    // Linux, SunOS, HP-UX, 4.4BSD, FreeBSD have a system call statfs() that returns
    // a struct statfs, containing a fsid_t f_fsid, where fsid_t is defined
    // as struct { int val[2];  }
    //
    // Solaris, Irix and POSIX have a system call statvfs(2) that returns a
    // struct statvfs, containing an  unsigned  long  f_fsid
    #[cfg(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "android",
        target_os = "openbsd"
    ))]
    fn fsid(&self) -> u64 {
        // Use type inference to determine the type of f_fsid
        // (libc::__fsid_t on Android, libc::fsid_t on other platforms)
        let f_fsid: &[u32; 2] = unsafe { &*(&raw const self.f_fsid as *const [u32; 2]) };
        ((u64::from(f_fsid[0])) << 32) | u64::from(f_fsid[1])
    }
    #[cfg(not(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "android",
        target_os = "openbsd"
    )))]
    fn fsid(&self) -> u64 {
        self.f_fsid as u64
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn namelen(&self) -> u64 {
        self.f_namelen as u64
    }
    #[cfg(target_vendor = "apple")]
    fn namelen(&self) -> u64 {
        1024
    }
    #[cfg(any(target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
    fn namelen(&self) -> u64 {
        self.f_namemax as u64 // spell-checker:disable-line
    }
    // XXX: should everything just use statvfs?
    #[cfg(not(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "android",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    fn namelen(&self) -> u64 {
        self.f_namemax as u64 // spell-checker:disable-line
    }
}

#[cfg(unix)]
pub fn statfs(path: &OsStr) -> Result<StatFs, String> {
    #[cfg(unix)]
    let p = path.as_bytes();
    #[cfg(not(unix))]
    let p = path.into_string().unwrap();

    match CString::new(p) {
        Ok(p) => {
            let mut buffer: StatFs = unsafe { mem::zeroed() };
            unsafe {
                match statfs_fn(p.as_ptr(), &mut buffer) {
                    0 => Ok(buffer),
                    _ => {
                        let errno = IOError::last_os_error().raw_os_error().unwrap_or(0);
                        Err(CStr::from_ptr(strerror(errno))
                            .to_str()
                            .map_err(|_| "Error message contains invalid UTF-8".to_owned())?
                            .to_owned())
                    }
                }
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(unix)]
pub fn pretty_filetype(mode: mode_t, size: u64) -> String {
    match mode & S_IFMT {
        S_IFREG => {
            if size == 0 {
                "regular empty file"
            } else {
                "regular file"
            }
        }
        S_IFDIR => "directory",
        S_IFLNK => "symbolic link",
        S_IFCHR => "character special file",
        S_IFBLK => "block special file",
        S_IFIFO => "fifo",
        S_IFSOCK => "socket",
        // TODO: Other file types
        _ => return format!("weird file ({:07o})", mode & S_IFMT),
    }
    .to_owned()
}

pub fn pretty_fstype<'a>(fstype: i64) -> Cow<'a, str> {
    // spell-checker:disable
    match fstype {
        0x6163_6673 => "acfs".into(),
        0xADF5 => "adfs".into(),
        0xADFF => "affs".into(),
        0x5346_414F => "afs".into(),
        0x0904_1934 => "anon-inode FS".into(),
        0x6175_6673 => "aufs".into(),
        0x0187 => "autofs".into(),
        0x4246_5331 => "befs".into(),
        0x6264_6576 => "bdevfs".into(),
        0xCA45_1A4E => "bcachefs".into(),
        0x1BAD_FACE => "bfs".into(),
        0xCAFE_4A11 => "bpf_fs".into(),
        0x4249_4E4D => "binfmt_misc".into(),
        0x9123_683E => "btrfs".into(),
        0x7372_7279 => "btrfs_test".into(),
        0x00C3_6400 => "ceph".into(),
        0x0027_E0EB => "cgroupfs".into(),
        0x6367_7270 => "cgroup2fs".into(),
        0xFF53_4D42 => "cifs".into(),
        0x7375_7245 => "coda".into(),
        0x012F_F7B7 => "coh".into(),
        0x6265_6570 => "configfs".into(),
        0x28CD_3D45 => "cramfs".into(),
        0x453D_CD28 => "cramfs-wend".into(),
        0x6462_6720 => "debugfs".into(),
        0x1373 => "devfs".into(),
        0x1CD1 => "devpts".into(),
        0xF15F => "ecryptfs".into(),
        0xDE5E_81E4 => "efivarfs".into(),
        0x0041_4A53 => "efs".into(),
        0x5DF5 => "exofs".into(),
        0x137D => "ext".into(),
        0xEF53 => "ext2/ext3".into(),
        0xEF51 => "ext2".into(),
        0xF2F5_2010 => "f2fs".into(),
        0x4006 => "fat".into(),
        0x1983_0326 => "fhgfs".into(),
        0x6573_5546 => "fuseblk".into(),
        0x6573_5543 => "fusectl".into(),
        0x0BAD_1DEA => "futexfs".into(),
        0x0116_1970 => "gfs/gfs2".into(),
        0x4750_4653 => "gpfs".into(),
        0x4244 => "hfs".into(),
        0x482B => "hfs+".into(),
        0x4858 => "hfsx".into(),
        0x00C0_FFEE => "hostfs".into(),
        0xF995_E849 => "hpfs".into(),
        0x9584_58F6 => "hugetlbfs".into(),
        0x1130_7854 => "inodefs".into(),
        0x0131_11A8 => "ibrix".into(),
        0x2BAD_1DEA => "inotifyfs".into(),
        0x9660 => "isofs".into(),
        0x4004 => "isofs".into(),
        0x4000 => "isofs".into(),
        0x07C0 => "jffs".into(),
        0x72B6 => "jffs2".into(),
        0x3153_464A => "jfs".into(),
        0x6B41_4653 => "k-afs".into(),
        0xC97E_8168 => "logfs".into(),
        0x0BD0_0BD0 => "lustre".into(),
        0x5346_314D => "m1fs".into(),
        0x137F => "minix".into(),
        0x138F => "minix (30 char.)".into(),
        0x2468 => "minix v2".into(),
        0x2478 => "minix v2 (30 char.)".into(),
        0x4D5A => "minix3".into(),
        0x1980_0202 => "mqueue".into(),
        0x4D44 => "msdos".into(),
        0x564C => "novell".into(),
        0x6969 => "nfs".into(),
        0x6E66_7364 => "nfsd".into(),
        0x3434 => "nilfs".into(),
        0x6E73_6673 => "nsfs".into(),
        0x5346_544E => "ntfs".into(),
        0x9FA1 => "openprom".into(),
        0x7461_636F => "ocfs2".into(),
        0x794C_7630 => "overlayfs".into(),
        0xAAD7_AAEA => "panfs".into(),
        0x5049_5045 => "pipefs".into(),
        0x7C7C_6673 => "prl_fs".into(),
        0x9FA0 => "proc".into(),
        0x6165_676C => "pstorefs".into(),
        0x002F => "qnx4".into(),
        0x6819_1122 => "qnx6".into(),
        0x8584_58F6 => "ramfs".into(),
        0x5265_4973 => "reiserfs".into(),
        0x7275 => "romfs".into(),
        0x6759_6969 => "rpc_pipefs".into(),
        0x7363_6673 => "securityfs".into(),
        0xF97C_FF8C => "selinux".into(),
        0x4341_5D53 => "smackfs".into(),
        0x517B => "smb".into(),
        0xFE53_4D42 => "smb2".into(),
        0xBEEF_DEAD => "snfs".into(),
        0x534F_434B => "sockfs".into(),
        0x7371_7368 => "squashfs".into(),
        0x6265_6572 => "sysfs".into(),
        0x012F_F7B6 => "sysv2".into(),
        0x012F_F7B5 => "sysv4".into(),
        0x0102_1994 => "tmpfs".into(),
        0x7472_6163 => "tracefs".into(),
        0x2405_1905 => "ubifs".into(),
        0x1501_3346 => "udf".into(),
        0x0001_1954 => "ufs".into(),
        0x5419_0100 => "ufs".into(),
        0x9FA2 => "usbdevfs".into(),
        0x0102_1997 => "v9fs".into(),
        0xBACB_ACBC => "vmhgfs".into(),
        0xA501_FCF5 => "vxfs".into(),
        0x565A_4653 => "vzfs".into(),
        0x5346_4846 => "wslfs".into(),
        0xABBA_1974 => "xenfs".into(),
        0x012F_F7B4 => "xenix".into(),
        0x5846_5342 => "xfs".into(),
        0x012F_D16D => "xia".into(),
        0x2FC1_2FC1 => "zfs".into(),
        0xDE => "zfs".into(),
        other => format!("UNKNOWN ({other:#x})").into(),
    }
    // spell-checker:enable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn test_file_type() {
        assert_eq!("block special file", pretty_filetype(S_IFBLK, 0));
        assert_eq!("character special file", pretty_filetype(S_IFCHR, 0));
        assert_eq!("regular file", pretty_filetype(S_IFREG, 1));
        assert_eq!("regular empty file", pretty_filetype(S_IFREG, 0));
        assert_eq!("weird file (0000000)", pretty_filetype(0, 0));
    }

    #[test]
    fn test_fs_type() {
        // spell-checker:disable
        assert_eq!("ext2/ext3", pretty_fstype(0xEF53));
        assert_eq!("tmpfs", pretty_fstype(0x0102_1994));
        assert_eq!("nfs", pretty_fstype(0x6969));
        assert_eq!("btrfs", pretty_fstype(0x9123_683e));
        assert_eq!("xfs", pretty_fstype(0x5846_5342));
        assert_eq!("zfs", pretty_fstype(0x2FC1_2FC1));
        assert_eq!("ntfs", pretty_fstype(0x5346_544e));
        assert_eq!("fat", pretty_fstype(0x4006));
        assert_eq!("UNKNOWN (0x1234)", pretty_fstype(0x1234));
        // spell-checker:enable
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn test_mountinfo() {
        // spell-checker:ignore (word) relatime
        let info = MountInfo::new(
            LINUX_MOUNTINFO,
            &b"106 109 253:6 / /mnt rw,relatime - xfs /dev/fs0 rw"
                .split(|c| *c == b' ')
                .collect::<Vec<_>>(),
        )
        .unwrap();

        assert_eq!(info.mount_root, "/");
        assert_eq!(info.mount_dir, "/mnt");
        assert_eq!(info.mount_option, "rw,relatime");
        assert_eq!(info.fs_type, "xfs");
        assert_eq!(info.dev_name, "/dev/fs0");

        // Test parsing with different amounts of optional fields.
        let info = MountInfo::new(
            LINUX_MOUNTINFO,
            &b"106 109 253:6 / /mnt rw,relatime master:1 - xfs /dev/fs0 rw"
                .split(|c| *c == b' ')
                .collect::<Vec<_>>(),
        )
        .unwrap();

        assert_eq!(info.fs_type, "xfs");
        assert_eq!(info.dev_name, "/dev/fs0");

        let info = MountInfo::new(
            LINUX_MOUNTINFO,
            &b"106 109 253:6 / /mnt rw,relatime master:1 shared:2 - xfs /dev/fs0 rw"
                .split(|c| *c == b' ')
                .collect::<Vec<_>>(),
        )
        .unwrap();

        assert_eq!(info.fs_type, "xfs");
        assert_eq!(info.dev_name, "/dev/fs0");
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn test_mountinfo_dir_special_chars() {
        let info = MountInfo::new(
            LINUX_MOUNTINFO,
            &br#"317 61 7:0 / /mnt/f\134\040\011oo rw,relatime shared:641 - ext4 /dev/loop0 rw"#
                .split(|c| *c == b' ')
                .collect::<Vec<_>>(),
        )
        .unwrap();

        assert_eq!(info.mount_dir, r#"/mnt/f\ 	oo"#);

        let info = MountInfo::new(
            LINUX_MTAB,
            &br#"/dev/loop0 /mnt/f\134\040\011oo ext4 rw,relatime 0 0"#
                .split(|c| *c == b' ')
                .collect::<Vec<_>>(),
        )
        .unwrap();

        assert_eq!(info.mount_dir, r#"/mnt/f\ 	oo"#);
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn test_mountinfo_dir_non_unicode() {
        let info = MountInfo::new(
            LINUX_MOUNTINFO,
            &b"317 61 7:0 / /mnt/some-\xc0-dir-\xf3 rw,relatime shared:641 - ext4 /dev/loop0 rw"
                .split(|c| *c == b' ')
                .collect::<Vec<_>>(),
        )
        .unwrap();

        assert_eq!(
            info.mount_dir,
            crate::os_str_from_bytes(b"/mnt/some-\xc0-dir-\xf3").unwrap()
        );

        let info = MountInfo::new(
            LINUX_MOUNTINFO,
            &b"317 61 7:0 / /mnt/some-\\040-dir-\xf3 rw,relatime shared:641 - ext4 /dev/loop0 rw"
                .split(|c| *c == b' ')
                .collect::<Vec<_>>(),
        )
        .unwrap();

        // Note that the \040 above will have been substituted by a space.
        assert_eq!(
            info.mount_dir,
            crate::os_str_from_bytes(b"/mnt/some- -dir-\xf3").unwrap()
        );
    }
}
