// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
// (c) Fangxu Hu <framlog@gmail.com>
// (c) Sylvestre Ledru <sylvestre@debian.org>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

//! Set of functions to manage file systems

// spell-checker:ignore (arch) bitrig ; (fs) cifs smbfs

extern crate time;

pub use crate::*; // import macros from `../../macros.rs`

#[cfg(any(target_os = "linux", target_os = "android"))]
const LINUX_MTAB: &str = "/etc/mtab";
#[cfg(any(target_os = "linux", target_os = "android"))]
const LINUX_MOUNTINFO: &str = "/proc/self/mountinfo";
static MOUNT_OPT_BIND: &str = "bind";
#[cfg(windows)]
const MAX_PATH: usize = 266;
#[cfg(not(unix))]
static EXIT_ERR: i32 = 1;

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use winapi::shared::minwindef::DWORD;
#[cfg(windows)]
use winapi::um::fileapi::GetDiskFreeSpaceW;
#[cfg(windows)]
use winapi::um::fileapi::{
    FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetDriveTypeW, GetVolumeInformationW,
    GetVolumePathNamesForVolumeNameW, QueryDosDeviceW,
};
#[cfg(windows)]
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
#[cfg(windows)]
use winapi::um::winbase::DRIVE_REMOTE;

// Warning: the pointer has to be used *immediately* or the Vec
// it points to will be dropped!
#[cfg(windows)]
macro_rules! String2LPWSTR {
    ($str: expr) => {
        OsStr::new(&$str)
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>()
            .as_ptr()
    };
}

#[cfg(windows)]
#[allow(non_snake_case)]
fn LPWSTR2String(buf: &[u16]) -> String {
    let len = buf.iter().position(|&n| n == 0).unwrap();
    String::from_utf16(&buf[..len]).unwrap()
}

use self::time::Timespec;
#[cfg(unix)]
use libc::{
    mode_t, strerror, S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK,
};
use std::borrow::Cow;
use std::convert::{AsRef, From};
#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "linux",
    target_os = "android",
))]
use std::ffi::CStr;
#[cfg(not(windows))]
use std::ffi::CString;
use std::io::Error as IOError;
#[cfg(unix)]
use std::mem;
use std::path::Path;
use std::time::UNIX_EPOCH;

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd"
))]
pub use libc::statfs as StatFs;
#[cfg(any(
    target_os = "netbsd",
    target_os = "bitrig",
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
    target_os = "redox"
))]
pub use libc::statfs as statfs_fn;
#[cfg(any(
    target_os = "netbsd",
    target_os = "bitrig",
    target_os = "illumos",
    target_os = "solaris",
    target_os = "dragonfly"
))]
pub use libc::statvfs as statfs_fn;

pub trait BirthTime {
    fn pretty_birth(&self) -> String;
    fn birth(&self) -> String;
}

use std::fs::Metadata;
impl BirthTime for Metadata {
    fn pretty_birth(&self) -> String {
        self.created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|e| pretty_time(e.as_secs() as i64, i64::from(e.subsec_nanos())))
            .unwrap_or_else(|| "-".to_owned())
    }

    fn birth(&self) -> String {
        self.created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|e| format!("{}", e.as_secs()))
            .unwrap_or_else(|| "0".to_owned())
    }
}

#[derive(Debug, Clone)]
pub struct MountInfo {
    // it stores `volume_name` in windows platform and `dev_id` in unix platform
    pub dev_id: String,
    pub dev_name: String,
    pub fs_type: String,
    pub mount_dir: String,
    pub mount_option: String, // we only care "bind" option
    pub mount_root: String,
    pub remote: bool,
    pub dummy: bool,
}

impl MountInfo {
    fn set_missing_fields(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            // We want to keep the dev_id on Windows
            // but set dev_id
            if let Ok(stat) = std::fs::metadata(&self.mount_dir) {
                // Why do we cast this to i32?
                self.dev_id = (stat.dev() as i32).to_string();
            } else {
                self.dev_id = "".to_string();
            }
        }
        // set MountInfo::dummy
        // spell-checker:disable
        match self.fs_type.as_ref() {
            "autofs" | "proc" | "subfs"
            /* for Linux 2.6/3.x */
            | "debugfs" | "devpts" | "fusectl" | "mqueue" | "rpc_pipefs" | "sysfs"
            /* FreeBSD, Linux 2.4 */
            | "devfs"
            /* for NetBSD 3.0 */
            | "kernfs"
            /* for Irix 6.5 */
            | "ignore" => self.dummy = true,
            _ => self.dummy = self.fs_type == "none"
                && !self.mount_option.contains(MOUNT_OPT_BIND)
        }
        // spell-checker:enable
        // set MountInfo::remote
        #[cfg(windows)]
        {
            self.remote = DRIVE_REMOTE == unsafe { GetDriveTypeW(String2LPWSTR!(self.mount_root)) };
        }
        #[cfg(unix)]
        {
            if self.dev_name.find(':').is_some()
                || (self.dev_name.starts_with("//") && self.fs_type == "smbfs"
                    || self.fs_type == "cifs")
                || self.dev_name == "-hosts"
            {
                self.remote = true;
            } else {
                self.remote = false;
            }
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn new(file_name: &str, raw: &[&str]) -> Option<Self> {
        match file_name {
            // spell-checker:ignore (word) noatime
            // Format: 36 35 98:0 /mnt1 /mnt2 rw,noatime master:1 - ext3 /dev/root rw,errors=continue
            // "man proc" for more details
            LINUX_MOUNTINFO => {
                const FIELDS_OFFSET: usize = 6;
                let after_fields = raw[FIELDS_OFFSET..].iter().position(|c| *c == "-").unwrap()
                    + FIELDS_OFFSET
                    + 1;
                let mut m = Self {
                    dev_id: "".to_string(),
                    dev_name: raw[after_fields + 1].to_string(),
                    fs_type: raw[after_fields].to_string(),
                    mount_root: raw[3].to_string(),
                    mount_dir: raw[4].to_string(),
                    mount_option: raw[5].to_string(),
                    remote: false,
                    dummy: false,
                };
                m.set_missing_fields();
                Some(m)
            }
            LINUX_MTAB => {
                let mut m = Self {
                    dev_id: "".to_string(),
                    dev_name: raw[0].to_string(),
                    fs_type: raw[2].to_string(),
                    mount_root: "".to_string(),
                    mount_dir: raw[1].to_string(),
                    mount_option: raw[3].to_string(),
                    remote: false,
                    dummy: false,
                };
                m.set_missing_fields();
                Some(m)
            }
            _ => None,
        }
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
                dev_name_buf.len() as DWORD,
            )
        };
        volume_name.push('\\');
        let dev_name = LPWSTR2String(&dev_name_buf);

        let mut mount_root_buf = [0u16; MAX_PATH];
        let success = unsafe {
            GetVolumePathNamesForVolumeNameW(
                String2LPWSTR!(volume_name),
                mount_root_buf.as_mut_ptr(),
                mount_root_buf.len() as DWORD,
                ptr::null_mut(),
            )
        };
        if 0 == success {
            // TODO: support the case when `GetLastError()` returns `ERROR_MORE_DATA`
            return None;
        }
        let mount_root = LPWSTR2String(&mount_root_buf);

        let mut fs_type_buf = [0u16; MAX_PATH];
        let success = unsafe {
            GetVolumeInformationW(
                String2LPWSTR!(mount_root),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                fs_type_buf.as_mut_ptr(),
                fs_type_buf.len() as DWORD,
            )
        };
        let fs_type = if 0 != success {
            Some(LPWSTR2String(&fs_type_buf))
        } else {
            None
        };
        let mut mn_info = Self {
            dev_id: volume_name,
            dev_name,
            fs_type: fs_type.unwrap_or_else(|| "".to_string()),
            mount_root,
            mount_dir: "".to_string(),
            mount_option: "".to_string(),
            remote: false,
            dummy: false,
        };
        mn_info.set_missing_fields();
        Some(mn_info)
    }
}

#[cfg(any(
    target_os = "freebsd",
    target_vendor = "apple",
    target_os = "netbsd",
    target_os = "openbsd"
))]
impl From<StatFs> for MountInfo {
    fn from(statfs: StatFs) -> Self {
        let mut info = Self {
            dev_id: "".to_string(),
            dev_name: unsafe {
                // spell-checker:disable-next-line
                CStr::from_ptr(&statfs.f_mntfromname[0])
                    .to_string_lossy()
                    .into_owned()
            },
            fs_type: unsafe {
                // spell-checker:disable-next-line
                CStr::from_ptr(&statfs.f_fstypename[0])
                    .to_string_lossy()
                    .into_owned()
            },
            mount_dir: unsafe {
                // spell-checker:disable-next-line
                CStr::from_ptr(&statfs.f_mntonname[0])
                    .to_string_lossy()
                    .into_owned()
            },
            mount_root: "".to_string(),
            mount_option: "".to_string(),
            remote: false,
            dummy: false,
        };
        info.set_missing_fields();
        info
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
extern "C" {
    #[cfg(all(target_vendor = "apple", target_arch = "x86_64"))]
    #[link_name = "getmntinfo$INODE64"] // spell-checker:disable-line
    fn get_mount_info(mount_buffer_p: *mut *mut StatFs, flags: c_int) -> c_int;

    #[cfg(any(
        all(target_os = "freebsd"),
        all(target_os = "netbsd"),
        all(target_os = "openbsd"),
        all(target_vendor = "apple", target_arch = "aarch64")
    ))]
    #[link_name = "getmntinfo"] // spell-checker:disable-line
    fn get_mount_info(mount_buffer_p: *mut *mut StatFs, flags: c_int) -> c_int;
}

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
pub fn read_fs_list() -> Vec<MountInfo> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let (file_name, f) = File::open(LINUX_MOUNTINFO)
            .map(|f| (LINUX_MOUNTINFO, f))
            .or_else(|_| File::open(LINUX_MTAB).map(|f| (LINUX_MTAB, f)))
            .expect("failed to find mount list files");
        let reader = BufReader::new(f);
        reader
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| {
                let raw_data = line.split_whitespace().collect::<Vec<&str>>();
                MountInfo::new(file_name, &raw_data)
            })
            .collect::<Vec<_>>()
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
            crash!(1, "get_mount_info() failed");
        }
        let mounts = unsafe { slice::from_raw_parts(mount_buffer_ptr, len as usize) };
        mounts
            .iter()
            .map(|m| MountInfo::from(*m))
            .collect::<Vec<_>>()
    }
    #[cfg(windows)]
    {
        let mut volume_name_buf = [0u16; MAX_PATH];
        // As recommended in the MS documentation, retrieve the first volume before the others
        let find_handle = unsafe {
            FindFirstVolumeW(volume_name_buf.as_mut_ptr(), volume_name_buf.len() as DWORD)
        };
        if INVALID_HANDLE_VALUE == find_handle {
            crash!(
                EXIT_ERR,
                "FindFirstVolumeW failed: {}",
                IOError::last_os_error()
            );
        }
        let mut mounts = Vec::<MountInfo>::new();
        loop {
            let volume_name = LPWSTR2String(&volume_name_buf);
            if !volume_name.starts_with("\\\\?\\") || !volume_name.ends_with('\\') {
                show_warning!("A bad path was skipped: {}", volume_name);
                continue;
            }
            if let Some(m) = MountInfo::new(volume_name) {
                mounts.push(m);
            }
            if 0 == unsafe {
                FindNextVolumeW(
                    find_handle,
                    volume_name_buf.as_mut_ptr(),
                    volume_name_buf.len() as DWORD,
                )
            } {
                let err = IOError::last_os_error();
                if err.raw_os_error() != Some(winapi::shared::winerror::ERROR_NO_MORE_FILES as i32)
                {
                    crash!(EXIT_ERR, "FindNextVolumeW failed: {}", err);
                }
                break;
            }
        }
        unsafe {
            FindVolumeClose(find_handle);
        }
        mounts
    }
    #[cfg(any(target_os = "redox", target_os = "illumos", target_os = "solaris"))]
    {
        // No method to read mounts, yet
        Vec::new()
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
            Self {
                blocksize: statvfs.f_bsize as u64, // or `statvfs.f_frsize` ?
                blocks: statvfs.f_blocks as u64,
                bfree: statvfs.f_bfree as u64,
                bavail: statvfs.f_bavail as u64,
                bavail_top_bit_set: ((statvfs.f_bavail as u64) & (1u64.rotate_right(1))) != 0,
                files: statvfs.f_files as u64,
                ffree: statvfs.f_ffree as u64,
            }
        }
    }
    #[cfg(not(unix))]
    pub fn new(path: &Path) -> Self {
        let mut root_path = [0u16; MAX_PATH];
        let success = unsafe {
            GetVolumePathNamesForVolumeNameW(
                //path_utf8.as_ptr(),
                String2LPWSTR!(path.as_os_str()),
                root_path.as_mut_ptr(),
                root_path.len() as DWORD,
                ptr::null_mut(),
            )
        };
        if 0 == success {
            crash!(
                EXIT_ERR,
                "GetVolumePathNamesForVolumeNameW failed: {}",
                IOError::last_os_error()
            );
        }

        let mut sectors_per_cluster = 0;
        let mut bytes_per_sector = 0;
        let mut number_of_free_clusters = 0;
        let mut total_number_of_clusters = 0;

        let success = unsafe {
            GetDiskFreeSpaceW(
                String2LPWSTR!(path.as_os_str()),
                &mut sectors_per_cluster,
                &mut bytes_per_sector,
                &mut number_of_free_clusters,
                &mut total_number_of_clusters,
            )
        };
        if 0 == success {
            // Fails in case of CD for example
            // crash!(
            //     EXIT_ERR,
            //     "GetDiskFreeSpaceW failed: {}",
            //     IOError::last_os_error()
            // );
        }

        let bytes_per_cluster = sectors_per_cluster as u64 * bytes_per_sector as u64;
        Self {
            // f_bsize      File system block size.
            blocksize: bytes_per_cluster as u64,
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
        }
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
        self.f_bsize as i64
    }
    fn total_blocks(&self) -> u64 {
        self.f_blocks as u64
    }
    fn free_blocks(&self) -> u64 {
        self.f_bfree as u64
    }
    fn avail_blocks(&self) -> u64 {
        self.f_bavail as u64
    }
    fn total_file_nodes(&self) -> u64 {
        self.f_files as u64
    }
    fn free_file_nodes(&self) -> u64 {
        self.f_ffree as u64
    }
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_vendor = "apple",
        target_os = "freebsd"
    ))]
    fn fs_type(&self) -> i64 {
        self.f_type as i64
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
        self.f_iosize as u64
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
        let f_fsid: &[u32; 2] =
            unsafe { &*(&self.f_fsid as *const nix::sys::statfs::fsid_t as *const [u32; 2]) };
        (u64::from(f_fsid[0])) << 32 | u64::from(f_fsid[1])
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
pub fn statfs<P: AsRef<Path>>(path: P) -> Result<StatFs, String>
where
    Vec<u8>: From<P>,
{
    match CString::new(path) {
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

pub fn pretty_time(sec: i64, nsec: i64) -> String {
    // sec == seconds since UNIX_EPOCH
    // nsec == nanoseconds since (UNIX_EPOCH + sec)
    let tm = time::at(Timespec::new(sec, nsec as i32));
    let res = time::strftime("%Y-%m-%d %H:%M:%S.%f %z", &tm).unwrap();
    if res.ends_with(" -0000") {
        res.replace(" -0000", " +0000")
    } else {
        res
    }
}

#[cfg(unix)]
pub fn pretty_filetype<'a>(mode: mode_t, size: u64) -> &'a str {
    match mode & S_IFMT {
        S_IFREG => {
            if size != 0 {
                "regular file"
            } else {
                "regular empty file"
            }
        }
        S_IFDIR => "directory",
        S_IFLNK => "symbolic link",
        S_IFCHR => "character special file",
        S_IFBLK => "block special file",
        S_IFIFO => "fifo",
        S_IFSOCK => "socket",
        // TODO: Other file types
        // See coreutils/gnulib/lib/file-type.c // spell-checker:disable-line
        _ => "weird file",
    }
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
        0x1BAD_FACE => "bfs".into(),
        0xCAFE_4A11 => "bpf_fs".into(),
        0x4249_4E4D => "binfmt_misc".into(),
        0x9123_683E => "btrfs".into(),
        0x7372_7279 => "btrfs_test".into(),
        0x00C3_6400 => "ceph".into(),
        0x0027_E0EB => "cgroupfs".into(),
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
        other => format!("UNKNOWN ({:#x})", other).into(),
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
        assert_eq!("weird file", pretty_filetype(0, 0));
    }

    #[test]
    fn test_fs_type() {
        // spell-checker:disable
        assert_eq!("ext2/ext3", pretty_fstype(0xEF53));
        assert_eq!("tmpfs", pretty_fstype(0x01021994));
        assert_eq!("nfs", pretty_fstype(0x6969));
        assert_eq!("btrfs", pretty_fstype(0x9123683e));
        assert_eq!("xfs", pretty_fstype(0x58465342));
        assert_eq!("zfs", pretty_fstype(0x2FC12FC1));
        assert_eq!("ntfs", pretty_fstype(0x5346544e));
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
            &"106 109 253:6 / /mnt rw,relatime - xfs /dev/fs0 rw"
                .split_ascii_whitespace()
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
            &"106 109 253:6 / /mnt rw,relatime master:1 - xfs /dev/fs0 rw"
                .split_ascii_whitespace()
                .collect::<Vec<_>>(),
        )
        .unwrap();

        assert_eq!(info.fs_type, "xfs");
        assert_eq!(info.dev_name, "/dev/fs0");

        let info = MountInfo::new(
            LINUX_MOUNTINFO,
            &"106 109 253:6 / /mnt rw,relatime master:1 shared:2 - xfs /dev/fs0 rw"
                .split_ascii_whitespace()
                .collect::<Vec<_>>(),
        )
        .unwrap();

        assert_eq!(info.fs_type, "xfs");
        assert_eq!(info.dev_name, "/dev/fs0");
    }
}
