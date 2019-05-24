#![crate_name = "uu_df"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Fangxu Hu <framlog@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate libc;
extern crate clap;
#[macro_use] extern crate uucore;

#[cfg(windows)]
extern crate kernel32;
#[cfg(windows)]
extern crate winapi;

use clap::{Arg, App, ArgMatches};
#[cfg(unix)]
use libc::{uid_t, fsid_t};
#[cfg(target_os = "macos")]
use libc::statfs;
use std::{env, io, path, ptr, mem, slice, time};
use std::ffi::{CStr, CString, OsString, OsStr};
#[cfg(unix)]
use std::os::raw::{c_char, c_int};
use std::fs::{File};
use std::collections::HashSet;
use std::io::{Write, BufReader, BufRead};
use std::cell::Cell;
use std::collections::HashMap;
#[cfg(windows)]
use kernel32::{GetDriveTypeW, FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, QueryDosDeviceW,
    GetVolumeInformationW, GetLastError, GetVolumePathNamesForVolumeNameW, GetDiskFreeSpaceW};
#[cfg(windows)]
use std::os::windows::prelude::*;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Show information about the file system on which each FILE resides,\n\
                      or all file systems by default.";

const EXIT_OK: i32 = 0;
const EXIT_ERR: i32 = 1;

#[cfg(windows)] const MAX_PATH: usize = 266;

#[cfg(target_os = "linux")] static LINUX_MOUNTINFO: &str = "/proc/self/mountinfo";
#[cfg(target_os = "linux")] static LINUX_MTAB: &str = "/etc/mtab";

static OPT_ALL: &str = "all";
static OPT_BLOCKSIZE: &str = "blocksize";
static OPT_DIRECT: &str = "direct";
static OPT_TOTAL: &str = "total";
static OPT_HUMAN_READABLE: &str = "human-readable";
static OPT_HUMAN_READABLE_2: &str = "human-readable-2";
static OPT_INODES: &str = "inodes";
static OPT_KILO: &str = "kilo";
static OPT_LOCAL: &str = "local";
static OPT_NO_SYNC: &str = "no-sync";
static OPT_OUTPUT: &str = "output";
static OPT_PORTABILITY: &str = "portability";
static OPT_SYNC: &str = "sync";
static OPT_TYPE: &str = "type";
static OPT_PRINT_TYPE: &str = "print-type";
static OPT_EXCLUDE_TYPE: &str = "exclude-type";
static OPT_VERSION: &str = "version";

static MOUNT_OPT_BIND: &str = "bind";

/// Store names of file systems as a selector.
/// Note: `exclude` takes priority over `include`.
struct FsSelector {
    include: HashSet<String>,
    exclude: HashSet<String>,
}

struct Options {
    show_local_fs: bool,
    show_all_fs: bool,
    show_listed_fs: bool,
    show_fs_type: bool,
    show_inode_instead: bool,
    print_grand_total: bool,
    block_size: i64,
    human_readable_base: i64,
    fs_selector: FsSelector,
}

#[derive(Debug, Clone)]
struct MountInfo {
    // it stores `volume_name` in windows platform and `dev_id` in unix platform
    dev_id: String, 
    dev_name: String,
    fs_type: String,
    mount_dir: String,
    mount_option: String, // we only care "bind" option
    mount_root: String,
    remote: bool,
    dummy: bool,
}

#[cfg(all(target_os = "freebsd", not(all(target_os = "macos", target_arch = "x86_64"))))]
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
struct statfs {
    f_version: u32,
    f_type: u32,
    f_flags: u64,
    f_bsize: u64,
    f_iosize: u64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: i64,
    f_files: u64,
    f_ffree: i64,
    f_syncwrites: u64,
    f_asyncwrites: u64,
    f_syncreads: u64,
    f_asyncreads: u64,
    f_spare: [u64; 10usize],
    f_namemax: u32,
    f_owner: uid_t,
    f_fsid: fsid_t,
    f_charspare: [c_char; 80usize],
    f_fstypename: [c_char; 16usize],
    f_mntfromname: [c_char; 88usize],
    f_mntonname: [c_char; 88usize],
}

#[derive(Debug, Clone)]
struct FsUsage {
    blocksize: u64,
    blocks: u64,
    bfree: u64,
    bavail: u64,
    bavail_top_bit_set: bool,
    files: u64,
    ffree: u64
}

#[derive(Debug, Clone)]
struct Filesystem {
    mountinfo: MountInfo,
    usage: FsUsage,
}

#[cfg(windows)]
macro_rules! String2LPWSTR {
    ($str: expr) => (OsString::from($str.clone()).as_os_str().encode_wide().chain(Some(0)).collect::<Vec<u16>>().as_ptr())
}

#[cfg(windows)]
#[allow(non_snake_case)]
fn LPWSTR2String(buf: &[u16]) -> String {
    let len = unsafe { libc::wcslen(buf.as_ptr()) };
    OsString::from_wide(&buf[..len as usize]).into_string().unwrap()
}


fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}

#[cfg(any(target_os = "freebsd", target_os = "macos"))]
#[link(name="C")]
extern "C" {
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    #[link_name = "getmntinfo$INODE64"]
    fn getmntinfo(mntbufp: *mut *mut statfs, flags: c_int) -> c_int;

    #[cfg(all(target_os = "freebsd", not(all(target_os = "macos", target_arch = "x86_64"))))]
    fn getmntinfo(mntbufp: *mut *mut statfs, flags: c_int) -> c_int;
}

#[cfg(any(target_os = "freebsd", target_os = "macos"))]
impl From<statfs> for MountInfo {
    fn from(statfs: statfs) -> Self {
        let mut info = MountInfo {
            dev_id: "".to_string(),
            dev_name: unsafe { CStr::from_ptr(&statfs.f_mntfromname[0]).to_string_lossy().into_owned() },
            fs_type: unsafe { CStr::from_ptr(&statfs.f_fstypename[0]).to_string_lossy().into_owned() },
            mount_dir: unsafe { CStr::from_ptr(&statfs.f_mntonname[0]).to_string_lossy().into_owned() },
            mount_root: "".to_string(),
            mount_option: "".to_string(),
            remote: false,
            dummy: false,
        };
        info.set_missing_fields();
        info
    }
}

impl FsSelector {
    fn new() -> FsSelector {
        FsSelector {
            include: HashSet::new(),
            exclude: HashSet::new()
        }
    }

    #[inline(always)]
    fn include(&mut self, fs_type: String) {
        self.include.insert(fs_type);
    }

    #[inline(always)]
    fn exclude(&mut self, fs_type: String) {
        self.exclude.insert(fs_type);
    }

    fn should_select(&self, fs_type: &String) -> bool {
        if self.exclude.contains(fs_type) {
            return false;
        }
        if self.include.len() <= 0 || self.include.contains(fs_type) {
            true
        } else {
            false
        }
    }
}

impl Options {
    fn new() -> Options {
        Options {
            show_local_fs: false,
            show_all_fs: false,
            show_listed_fs: false,
            show_fs_type: false,
            show_inode_instead: false,
            print_grand_total: false,
            block_size: match env::var("BLOCKSIZE") {
                Ok(size) => size.parse().unwrap(),
                Err(_) => 512
            },
            human_readable_base: -1,
            fs_selector: FsSelector::new()
        }
    }
}

impl MountInfo {
    fn set_missing_fields(&mut self) {
        // set dev_id
        let path = CString::new(self.mount_dir.clone()).unwrap();
        unsafe {
            let mut stat = mem::zeroed();
            if libc::stat(path.as_ptr(), &mut stat) == 0 {
                self.dev_id = (stat.st_dev as i32).to_string();
            } else {
                self.dev_id = "".to_string();
            }
        }
        // set MountInfo::dummy
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
                && self.mount_option.find(MOUNT_OPT_BIND).is_none(),
        }
        // set MountInfo::remote
        #[cfg(windows)] {
            self.remote = winapi::winbase::DRIVE_REMOTE == unsafe {
                GetDriveTypeW(String2LPWSTR!(self.mount_root))
            };
        }
        #[cfg(unix)] {
            if self.dev_name.find(":").is_some()
                || (self.dev_name.starts_with("//")
                    && self.fs_type == "smbfs" || self.fs_type == "cifs")
                || self.dev_name == "-hosts" {
                self.remote = true;
            } else {
                self.remote = false;
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn new(file_name: &str, raw: Vec<&str>) -> Option<MountInfo> {
        match file_name {
            "/proc/self/mountinfo" => {
                let mut m = MountInfo {
                    dev_id: "".to_string(),
                    dev_name: raw[8].to_string(),
                    fs_type: raw[7].to_string(),
                    mount_root: raw[3].to_string(),
                    mount_dir: raw[4].to_string(),
                    mount_option: raw[5].to_string(),
                    remote: false,
                    dummy: false,
                };
                m.set_missing_fields();
                Some(m)
            },
            "/etc/mtab" => {
                let mut m = MountInfo {
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
            },
            _ => None
        }
    }
     #[cfg(windows)]
    fn new(mut volume_name: String) -> Option<MountInfo> {
        let mut dev_name_buf = [0u16; MAX_PATH];
        volume_name.pop();
        let dev_name_len = unsafe { QueryDosDeviceW(
            OsString::from(volume_name.clone()).as_os_str().encode_wide().chain(Some(0)).skip(4).collect::<Vec<u16>>().as_ptr(),
            dev_name_buf.as_mut_ptr(), dev_name_buf.len() as winapi::DWORD) };
        volume_name.push('\\');
        let dev_name = LPWSTR2String(&dev_name_buf);

        let mut mount_root_buf = [0u16; MAX_PATH];
        let success = unsafe {
            GetVolumePathNamesForVolumeNameW(
                String2LPWSTR!(volume_name),
                mount_root_buf.as_mut_ptr(),
                mount_root_buf.len() as winapi::DWORD,
                ptr::null_mut()
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
                0 as winapi::DWORD,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                fs_type_buf.as_mut_ptr(),
                fs_type_buf.len() as winapi::DWORD
            )
        };
        let fs_type = if 0 != success { Some(LPWSTR2String(&fs_type_buf)) } else { None };

        let mut mn_info = MountInfo {
            dev_id: volume_name,
            dev_name,
            fs_type: fs_type.unwrap_or("".to_string()),
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

impl FsUsage {
    #[cfg(unix)]
    fn new(statvfs: libc::statvfs) -> FsUsage {
        FsUsage {
            blocksize: if statvfs.f_frsize != 0 { statvfs.f_frsize as u64 } else { statvfs.f_bsize as u64 } ,
            blocks: statvfs.f_blocks as u64,
            bfree: statvfs.f_bfree as u64,
            bavail: statvfs.f_bavail as u64,
            bavail_top_bit_set: ((statvfs.f_bavail as u64) & (1u64.rotate_right(1))) != 0,
            files: statvfs.f_files as u64,
            ffree: statvfs.f_ffree as u64
        }
    }
    // TODO: add windows support
}

impl Filesystem {
    // TODO: resolve uuid in `mountinfo.dev_name` if exists
    fn new(mountinfo: MountInfo) -> Option<Filesystem> {
        let stat_path = if mountinfo.mount_dir.len() > 0 {
            mountinfo.mount_dir.clone()
        } else {
            mountinfo.dev_name.clone()
        };
        #[cfg(unix)]
        unsafe {
            let path = CString::new(stat_path).unwrap();
            let mut statvfs = mem::zeroed();
            if libc::statvfs(path.as_ptr(), &mut statvfs) < 0 {
                None
            } else {
                Some(Filesystem{
                    mountinfo,
                    usage: FsUsage::new(statvfs)
                })
            }
        }
        #[cfg(windows)] {
            // TODO: add windows support
            None
        }
    }
}

/// Read file system list.
fn read_fs_list() -> Vec<MountInfo> {
    #[cfg(target_os = "linux")] {
        let (file_name, fobj) = File::open(LINUX_MOUNTINFO).map(|f| (LINUX_MOUNTINFO, f))
            .or(File::open(LINUX_MTAB).map(|f| (LINUX_MTAB, f)))
            .expect("failed to find mount list files");
        let reader = BufReader::new(fobj);
        return reader.lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| {
                let raw_data = line.split_whitespace().collect::<Vec<&str>>();
                MountInfo::new(file_name, raw_data)
            }).collect::<Vec<_>>();
    }
    #[cfg(any(target_os = "freebsd", target_os = "macos"))] {
        let mut mptr: *mut statfs = ptr::null_mut();
        let len = unsafe { getmntinfo(&mut mptr, 1 as c_int) };
        if len < 0 {
            crash!(EXIT_ERR, "getmntinfo failed");
        }
        let mounts = unsafe { slice::from_raw_parts(mptr, len as usize) };
        return mounts.into_iter().map(|m| MountInfo::from(*m)).collect::<Vec<_>>();
    }
    #[cfg(windows)] {
        let mut volume_name_buf = [0u16; MAX_PATH];
        let find_handle = unsafe { FindFirstVolumeW(volume_name_buf.as_mut_ptr(), volume_name_buf.len() as winapi::DWORD) };
        if winapi::shlobj::INVALID_HANDLE_VALUE == find_handle {
            crash!(EXIT_ERR, "FindFirstVolumeW failed: {}", unsafe { GetLastError() });
        }
        let mut mounts = Vec::<MountInfo>::new();
        loop {
            let volume_name = LPWSTR2String(&volume_name_buf);
            if !volume_name.starts_with("\\\\?\\") || !volume_name.ends_with("\\") {
                show_warning!("A bad path was skipped: {}", volume_name);
                continue;
            }
            if let Some(m) = MountInfo::new(volume_name) {
                mounts.push(m);
            }
            if 0 == unsafe { FindNextVolumeW(find_handle, volume_name_buf.as_mut_ptr(), volume_name_buf.len() as winapi::DWORD) } {
                let err = unsafe { GetLastError() };
                if err != winapi::ERROR_NO_MORE_FILES {
                    crash!(EXIT_ERR, "FindNextVolumeW failed: {}", err);
                }
                break;
            }
        }
        unsafe {
            FindVolumeClose(find_handle);
        }
        return mounts;
    }
    // panic for other os
    unimplemented!();
}

fn filter_mount_list(vmi: Vec<MountInfo>, opt: &Options) -> Vec<MountInfo> {
    vmi.into_iter().filter_map(|mi| {
        if (mi.remote && opt.show_local_fs)
            || (mi.dummy && !opt.show_all_fs && !opt.show_listed_fs)
            || !opt.fs_selector.should_select(&mi.fs_type) {
            None
        } else {
            Some((mi.dev_id.clone(), mi))
        }
    }).fold(HashMap::<String, Cell<MountInfo>>::new(), |mut acc, (id, mi)| {
        if acc.contains_key(&id) {
            let seen = acc.get(&id).unwrap().replace(mi.clone());
            let target_nearer_root = seen.mount_dir.len() > mi.mount_dir.len();
            // With bind mounts, prefer items nearer the root of the source
            let source_below_root = seen.mount_root.len() > 0 && mi.mount_root.len() > 0
                && seen.mount_root.len() < mi.mount_root.len();
            // let "real" devices with '/' in the name win.
            if (!mi.dev_name.starts_with("/") || seen.dev_name.starts_with("/"))
                // let points towards the root of the device win.
                && (!target_nearer_root || source_below_root)
                // let an entry overmounted on a new device win...
                && (seen.dev_name == mi.dev_name
                    /* ... but only when matching an existing mnt point,
                       to avoid problematic replacement when given
                       inaccurate mount lists, seen with some chroot
                       environments for example.  */
                    || seen.mount_dir != mi.mount_dir) {
                acc.get(&id).unwrap().replace(seen);
            }
        } else {
            acc.insert(id, Cell::new(mi));
        }
        acc
    }).into_iter().map(|ent| ent.1.into_inner()).collect::<Vec<_>>()
}

/// Convert `value` to a human readable string based on `base`.
/// e.g. It returns 1G when value is 1 * 1024 * 1024 * 1024 and base is 1024.
/// Note: It returns `value` if `base` isn't positive.
fn human_readable(value: u64, base: i64) -> String {
    #![allow(non_snake_case)]
    if base <= 0 {
        return value.to_string();
    }
    let KB: u64 = base as u64;
    let MB: u64 = KB * base as u64;
    let GB: u64 = MB * base as u64;
    let TB: u64 = GB * base as u64;
    if value >= TB {
        format!("{:.1}T", (value as f64) / (TB as f64))
    } else if value >= GB {
        format!("{:.1}G", (value as f64) / (GB as f64))
    } else if value >= MB {
        format!("{:.1}M", (value as f64) / (MB as f64))
    } else if value >= KB {
        format!("{:.1}K", (value as f64) / (KB as f64))
    } else {
        format!("{}B", value)
    }
}

fn use_size(free_size: u64, total_size: u64) -> String {
    if total_size == 0 {
        return String::from("-");
    }
    return format!("{:.0}%", 100f64 - 100f64 * (free_size as f64 / total_size as f64));
}

pub fn uumain(args: Vec<String>) -> i32 {
    let usage = get_usage();
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(Arg::with_name(OPT_ALL)
            .short("a")
            .long("all")
            .help("include dummy file systems"))
        .arg(Arg::with_name(OPT_BLOCKSIZE)
            .short("B")
            .long("block-size")
            .takes_value(true)
            .help("scale sizes by SIZE before printing them; e.g.\
                      '-BM' prints sizes in units of 1,048,576 bytes"))
        .arg(Arg::with_name(OPT_DIRECT)
            .long("direct")
            .help("show statistics for a file instead of mount point"))
        .arg(Arg::with_name(OPT_TOTAL)
            .long("total")
            .help("produce a grand total"))
        .arg(Arg::with_name(OPT_HUMAN_READABLE)
            .short("h")
            .long("human-readable")
            .conflicts_with(OPT_HUMAN_READABLE_2)
            .help("print sizes in human readable format (e.g., 1K 234M 2G)"))
        .arg(Arg::with_name(OPT_HUMAN_READABLE_2)
            .short("H")
            .long("si")
            .conflicts_with(OPT_HUMAN_READABLE)
            .help("likewise, but use powers of 1000 not 1024"))
        .arg(Arg::with_name(OPT_INODES)
            .short("i")
            .long("inodes")
            .help("list inode information instead of block usage"))
        .arg(Arg::with_name(OPT_KILO)
            .short("k")
            .help("like --block-size=1K"))
        .arg(Arg::with_name(OPT_LOCAL)
            .short("l")
            .long("local")
            .help("limit listing to local file systems"))
        .arg(Arg::with_name(OPT_NO_SYNC)
            .long("no-sync")
            .conflicts_with(OPT_SYNC)
            .help("do not invoke sync before getting usage info (default)"))
        .arg(Arg::with_name(OPT_OUTPUT)
            .long("output")
            .takes_value(true)
            .use_delimiter(true)
            .help("use the output format defined by FIELD_LIST,\
                    or print all fields if FIELD_LIST is omitted."))
        .arg(Arg::with_name(OPT_PORTABILITY)
            .short("P")
            .long("portability")
            .help("use the POSIX output format"))
        .arg(Arg::with_name(OPT_SYNC)
            .long("sync")
            .conflicts_with(OPT_NO_SYNC)
            .help("invoke sync before getting usage info"))
        .arg(Arg::with_name(OPT_TYPE)
            .short("t")
            .long("type")
            .takes_value(true)
            .use_delimiter(true)
            .help("limit listing to file systems of type TYPE"))
        .arg(Arg::with_name(OPT_PRINT_TYPE)
            .short("T")
            .long("print-type")
            .help("print file system type"))
        .arg(Arg::with_name(OPT_EXCLUDE_TYPE)
            .short("x")
            .long("exclude-type")
            .takes_value(true)
            .use_delimiter(true)
            .help("limit listing to file systems not of type TYPE"))
        .arg(Arg::with_name(OPT_VERSION)
            .short("v")
            .long("version")
            .help("output version information and exit"))
        .get_matches_from(&args);

    if matches.is_present(OPT_VERSION) {
        println!("{} {}", executable!(), VERSION);
        return EXIT_OK;
    }

    #[cfg(windows)] {
        if matches.is_present(OPT_INODES) {
            println!("{}: {}", executable!(), "doesn't support -i option");
            return EXIT_OK;
        }
    }

    let mut opt = Options::new();
    if matches.is_present(OPT_LOCAL) {
        opt.show_local_fs = true;
    }
    if matches.is_present(OPT_ALL) {
        opt.show_all_fs = true;
    }
    if matches.is_present(OPT_TOTAL) {
        opt.print_grand_total = true;
    }
    if matches.is_present(OPT_INODES) {
        opt.show_inode_instead = true;
    }
    if matches.is_present(OPT_PRINT_TYPE) {
        opt.show_fs_type = true;
    }
    if matches.is_present(OPT_HUMAN_READABLE) {
        opt.human_readable_base = 1024;
    }
    if matches.is_present(OPT_HUMAN_READABLE_2) {
        opt.human_readable_base = 1000;
    }
    for fs_type in matches.values_of_lossy(OPT_TYPE).unwrap_or(Vec::new()).iter() {
        opt.fs_selector.include(fs_type.to_owned());
    }
    for fs_type in matches.values_of_lossy(OPT_EXCLUDE_TYPE).unwrap_or(Vec::new()).iter() {
        opt.fs_selector.exclude(fs_type.to_owned());
    }

    let fs_list = filter_mount_list(read_fs_list(), &opt).into_iter()
        .filter_map(|mi| Filesystem::new(mi))
        .filter(|fs| fs.usage.blocks != 0 || opt.show_all_fs || opt.show_listed_fs)
        .collect::<Vec<_>>();

    // set headers
    let mut header = vec!["Filesystem"];
    if opt.show_fs_type {
        header.push("Type");
    }
    header.extend_from_slice(&if opt.show_inode_instead {
        ["Inodes", "Iused", "IFree", "IUses%"]
    } else {
        [if opt.human_readable_base == -1 { "1k-blocks" } else { "Size" }, "Used", "Available", "Use%"]
    });
    header.push("Mounted on");

    for (idx, title) in header.iter().enumerate() {
        if idx == 0 || idx == header.len() - 1 {
            print!("{0: <16} ", title);
        } else if opt.show_fs_type && idx == 1 {
            print!("{0: <5} ", title);
        } else if idx == header.len() - 2 {
            print!("{0: >5} ", title);
        } else {
            print!("{0: >12} ", title);
        }
    }
    println!();
    for fs in fs_list.iter() {
        print!("{0: <16} ", fs.mountinfo.dev_name);
        if opt.show_fs_type {
            print!("{0: <5} ", fs.mountinfo.fs_type);
        }
        if opt.show_inode_instead {
            print!("{0: >12} ", human_readable(fs.usage.files, opt.human_readable_base));
            print!("{0: >12} ", human_readable(fs.usage.files - fs.usage.ffree, opt.human_readable_base));
            print!("{0: >12} ", human_readable(fs.usage.ffree, opt.human_readable_base));
            print!("{0: >5} ", format!("{0:.1}%", 100f64 - 100f64 * (fs.usage.ffree as f64 / fs.usage.files as f64)));
        } else {
            let total_size = fs.usage.blocksize * fs.usage.blocks;
            let free_size = fs.usage.blocksize * fs.usage.bfree;
            print!("{0: >12} ", human_readable(total_size, opt.human_readable_base));
            print!("{0: >12} ", human_readable(total_size - free_size, opt.human_readable_base));
            print!("{0: >12} ", human_readable(free_size, opt.human_readable_base));
            print!("{0: >5} ", use_size(free_size, total_size));
        }
        print!("{0: <16}", fs.mountinfo.mount_dir);
        println!();
    }

    EXIT_OK
}
