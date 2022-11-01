//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use crate::common::testfs::fuse::TestFs;
use fuser::BackgroundSession;
use fuser::MountOption::FSName;
use std::ffi::CString;

pub fn testfs_mount(mount_point: String) -> std::io::Result<BackgroundSession> {
    println!("mount_point: {}", mount_point);

    let options = [FSName(String::from("testfs"))];

    let res = fuser::spawn_mount2(TestFs, mount_point, &options);
    println!("{:?}", res);
    res
}

pub fn testfs_unmount(mount_point: String) {
    let mount_point_c_string = CString::new(mount_point).expect("CString::new failed");
    unsafe {
        libc::umount(mount_point_c_string.as_ptr());
    }
}
