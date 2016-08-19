// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

pub use super::uucore::libc;
extern crate time;

use self::time::Timespec;
pub use libc::{S_IFMT, S_IFDIR, S_IFCHR, S_IFBLK, S_IFREG, S_IFIFO, S_IFLNK, S_IFSOCK, S_ISUID, S_ISGID, S_ISVTX,
               S_IRUSR, S_IWUSR, S_IXUSR, S_IRGRP, S_IWGRP, S_IXGRP, S_IROTH, S_IWOTH, S_IXOTH, mode_t, c_int,
               strerror};

pub trait BirthTime {
    fn pretty_birth(&self) -> String;
    fn birth(&self) -> String;
}

use std::fs::Metadata;
impl BirthTime for Metadata {
    #[cfg(feature = "nightly")]
    fn pretty_birth(&self) -> String {
        self.created()
            .map(|t| t.elapsed().unwrap())
            .map(|e| pretty_time(e.as_secs() as i64, e.subsec_nanos() as i64))
            .unwrap_or("-".to_owned())
    }
    #[cfg(not(feature = "nightly"))]
    fn pretty_birth(&self) -> String {
        "-".to_owned()
    }
    #[cfg(feature = "nightly")]
    fn birth(&self) -> String {
        self.created()
            .map(|t| t.elapsed().unwrap())
            .map(|e| format!("{}", e.as_secs()))
            .unwrap_or("0".to_owned())
    }
    #[cfg(not(feature = "nightly"))]
    fn birth(&self) -> String {
        "0".to_owned()
    }
}

#[macro_export]
macro_rules! has {
    ($mode:expr, $perm:expr) => (
        $mode & $perm != 0
    )
}

pub fn pretty_time(sec: i64, nsec: i64) -> String {
    let tm = time::at(Timespec::new(sec, nsec as i32));
    let res = time::strftime("%Y-%m-%d %H:%M:%S.%f %z", &tm).unwrap();
    if res.ends_with(" -0000") {
        res.replace(" -0000", " +0000")
    } else {
        res
    }
}

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
        // See coreutils/gnulib/lib/file-type.c
        _ => "weird file",
    }
}

pub fn pretty_access(mode: mode_t) -> String {
    let mut result = String::with_capacity(10);
    result.push(match mode & S_IFMT {
        S_IFDIR => 'd',
        S_IFCHR => 'c',
        S_IFBLK => 'b',
        S_IFREG => '-',
        S_IFIFO => 'p',
        S_IFLNK => 'l',
        S_IFSOCK => 's',
        // TODO: Other file types
        // See coreutils/gnulib/lib/filemode.c
        _ => '?',
    });

    result.push(if has!(mode, S_IRUSR) {
        'r'
    } else {
        '-'
    });
    result.push(if has!(mode, S_IWUSR) {
        'w'
    } else {
        '-'
    });
    result.push(if has!(mode, S_ISUID as mode_t) {
        if has!(mode, S_IXUSR) {
            's'
        } else {
            'S'
        }
    } else if has!(mode, S_IXUSR) {
        'x'
    } else {
        '-'
    });

    result.push(if has!(mode, S_IRGRP) {
        'r'
    } else {
        '-'
    });
    result.push(if has!(mode, S_IWGRP) {
        'w'
    } else {
        '-'
    });
    result.push(if has!(mode, S_ISGID as mode_t) {
        if has!(mode, S_IXGRP) {
            's'
        } else {
            'S'
        }
    } else if has!(mode, S_IXGRP) {
        'x'
    } else {
        '-'
    });

    result.push(if has!(mode, S_IROTH) {
        'r'
    } else {
        '-'
    });
    result.push(if has!(mode, S_IWOTH) {
        'w'
    } else {
        '-'
    });
    result.push(if has!(mode, S_ISVTX as mode_t) {
        if has!(mode, S_IXOTH) {
            't'
        } else {
            'T'
        }
    } else if has!(mode, S_IXOTH) {
        'x'
    } else {
        '-'
    });

    result
}

use std::mem::{self, transmute};
use std::path::Path;
use std::borrow::Cow;
use std::ffi::CString;
use std::convert::{AsRef, From};
use std::error::Error;
use std::io::Error as IOError;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "android"))]
use libc::statfs as Sstatfs;
// #[cfg(any(target_os = "openbsd", target_os = "netbsd", target_os = "openbsd", target_os = "bitrig", target_os = "dragonfly"))]
// use self::libc::statvfs as Sstatfs;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "android"))]
use libc::statfs as statfs_fn;
// #[cfg(any(target_os = "openbsd", target_os = "netbsd", target_os = "openbsd", target_os = "bitrig", target_os = "dragonfly"))]
// use self::libc::statvfs as statfs_fn;

pub trait FsMeta {
    fn fs_type(&self) -> i64;
    fn iosize(&self) -> i64;
    fn blksize(&self) -> i64;
    fn total_blocks(&self) -> u64;
    fn free_blocks(&self) -> u64;
    fn avail_blocks(&self) -> u64;
    fn total_fnodes(&self) -> u64;
    fn free_fnodes(&self) -> u64;
    fn fsid(&self) -> u64;
    fn namelen(&self) -> i64;
}

impl FsMeta for Sstatfs {
    fn blksize(&self) -> i64 {
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
    fn total_fnodes(&self) -> u64 {
        self.f_files as u64
    }
    fn free_fnodes(&self) -> u64 {
        self.f_ffree as u64
    }
    fn fs_type(&self) -> i64 {
        self.f_type as i64
    }

    #[cfg(target_os = "linux")]
    fn iosize(&self) -> i64 {
        self.f_frsize as i64
    }
    #[cfg(target_os = "macos")]
    fn iosize(&self) -> i64 {
        self.f_iosize as i64
    }
    // FIXME:
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn iosize(&self) -> i64 {
        0
    }

    // Linux, SunOS, HP-UX, 4.4BSD, FreeBSD have a system call statfs() that returns
    // a struct statfs, containing a fsid_t f_fsid, where fsid_t is defined
    // as struct { int val[2];  }
    //
    // Solaris, Irix and POSIX have a system call statvfs(2) that returns a
    // struct statvfs, containing an  unsigned  long  f_fsid
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn fsid(&self) -> u64 {
        let f_fsid: &[u32; 2] = unsafe { transmute(&self.f_fsid) };
        (f_fsid[0] as u64) << 32 | f_fsid[1] as u64
    }
    // FIXME:
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn fsid(&self) -> u64 {
        0
    }

    #[cfg(target_os = "linux")]
    fn namelen(&self) -> i64 {
        self.f_namelen as i64
    }
    #[cfg(target_os = "macos")]
    fn namelen(&self) -> i64 {
        1024
    }
    // FIXME:
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn namelen(&self) -> u64 {
        0
    }
}

pub fn statfs<P: AsRef<Path>>(path: P) -> Result<Sstatfs, String>
    where Vec<u8>: From<P>
{
    match CString::new(path) {
        Ok(p) => {
            let mut buffer: Sstatfs = unsafe { mem::zeroed() };
            unsafe {
                match statfs_fn(p.as_ptr(), &mut buffer) {
                    0 => Ok(buffer),
                    _ => {
                        let errno = IOError::last_os_error().raw_os_error().unwrap_or(0);
                        Err(CString::from_raw(strerror(errno))
                            .into_string()
                            .unwrap_or("Unknown Error".to_owned()))
                    }
                }
            }
        }
        Err(e) => Err(e.description().to_owned()),
    }
}

pub fn pretty_fstype<'a>(fstype: i64) -> Cow<'a, str> {
    match fstype {
        0x61636673 => "acfs".into(),
        0xADF5 => "adfs".into(),
        0xADFF => "affs".into(),
        0x5346414F => "afs".into(),
        0x09041934 => "anon-inode FS".into(),
        0x61756673 => "aufs".into(),
        0x0187 => "autofs".into(),
        0x42465331 => "befs".into(),
        0x62646576 => "bdevfs".into(),
        0x1BADFACE => "bfs".into(),
        0xCAFE4A11 => "bpf_fs".into(),
        0x42494E4D => "binfmt_misc".into(),
        0x9123683E => "btrfs".into(),
        0x73727279 => "btrfs_test".into(),
        0x00C36400 => "ceph".into(),
        0x0027E0EB => "cgroupfs".into(),
        0xFF534D42 => "cifs".into(),
        0x73757245 => "coda".into(),
        0x012FF7B7 => "coh".into(),
        0x62656570 => "configfs".into(),
        0x28CD3D45 => "cramfs".into(),
        0x453DCD28 => "cramfs-wend".into(),
        0x64626720 => "debugfs".into(),
        0x1373 => "devfs".into(),
        0x1CD1 => "devpts".into(),
        0xF15F => "ecryptfs".into(),
        0xDE5E81E4 => "efivarfs".into(),
        0x00414A53 => "efs".into(),
        0x5DF5 => "exofs".into(),
        0x137D => "ext".into(),
        0xEF53 => "ext2/ext3".into(),
        0xEF51 => "ext2".into(),
        0xF2F52010 => "f2fs".into(),
        0x4006 => "fat".into(),
        0x19830326 => "fhgfs".into(),
        0x65735546 => "fuseblk".into(),
        0x65735543 => "fusectl".into(),
        0x0BAD1DEA => "futexfs".into(),
        0x01161970 => "gfs/gfs2".into(),
        0x47504653 => "gpfs".into(),
        0x4244 => "hfs".into(),
        0x482B => "hfs+".into(),
        0x4858 => "hfsx".into(),
        0x00C0FFEE => "hostfs".into(),
        0xF995E849 => "hpfs".into(),
        0x958458F6 => "hugetlbfs".into(),
        0x11307854 => "inodefs".into(),
        0x013111A8 => "ibrix".into(),
        0x2BAD1DEA => "inotifyfs".into(),
        0x9660 => "isofs".into(),
        0x4004 => "isofs".into(),
        0x4000 => "isofs".into(),
        0x07C0 => "jffs".into(),
        0x72B6 => "jffs2".into(),
        0x3153464A => "jfs".into(),
        0x6B414653 => "k-afs".into(),
        0xC97E8168 => "logfs".into(),
        0x0BD00BD0 => "lustre".into(),
        0x5346314D => "m1fs".into(),
        0x137F => "minix".into(),
        0x138F => "minix (30 char.)".into(),
        0x2468 => "minix v2".into(),
        0x2478 => "minix v2 (30 char.)".into(),
        0x4D5A => "minix3".into(),
        0x19800202 => "mqueue".into(),
        0x4D44 => "msdos".into(),
        0x564C => "novell".into(),
        0x6969 => "nfs".into(),
        0x6E667364 => "nfsd".into(),
        0x3434 => "nilfs".into(),
        0x6E736673 => "nsfs".into(),
        0x5346544E => "ntfs".into(),
        0x9FA1 => "openprom".into(),
        0x7461636F => "ocfs2".into(),
        0x794C7630 => "overlayfs".into(),
        0xAAD7AAEA => "panfs".into(),
        0x50495045 => "pipefs".into(),
        0x7C7C6673 => "prl_fs".into(),
        0x9FA0 => "proc".into(),
        0x6165676C => "pstorefs".into(),
        0x002F => "qnx4".into(),
        0x68191122 => "qnx6".into(),
        0x858458F6 => "ramfs".into(),
        0x52654973 => "reiserfs".into(),
        0x7275 => "romfs".into(),
        0x67596969 => "rpc_pipefs".into(),
        0x73636673 => "securityfs".into(),
        0xF97CFF8C => "selinux".into(),
        0x43415D53 => "smackfs".into(),
        0x517B => "smb".into(),
        0xFE534D42 => "smb2".into(),
        0xBEEFDEAD => "snfs".into(),
        0x534F434B => "sockfs".into(),
        0x73717368 => "squashfs".into(),
        0x62656572 => "sysfs".into(),
        0x012FF7B6 => "sysv2".into(),
        0x012FF7B5 => "sysv4".into(),
        0x01021994 => "tmpfs".into(),
        0x74726163 => "tracefs".into(),
        0x24051905 => "ubifs".into(),
        0x15013346 => "udf".into(),
        0x00011954 => "ufs".into(),
        0x54190100 => "ufs".into(),
        0x9FA2 => "usbdevfs".into(),
        0x01021997 => "v9fs".into(),
        0xBACBACBC => "vmhgfs".into(),
        0xA501FCF5 => "vxfs".into(),
        0x565A4653 => "vzfs".into(),
        0x53464846 => "wslfs".into(),
        0xABBA1974 => "xenfs".into(),
        0x012FF7B4 => "xenix".into(),
        0x58465342 => "xfs".into(),
        0x012FD16D => "xia".into(),
        0x2FC12FC1 => "zfs".into(),
        other => format!("UNKNOWN ({:#x})", other).into(),
    }
}
