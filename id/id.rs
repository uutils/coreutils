#[crate_id(name="id", version="1.0.0", author="Alan Andrade")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Boden Garman <bpgarman@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 * XXX: Add synced with file [here]
 */

use std::{libc,ptr,vec};
use std::str::raw;

// XXX: Sync with whoami.rs

struct c_passwd {
    pw_name: *libc::c_char,  /* user name */
    pw_uid:   libc::c_int,   /* user uid */
    pw_gid:   libc::c_int    /* user gid */
}

struct c_group {
    gr_name: *libc::c_char /* group name */
}

extern {
    // Effective user id
    fn geteuid () -> libc::c_int;
    // Real user id
    fn getuid () -> libc::c_int;

    // Effective group id
    fn getegid () -> libc::c_int;
    // Real group id
    fn getgid () -> libc::c_int;

    fn getpwuid(uid: libc::c_int) -> *c_passwd;
    fn getgrgid(gid: libc::c_int) -> *c_group;

    //fn getgrent() -> *c_group;
    //fn endgrent();
}

fn main () {
    let euid = unsafe { geteuid () };
    let ruid = unsafe { getuid () };
    let egid = unsafe { getegid () };
    let rgid = unsafe { getgid () };

    print_full_info(ruid, rgid, euid, egid);
}

fn print_full_info (ruid: libc::c_int,
                    rgid: libc::c_int,
                    euid: libc::c_int,
                    egid: libc::c_int) {

    print!("uid={:d}", ruid);
    // XXX: Read from pointer straight away ?
    //
    // How'd be NULL check ?
    let pwd = unsafe { ptr::read_ptr(getpwuid(ruid)) };
    //if pwd {
        let username = unsafe {
            raw::from_c_str(pwd.pw_name)
        };

        print!("({:s})", username);
    //}


    print!(" gid={:d}", rgid);
    let grp = unsafe { getgrgid(rgid) };
    if ptr::is_not_null(grp) {
        unsafe {
            print!("({:s})", raw::from_c_str(ptr::read_ptr(grp).gr_name));
        }
    }


    if euid != ruid {
        print!(" euid={:d}", euid);
        let pwd = unsafe { getpwuid(euid) };
        if ptr::is_not_null(pwd) {
            let username = unsafe {
                raw::from_c_str(ptr::read_ptr(pwd).pw_name)
            };
            print!("({:s})", username);
        }
    }


    if egid != rgid {
        print!(" egid={:d}", egid);
        let grp = unsafe { getgrgid(rgid) };
        if ptr::is_not_null(grp) {
            let groupname = unsafe {
                raw::from_c_str(ptr::read_ptr(grp).gr_name)
            };
            print!("({:s})", groupname)
        }
    }

    let mut groups = std::vec::with_capacity(69);
    unsafe { groups.set_len(69); }
    let n_groups = unsafe { libc::getgroups(13, groups.as_mut_ptr()) };

    if n_groups < 0 {
        println!("{:?}", std::os::errno());
    }

    if n_groups > 0 {
        print!(" groups=");

        for i in range(0, n_groups) {
            if i > 0 {
                print!(",")
            }
            print!("{:u}", groups[i]);
            let group = unsafe { getgrgid(groups[i] as i32) };
            if ptr::is_not_null(group) {
                let name = unsafe {
                    raw::from_c_str(ptr::read_ptr(group).gr_name)
                };
                print!("({:s})", name);
            }
        }

        println!("");
    }

}
