extern crate libc;
extern crate time;

use self::time::Timespec;
pub use self::libc::{S_IFMT, S_IFDIR, S_IFCHR, S_IFBLK, S_IFREG, S_IFIFO, S_IFLNK, S_IFSOCK,
                     S_ISUID, S_ISGID, S_ISVTX, S_IRUSR, S_IWUSR, S_IXUSR, S_IRGRP, S_IWGRP,
                     S_IXGRP, S_IROTH, S_IWOTH, S_IXOTH};

#[macro_export]
macro_rules! has {
    ($mode:expr, $perm:expr) => (
        ($mode & $perm != 0)
    )
}

pub fn pretty_time(sec: i64, nsec: i64) -> String {
    let tm = time::at(Timespec::new(sec, nsec as i32));
    time::strftime("%Y-%m-%d %H:%M:%S.%f %z", &tm).unwrap()
}

pub fn pretty_filetype<'a>(mode: u32, size: u64) -> &'a str {
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

pub fn pretty_access(mode: u32) -> String {
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
    result.push(if has!(mode, S_ISUID as u32) {
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
    result.push(if has!(mode, S_ISGID as u32) {
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
    result.push(if has!(mode, S_ISVTX as u32) {
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

pub struct Statfs {
    pub f_type: i64,
    pub f_bsize: i64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_namelen: i64,
    pub f_frsize: i64,
    pub f_fsid: u64,
}

pub fn statfs<P: AsRef<Path>>(path: P) -> Result<Statfs, String>
    where Vec<u8>: From<P>
{
    use std::error::Error;
    match CString::new(path) {
        Ok(p) => {
            let mut buffer = unsafe { mem::zeroed() };
            unsafe {
                match self::libc::statfs(p.as_ptr(), &mut buffer) {
                    0 => {
                        let fsid: u64;
                        if cfg!(unix) {
                            // Linux, SunOS, HP-UX, 4.4BSD, FreeBSD have a system call statfs() that returns
                            // a struct statfs, containing a fsid_t f_fsid, where fsid_t is defined
                            // as struct { int val[2];  }
                            let f_fsid: &[u32; 2] = transmute(&buffer.f_fsid);
                            fsid = (f_fsid[0] as u64) << 32_u64 | f_fsid[1] as u64;
                        } else {
                            // Solaris, Irix and POSIX have a system call statvfs(2) that returns a
                            // struct statvfs, containing an  unsigned  long  f_fsid
                            fsid = 0;
                        }
                        Ok(Statfs {
                            f_type: buffer.f_type as i64,
                            f_bsize: buffer.f_bsize as i64,
                            f_blocks: buffer.f_blocks as u64,
                            f_bfree: buffer.f_bfree as u64,
                            f_bavail: buffer.f_bavail as u64,
                            f_files: buffer.f_files as u64,
                            f_ffree: buffer.f_ffree as u64,
                            f_fsid: fsid,
                            f_namelen: buffer.f_namelen as i64,
                            f_frsize: buffer.f_frsize as i64,
                        })
                    }
                    // TODO: Return explicit error message
                    _ => Err("Unknown error".to_owned()),
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

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_access() {
        assert_eq!("drwxr-xr-x", pretty_access(S_IFDIR | 0o755));
        assert_eq!("-rw-r--r--", pretty_access(S_IFREG | 0o644));
        assert_eq!("srw-r-----", pretty_access(S_IFSOCK | 0o640));
        assert_eq!("lrw-r-xr-x", pretty_access(S_IFLNK | 0o655));
        assert_eq!("?rw-r-xr-x", pretty_access(0o655));

        assert_eq!("brwSr-xr-x",
                   pretty_access(S_IFBLK | S_ISUID as u32 | 0o655));
        assert_eq!("brwsr-xr-x",
                   pretty_access(S_IFBLK | S_ISUID as u32 | 0o755));

        assert_eq!("prw---sr--",
                   pretty_access(S_IFIFO | S_ISGID as u32 | 0o614));
        assert_eq!("prw---Sr--",
                   pretty_access(S_IFIFO | S_ISGID as u32 | 0o604));

        assert_eq!("c---r-xr-t",
                   pretty_access(S_IFCHR | S_ISVTX as u32 | 0o055));
        assert_eq!("c---r-xr-T",
                   pretty_access(S_IFCHR | S_ISVTX as u32 | 0o054));
    }

    #[test]
    fn test_file_type() {
        assert_eq!("block special file", pretty_filetype(S_IFBLK, 0));
        assert_eq!("character special file", pretty_filetype(S_IFCHR, 0));
        assert_eq!("regular file", pretty_filetype(S_IFREG, 1));
        assert_eq!("regular empty file", pretty_filetype(S_IFREG, 0));
        assert_eq!("weird file", pretty_filetype(0, 0));
    }

    #[test]
    fn test_fs_type() {
        assert_eq!("ext2/ext3", pretty_fstype(0xEF53));
        assert_eq!("tmpfs", pretty_fstype(0x01021994));
        assert_eq!("nfs", pretty_fstype(0x6969));
        assert_eq!("btrfs", pretty_fstype(0x9123683e));
        assert_eq!("xfs", pretty_fstype(0x58465342));
        assert_eq!("zfs", pretty_fstype(0x2FC12FC1));
        assert_eq!("ntfs", pretty_fstype(0x5346544e));
        assert_eq!("fat", pretty_fstype(0x4006));
        assert_eq!("UNKNOWN (0x1234)", pretty_fstype(0x1234));
    }
}
