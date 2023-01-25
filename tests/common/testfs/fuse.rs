//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

//spell-checker: ignore bkuptime chgtime crtime ENAMETOOLONG ERANGE fuser getattr getxattr listxattr removexattr setattr setattr setxattr testfs

use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::time::{Duration, SystemTime};

use crate::common::testfs::util::log_testfs;
use fuser::Filesystem;
use fuser::KernelConfig;
use fuser::ReplyAttr;
use fuser::ReplyCreate;
use fuser::ReplyEmpty;
use fuser::ReplyEntry;
use fuser::ReplyOpen;
use fuser::Request;
use fuser::FUSE_ROOT_ID;
use fuser::{FileAttr, ReplyXattr, TimeOrNow};
use fuser::{FileType, ReplyData};
use libc;
use libc::c_int;
use nix::errno::Errno;
use once_cell::sync::Lazy;
use rand::Rng;

pub struct TestFs;

const BLOCK_SIZE: u32 = 512;
const MAX_NAME_LENGTH: usize = 255;

type Inode = u64;

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serializable", derive(Serialize, Deserialize))]
struct InodeAttr {
    file_attr: FileAttr,
    name: String,
}

type InodeMap<T> = Lazy<Mutex<HashMap<Inode, T>>>;
type InodeMapGuard<'a, T> = MutexGuard<'a, HashMap<Inode, T>>;

static INODES: InodeMap<InodeAttr> = Lazy::new(|| Mutex::new(HashMap::new()));
static ENTRIES: InodeMap<Vec<(Inode, FileType, String)>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn inodes<'a>() -> InodeMapGuard<'a, InodeAttr> {
    INODES.lock().expect("Inodes lock failed")
}

fn dir_entries<'a>() -> InodeMapGuard<'a, Vec<(Inode, FileType, String)>> {
    ENTRIES.lock().expect("Entries lock failed")
}

fn has_inode(inode: Inode) -> bool {
    inodes().contains_key(&inode)
}

fn get_inode(inode: &Inode) -> Result<InodeAttr, c_int> {
    match inodes().get(inode) {
        Some(inode_attr) => Ok(inode_attr.clone()),
        None => Err(libc::ENOENT),
    }
}

fn set_inode(inode: Inode, inode_attr: InodeAttr) {
    inodes().insert(inode, inode_attr);
}

fn gen_new_inode() -> Inode {
    let mut rng = rand::thread_rng();
    let mut candidate_inode: Inode = FUSE_ROOT_ID;
    while inodes().contains_key(&candidate_inode) {
        candidate_inode = rng.gen();
    }
    candidate_inode
}

impl TestFs {
    fn find_by_name(
        &self,
        parent: Inode,
        name: String,
    ) -> Result<(Inode, FileType, String), (c_int, String)> {
        if name.len() > MAX_NAME_LENGTH {
            return Err((libc::ENAMETOOLONG, name));
        }
        match get_inode(&parent) {
            Ok(_parent_inode_attr) => {
                let de = dir_entries();
                let parent_entries = de.get(&parent).expect("Parent get failed");
                for (entry_inode, entry_type, entry_name) in parent_entries {
                    if &name == entry_name {
                        return Ok((*entry_inode, *entry_type, name));
                    }
                }
            }
            Err(err) => {
                return Err((err, name));
            }
        }
        Err((libc::ENOENT, name))
    }

    fn new_dir_inode(&self, parent: Inode) -> (Inode, InodeAttr, Vec<(Inode, FileType, String)>) {
        let new_inode = gen_new_inode();
        let new_inode_attr = InodeAttr {
            file_attr: FileAttr {
                ino: new_inode,
                size: 0,
                blocks: 0,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
                kind: FileType::Directory,
                perm: 0o777,
                nlink: 1,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                blksize: BLOCK_SIZE,
                flags: 0,
            },
            name: String::new(),
        };

        let entries = vec![
            (new_inode, FileType::Directory, String::from(".")),
            (parent, FileType::Directory, String::from("..")),
        ];

        (new_inode, new_inode_attr, entries)
    }

    fn refresh_inode_time(&self, inode: Inode) -> Result<(), c_int> {
        match get_inode(&inode) {
            Ok(mut inode_attr) => {
                inode_attr.file_attr.mtime = SystemTime::now();
                inode_attr.file_attr.atime = SystemTime::now();
                inode_attr.file_attr.ctime = SystemTime::now();
                set_inode(inode, inode_attr);

                Ok(())
            }
            Err(err) => Err(err),
        }
    }
}

impl Filesystem for TestFs {
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> Result<(), c_int> {
        log_testfs!("init()");
        if !has_inode(FUSE_ROOT_ID) {
            log_testfs!("init: setting FUSE_ROOT_ID");
            let (new_inode, new_inode_attr, entries) = self.new_dir_inode(FUSE_ROOT_ID);
            set_inode(new_inode, new_inode_attr);
            dir_entries().insert(new_inode, entries);
        }
        Ok(())
    }

    fn lookup(&mut self, _req: &Request<'_>, parent: Inode, name: &OsStr, reply: ReplyEntry) {
        match self.find_by_name(
            parent,
            String::from(name.to_str().expect("&OsStr to &str failed")),
        ) {
            Ok((entry_inode, _entry_type, _entry_name)) => match get_inode(&entry_inode) {
                Ok(entry_inode_attr) => {
                    reply.entry(&Duration::new(0, 0), &entry_inode_attr.file_attr, 0);
                    log_testfs!("success");
                }
                Err(err) => {
                    log_testfs!("err: {}", Errno::from_i32(err));
                }
            },
            Err(err) => {
                reply.error(err.0);
                log_testfs!("err: {}", Errno::from_i32(err.0));
            }
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, inode: Inode, reply: ReplyAttr) {
        log_testfs!("getattr(inode: {})", inode);
        match get_inode(&inode) {
            Ok(inode_attr) => {
                reply.attr(&Duration::new(0, 0), &inode_attr.file_attr);
            }
            Err(err) => reply.error(err),
        };
    }

    fn setattr(
        &mut self,
        _req: &Request<'_>,
        inode: Inode,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        log_testfs!("setattr(inode: {})", inode);
        let attrs = get_inode(&inode).expect("Get inode failed").file_attr;
        reply.attr(&Duration::new(0, 0), &attrs);
    }

    fn open(&mut self, _req: &Request<'_>, inode: Inode, _flags: i32, reply: ReplyOpen) {
        log_testfs!("open(inode: {}, flags: {})", inode, _flags);
        match get_inode(&inode) {
            Ok(_inode_attr) => {
                reply.opened(0, 0);
            }
            Err(err) => {
                reply.error(err);
            }
        }
    }

    fn setxattr(
        &mut self,
        _req: &Request<'_>,
        inode: Inode,
        name: &OsStr,
        _value: &[u8],
        _flags: i32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        log_testfs!(
            "setxattr(inode: {}, name: {:?}, _value: {:?})",
            inode,
            name.to_str(),
            String::from_utf8(Vec::from(_value))
        );
        let inode_attr = get_inode(&inode).expect("Get inode failed");
        log_testfs!("setxattr at: {}", inode_attr.name);
        if inode_attr.name.ends_with("xattr_deny") {
            log_testfs!("setxattr will deny");
            reply.error(libc::EPERM);
        } else if inode_attr.name.ends_with("xattr_allow") {
            log_testfs!("setxattr will allow");
            reply.ok();
        } else {
            reply.error(libc::EINVAL);
        }
    }

    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        inode: Inode,
        name: &OsStr,
        size: u32,
        reply: ReplyXattr,
    ) {
        log_testfs!("getxattr(inode: {}, name: {:?})", inode, name.to_str());
        let data = "dummy_data".as_bytes();
        if size == 0 {
            reply.size(data.len() as u32);
        } else if data.len() <= size as usize {
            reply.data(data);
        } else {
            reply.error(libc::ERANGE);
        }
    }

    fn listxattr(&mut self, _req: &Request<'_>, inode: Inode, size: u32, reply: ReplyXattr) {
        log_testfs!("listxattr(inode: {})", inode);
        let mut bytes = "dummy_key".as_bytes().to_vec();
        bytes.push(0);

        if size == 0 {
            reply.size(bytes.len() as u32);
        } else if bytes.len() <= size as usize {
            reply.data(&bytes);
        } else {
            reply.error(libc::ERANGE);
        }
    }

    fn removexattr(&mut self, _req: &Request<'_>, inode: Inode, name: &OsStr, reply: ReplyEmpty) {
        log_testfs!("(inode: {}, name: {:?})", inode, name.to_str());
        reply.ok();
    }

    fn access(&mut self, _req: &Request<'_>, inode: Inode, mask: i32, reply: ReplyEmpty) {
        log_testfs!("access(inode: {}, mask: {})", inode, mask);
        reply.ok();
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        let name = String::from(name.to_str().expect("OsStr to str failed"));
        log_testfs!("create(parent: {parent}, name: {name})");

        match self.find_by_name(parent, name) {
            Ok((_entry_inode, _entry_type, _entry_name)) => {
                log_testfs!("create err: exists");
                reply.error(libc::EEXIST);
            }
            Err((err, name)) => {
                if err == libc::ENOENT {
                    // good to go
                    let (new_inode, mut new_inode_attr, new_inode_entries) =
                        self.new_dir_inode(parent);
                    new_inode_attr.file_attr.kind = FileType::RegularFile;
                    new_inode_attr.name = name.clone();
                    set_inode(new_inode, new_inode_attr);
                    dir_entries().insert(new_inode, new_inode_entries);

                    let mut parent_inode_entries =
                        dir_entries().remove(&parent).expect("Remove failed");
                    parent_inode_entries.push((new_inode, FileType::RegularFile, name));
                    dir_entries().insert(parent, parent_inode_entries);

                    if let Err(err) = self.refresh_inode_time(parent) {
                        reply.error(err);
                        return;
                    }

                    new_inode_attr = get_inode(&new_inode).expect("Get inode failed");
                    reply.created(&Duration::new(0, 0), &new_inode_attr.file_attr, 0, 0, 0);
                } else {
                    log_testfs!("create err: {}", Errno::from_i32(err));
                    reply.error(err);
                }
            }
        }
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        inode: Inode,
        _fh: u64,
        _offset: i64,
        _size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        log_testfs!("inode({})", inode);
        reply.data("".as_bytes());
    }
}
