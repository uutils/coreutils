//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

//spell-checker: ignore testfs fuser

#![allow(dead_code)]

use crate::common::testfs::fuse::TestFs;
use fuser::BackgroundSession;
use fuser::MountOption::FSName;
use std::ffi::CString;

macro_rules! log_testfs {
    ($($arg:tt)*) => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        let caller_name = &name[..name.len() - 3]
            .split("::").last().expect("Getting function name failed");

        print!("{}: ", caller_name);
        println!($($arg)*);
    }};
}

pub(crate) use log_testfs;

pub fn testfs_mount(mount_point: String) -> std::io::Result<BackgroundSession> {
    log_testfs!("mount_point: {}", mount_point);

    let options = [FSName(String::from("testfs"))];

    let res = fuser::spawn_mount2(TestFs, mount_point, &options);
    log_testfs!("{:?}", res);
    res
}

pub fn testfs_unmount(mount_point: String) {
    log_testfs!("mount_point: {}", mount_point);

    let mount_point_c_string = CString::new(mount_point).expect("CString::new failed");
    unsafe {
        libc::umount(mount_point_c_string.as_ptr());
    }
}
