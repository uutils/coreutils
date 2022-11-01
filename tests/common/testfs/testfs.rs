//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use std::cmp::min;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::time::{Duration, SystemTime};

use fuser::{FileAttr, ReplyBmap, ReplyDirectoryPlus, ReplyIoctl, ReplyLock, ReplyLseek, ReplyStatfs, ReplyXattr, TimeOrNow};
use fuser::FileType;
use fuser::Filesystem;
use fuser::KernelConfig;
use fuser::ReplyAttr;
use fuser::ReplyCreate;
use fuser::ReplyData;
use fuser::ReplyDirectory;
use fuser::ReplyEmpty;
use fuser::ReplyEntry;
use fuser::ReplyOpen;
use fuser::ReplyWrite;
use fuser::Request;
use fuser::FUSE_ROOT_ID;
use libc;
use libc::{c_int};
use nix::errno::Errno;
use once_cell::sync::Lazy;
use rand::Rng;

pub struct TestFs;

const BLOCK_SIZE: u32 = 512;
const MAX_NAME_LENGTH: usize = 255;
const MAX_FILE_SIZE: usize = 512;

type Inode = u64;

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serializable", derive(Serialize, Deserialize))]
struct InodeAttr {
    file_attr: FileAttr,
    name: String,
}

static INODES: Lazy<Mutex<HashMap<Inode, InodeAttr>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static ENTRIES: Lazy<Mutex<HashMap<Inode, Vec<(Inode, FileType, String)>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static STORE: Lazy<Mutex<HashMap<Inode, [u8; MAX_FILE_SIZE]>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn inodes<'a>() -> MutexGuard<'a, HashMap<Inode, InodeAttr>> {
    INODES.lock().expect("Inodes lock failed")
}

fn dir_entries<'a>() -> MutexGuard<'a, HashMap<Inode, Vec<(Inode, FileType, String)>>> {
    ENTRIES.lock().expect("Entries lock failed")
}

fn store<'a>() -> MutexGuard<'a, HashMap<Inode, [u8; MAX_FILE_SIZE]>> {
    STORE.lock().expect("Store lock failed")
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
    return candidate_inode;
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
        return Err((libc::ENOENT, name));
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

        return (new_inode, new_inode_attr, entries);
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
            Err(err) => {
                return Err(err);
            }
        }
    }
}

macro_rules! caller_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }}
}

impl Filesystem for TestFs {
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> Result<(), c_int> {
        println!("init()");
        if !has_inode(FUSE_ROOT_ID) {
            println!("init: setting FUSE_ROOT_ID");
            let (new_inode, new_inode_attr, entries) = self.new_dir_inode(FUSE_ROOT_ID);
            set_inode(new_inode, new_inode_attr);
            dir_entries().insert(new_inode, entries);
        }
        Ok(())
    }

    fn destroy(&mut self) {
        println!("{}", caller_name!());
    }

    fn lookup(&mut self, _req: &Request<'_>, parent: Inode, name: &OsStr, reply: ReplyEntry) {
        match self.find_by_name(
            parent,
            String::from(name.to_str().expect("&OsStr to &str failed")),
        ) {
            Ok((entry_inode, _entry_type, _entry_name)) => match get_inode(&entry_inode) {
                Ok(entry_inode_attr) => {
                    reply.entry(&Duration::new(0, 0), &entry_inode_attr.file_attr, 0);
                    println!("lookup succ");
                }
                Err(err) => {
                    println!("lookup err: {}", Errno::from_i32(err));
                }
            },
            Err(err) => {
                reply.error(err.0);
                println!("lookup err: {}", Errno::from_i32(err.0));
            }
        }
    }

    fn forget(&mut self, _req: &Request<'_>, _inode: Inode, _nlookup: u64) {
        println!("{}", caller_name!());
    }

    fn getattr(&mut self, _req: &Request<'_>, inode: Inode, reply: ReplyAttr) {
        println!("getattr(inode: {})", inode);
        match get_inode(&inode) {
            Ok(inode_attr) => {
                reply.attr(&Duration::new(0, 0), &inode_attr.file_attr);
            }
            Err(err) => reply.error(err),
        };
    }

    fn setattr(&mut self, _req: &Request<'_>, inode: Inode, mode: Option<u32>, uid: Option<u32>, gid: Option<u32>, size: Option<u64>, _atime: Option<TimeOrNow>, _mtime: Option<TimeOrNow>, _ctime: Option<SystemTime>, fh: Option<u64>, _crtime: Option<SystemTime>, _chgtime: Option<SystemTime>, _bkuptime: Option<SystemTime>, flags: Option<u32>, reply: ReplyAttr) {
        println!("setattr(inode: {})", inode);
        let attrs = get_inode(&inode).expect("Get inode failed").file_attr;
        reply.attr(&Duration::new(0, 0), &attrs.into());
    }

    fn readlink(&mut self, _req: &Request<'_>, inode: Inode, reply: ReplyData) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn mknod(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, mode: u32, umask: u32, rdev: u32, reply: ReplyEntry) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: Inode,
        name: &OsStr,
        mode: u32,
        umask: u32,
        reply: ReplyEntry,
    ) {
        let name: String = String::from(name.to_str().expect("OsStr to str failed"));
        println!("mkdir(parent: {parent}, name: {name}, mode: {mode}, umask: {umask})");
        if name.len() > MAX_NAME_LENGTH {
            reply.error(libc::ENAMETOOLONG);
            println!("mkdir err: name too long");
            return;
        }
        match get_inode(&parent) {
            Ok(_parent_inode_attr) => {
                let mut parent_entries = dir_entries().remove(&parent).expect("Remove failed");
                for (_, _, entry_name) in &parent_entries {
                    if &name == entry_name {
                        reply.error(libc::EEXIST);
                        println!("mkdir err: name taken");
                        return;
                    }
                }

                let (new_inode, new_inode_attr, new_inode_entries) = self.new_dir_inode(parent);
                set_inode(new_inode, new_inode_attr);
                dir_entries().insert(new_inode, new_inode_entries);

                parent_entries.push((new_inode, FileType::Directory, String::from(name)));
                dir_entries().insert(parent, parent_entries);

                reply.entry(
                    &Duration::new(0, 0),
                    &get_inode(&new_inode).expect("Get inode failed").file_attr,
                    0,
                );
            }
            Err(err) => {
                reply.error(err);
                println!("mkdir err: {}", (Errno::from_i32(err)));
            }
        }
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name: String = String::from(name.to_str().expect("OsStr to str failed"));
        println!("rmdir(parent: {parent}, name: {name})");
        if name.len() > MAX_NAME_LENGTH {
            reply.error(libc::ENAMETOOLONG);
            println!("rmdir err: name too long");
            return;
        }
        match get_inode(&parent) {
            Ok(_parent_inode_attr) => {
                let mut parent_entries = dir_entries().remove(&parent).expect("Remove failed");
                for (i, (_entry_inode, _entry_file_type, entry_name)) in (&parent_entries).into_iter().enumerate() {
                    if &name == entry_name {
                        let (removed_entry_inode, _, _) = parent_entries.remove(i);
                        println!("rmdir succ: removed inode {}", removed_entry_inode);
                        dir_entries().insert(parent, parent_entries);
                        reply.ok();
                        return;
                    }
                }
                reply.error(libc::ENOENT);
            }
            Err(err) => {
                reply.error(err);
                println!("rmdir err: {}", (Errno::from_i32(err)));
            }
        }
    }

    fn symlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, link: &Path, reply: ReplyEntry) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn rename(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, newparent: u64, newname: &OsStr, flags: u32, reply: ReplyEmpty) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn link(&mut self, _req: &Request<'_>, inode: Inode, newparent: u64, newname: &OsStr, reply: ReplyEntry) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn open(&mut self, _req: &Request<'_>, inode: Inode, _flags: i32, reply: ReplyOpen) {
        println!("open(inode: {}, flags: {})", inode, _flags);
        match get_inode(&inode) {
            Ok(_inode_attr) => {
                reply.opened(0, 0);
                return;
            }
            Err(err) => {
                reply.error(err);
                return;
            }
        }
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        inode: Inode,
        fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        println!(
            "read(inode: {}, fh: {}, offset: {}, size: {}, ...)",
            inode, fh, offset, size
        );
        if offset < 0 {
            reply.error(libc::EINVAL);
            return;
        }
        let offset = offset as usize;
        if !store().contains_key(&inode) {
            reply.error(libc::ENOENT);
            return;
        }

        match get_inode(&inode) {
            Ok(inode_attr) => {
                let file_size = inode_attr.file_attr.size;
                let read_size = min(file_size as usize - offset, size as usize);
                let mut buffer: Vec<u8> = vec![0; read_size as usize];
                let s = store();
                let store = s.get(&inode).expect("Get from store failed");
                for i in offset..read_size {
                    buffer[i - offset] = store[i];
                }
                reply.data(&buffer);
                return;
            }
            Err(err) => {
                reply.error(err);
                return;
            }
        }
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        inode: Inode,
        fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        println!(
            "write(inode: {}, fh: {}, offset: {}, ...)",
            inode, fh, offset
        );

        if offset < 0 {
            reply.error(libc::EINVAL);
            return;
        }
        let offset = offset as usize;
        if offset + data.len() > MAX_FILE_SIZE {
            reply.error(libc::E2BIG);
            return;
        }

        if !store().contains_key(&inode) {
            store().insert(inode, [0; MAX_FILE_SIZE]);
        }

        let mut inode_store = store().remove(&inode).expect("Remove from store failed");
        for i in offset..min(MAX_FILE_SIZE, offset + data.len()) {
            inode_store[i] = data[i - offset];
        }
        store().insert(inode, inode_store);
        let new_size = offset + data.len();
        match get_inode(&inode) {
            Ok(mut inode_attr) => {
                inode_attr.file_attr.size = new_size as u64;
                set_inode(inode, inode_attr);
                if let Err(err) = self.refresh_inode_time(inode) {
                    reply.error(err);
                    return;
                }
                reply.written(data.len() as u32);
                return;
            }
            Err(err) => {
                reply.error(err);
                return;
            }
        }
    }

    fn flush(&mut self, _req: &Request<'_>, inode: Inode, fh: u64, lock_owner: u64, reply: ReplyEmpty) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn release(&mut self, _req: &Request<'_>, _inode: Inode, _fh: u64, _flags: i32, _lock_owner: Option<u64>, _flush: bool, reply: ReplyEmpty) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn fsync(&mut self, _req: &Request<'_>, inode: Inode, fh: u64, datasync: bool, reply: ReplyEmpty) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn opendir(&mut self, _req: &Request<'_>, _inode: Inode, _flags: i32, reply: ReplyOpen) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        inode: Inode,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        println!("readdir(ino: {inode}, fh: {_fh}, offset: {offset})");

        match get_inode(&inode) {
            Ok(inode_attr) => {
                let entries = dir_entries().remove(&inode_attr.file_attr.ino).expect("Remove failed");
                for (i, (entry_inode, entry_file_type, entry_name)) in (&entries).into_iter().enumerate().skip(offset as usize) {
                    if reply.add(*entry_inode, (i + 1) as i64, *entry_file_type, entry_name) {
                        break;
                    }
                }
                dir_entries().insert(inode_attr.file_attr.ino, entries);
                reply.ok();
            }
            Err(err) => {
                reply.error(err);
                println!("readdir err: {}", (Errno::from_i32(err)));
            }
        }
    }

    fn readdirplus(&mut self, _req: &Request<'_>, inode: Inode, fh: u64, offset: i64, reply: ReplyDirectoryPlus) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn releasedir(&mut self, _req: &Request<'_>, _inode: Inode, _fh: u64, _flags: i32, reply: ReplyEmpty) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn fsyncdir(&mut self, _req: &Request<'_>, inode: Inode, fh: u64, datasync: bool, reply: ReplyEmpty) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn statfs(&mut self, _req: &Request<'_>, _inode: Inode, reply: ReplyStatfs) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
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
        println!(
            "setxattr(inode: {}, name: {:?}, _value: {:?})",
            inode,
            name.to_str(),
            String::from_utf8(Vec::from(_value))
        );
        let inode_attr = get_inode(&inode).expect("Get inode failed");
        println!("setxattr at: {}", inode_attr.name);
        if inode_attr.name.ends_with("xattr_deny") {
            println!("setxattr will deny");
            reply.error(libc::EPERM);
        } else if inode_attr.name.ends_with("xattr_allow") {
            println!("setxattr will allow");
            reply.ok()
        } else {
            reply.error(libc::EINVAL)
        }
    }

    fn getxattr(&mut self, _req: &Request<'_>, inode: Inode, name: &OsStr, size: u32, reply: ReplyXattr) {
        println!(
            "getxattr(inode: {}, name: {:?})",
            inode,
            name.to_str()
        );
        let data  = "dummy_data".as_bytes();
        if size == 0 {
            reply.size(data.len() as u32);
        } else if data.len() <= size as usize {
            reply.data(data);
        } else {
            reply.error(libc::ERANGE);
        }
    }

    fn listxattr(&mut self, _req: &Request<'_>, inode: Inode, size: u32, reply: ReplyXattr) {
        println!(
            "listxattr(inode: {})",
            inode
        );
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
        println!("{}(inode: {}, name: {:?})", caller_name!(), inode, name.to_str());
        reply.ok();
    }

    fn access(&mut self, _req: &Request<'_>, inode: Inode, mask: i32, reply: ReplyEmpty) {
        println!("access(inode: {}, mask: {})", inode, mask);
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
        println!("create(parent: {parent}, name: {name})");

        match self.find_by_name(parent, name) {
            Ok((_entry_inode, _entry_type, _entry_name)) => {
                println!("create err: exists");
                reply.error(libc::EEXIST);
            }
            Err((err, name)) => {
                if err == libc::ENOENT {
                    // good to go
                    let (new_inode, mut new_inode_attr, new_inode_entries) = self.new_dir_inode(parent);
                    new_inode_attr.file_attr.kind = FileType::RegularFile;
                    new_inode_attr.name = name.clone();
                    set_inode(new_inode, new_inode_attr);
                    dir_entries().insert(new_inode, new_inode_entries);

                    let mut parent_inode_entries = dir_entries().remove(&parent).expect("Remove failed");
                    parent_inode_entries.push((new_inode, FileType::RegularFile, name));
                    dir_entries().insert(parent, parent_inode_entries);

                    if let Err(err) = self.refresh_inode_time(parent) {
                        reply.error(err);
                        return;
                    }

                    new_inode_attr = get_inode(&new_inode).expect("Get inode failed");
                    reply.created(&Duration::new(0, 0), &new_inode_attr.file_attr, 0, 0, 0);
                } else {
                    println!("create err: {}", Errno::from_i32(err));
                    reply.error(err);
                }
            }
        }
    }

    fn getlk(&mut self, _req: &Request<'_>, inode: Inode, fh: u64, lock_owner: u64, start: u64, end: u64, typ: i32, pid: u32, reply: ReplyLock) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn setlk(&mut self, _req: &Request<'_>, inode: Inode, fh: u64, lock_owner: u64, start: u64, end: u64, typ: i32, pid: u32, sleep: bool, reply: ReplyEmpty) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn bmap(&mut self, _req: &Request<'_>, inode: Inode, blocksize: u32, idx: u64, reply: ReplyBmap) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn ioctl(&mut self, _req: &Request<'_>, inode: Inode, fh: u64, flags: u32, cmd: u32, in_data: &[u8], out_size: u32, reply: ReplyIoctl) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn fallocate(&mut self, _req: &Request<'_>, inode: Inode, fh: u64, offset: i64, length: i64, mode: i32, reply: ReplyEmpty) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn lseek(&mut self, _req: &Request<'_>, inode: Inode, fh: u64, offset: i64, whence: i32, reply: ReplyLseek) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }

    fn copy_file_range(&mut self, _req: &Request<'_>, ino_in: u64, fh_in: u64, offset_in: i64, ino_out: u64, fh_out: u64, offset_out: i64, len: u64, flags: u32, reply: ReplyWrite) {
        println!("{}", caller_name!());
        reply.error(libc::ENOENT);
    }
}
