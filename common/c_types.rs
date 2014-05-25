#![allow(dead_code, non_camel_case_types)]

extern crate libc;

use self::libc::{
    c_char,
    c_int,
    uid_t,
    time_t
};
use self::libc::funcs::posix88::unistd::getgroups;

use std::vec::Vec;

use std::ptr::read;
use std::str::raw::from_c_str;

pub struct c_passwd {
    pub pw_name:    *c_char,    /* user name */
    pub pw_passwd:  *c_char,    /* user name */
    pub pw_uid:     c_int,      /* user uid */
    pub pw_gid:     c_int,      /* user gid */
    pub pw_change:  time_t,
    pub pw_class:   *c_char,
    pub pw_gecos:   *c_char,
    pub pw_dir:     *c_char,
    pub pw_shell:   *c_char,
    pub pw_expire:  time_t
}

#[cfg(target_os = "macos")]
pub struct utsname {
    pub sysname: [c_char, ..256],
    pub nodename: [c_char, ..256],
    pub release: [c_char, ..256],
    pub version: [c_char, ..256],
    pub machine: [c_char, ..256]
}

#[cfg(target_os = "linux")]
pub struct utsname {
    pub sysname: [c_char, ..65],
    pub nodename: [c_char, ..65],
    pub release: [c_char, ..65],
    pub version: [c_char, ..65],
    pub machine: [c_char, ..65],
    pub domainame: [c_char, ..65]
}

pub struct c_group {
    pub gr_name: *c_char /* group name */
}

pub struct c_tm {
    pub tm_sec: c_int,         /* seconds */
    pub tm_min: c_int,         /* minutes */
    pub tm_hour: c_int,        /* hours */
    pub tm_mday: c_int,        /* day of the month */
    pub tm_mon: c_int,         /* month */
    pub tm_year: c_int,        /* year */
    pub tm_wday: c_int,        /* day of the week */
    pub tm_yday: c_int,        /* day in the year */
    pub tm_isdst: c_int       /* daylight saving time */
}

extern {
    pub fn getpwuid(uid: c_int) -> *c_passwd;
    pub fn getpwnam(login: *c_char) -> *c_passwd;
    pub fn getgrouplist(name:   *c_char,
                        basegid: c_int,
                        groups: *c_int,
                        ngroups: *mut c_int) -> c_int;
    pub fn getgrgid(gid: uid_t) -> *c_group;
}

pub fn get_pw_from_args(free: &Vec<String>) -> Option<c_passwd> {
    if free.len() == 1 {
        let username = free.get(0).as_slice();

        // Passed user as id
        if username.chars().all(|c| c.is_digit()) {
            let id = from_str::<i32>(username).unwrap();
            let pw_pointer = unsafe { getpwuid(id) };

            if pw_pointer.is_not_null() {
                Some(unsafe { read(pw_pointer) })
            } else {
                crash!(1, "{:s}: no such user", username);
            }

        // Passed the username as a string
        } else {
            let pw_pointer = unsafe {
                getpwnam(username.as_slice().as_ptr() as *i8)
            };
            if pw_pointer.is_not_null() {
                Some(unsafe { read(pw_pointer) })
            } else {
                crash!(1, "{:s}: no such user", username);
            }
        }
    } else {
        None
    }
}

static NGROUPS: i32 = 20;

pub fn group(possible_pw: Option<c_passwd>, nflag: bool) {
    let mut groups = Vec::with_capacity(NGROUPS as uint);
    let mut ngroups;

    if possible_pw.is_some() {
        ngroups = NGROUPS;
        unsafe {
            getgrouplist(
                possible_pw.unwrap().pw_name,
                possible_pw.unwrap().pw_gid,
                groups.as_ptr(),
                &mut ngroups);
        }
    } else {
        ngroups = unsafe {
            getgroups(NGROUPS, groups.as_mut_ptr() as *mut u32)
        };
    }


    unsafe { groups.set_len(ngroups as uint) };

    for &g in groups.iter() {
        if nflag {
            let group = unsafe { getgrgid(g as u32) };
            if group.is_not_null() {
                let name = unsafe {
                    from_c_str(read(group).gr_name)
                };
                print!("{:s} ", name);
            }
        } else {
            print!("{:d} ", g);
        }
    }

    println!("");
}
