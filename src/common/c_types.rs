#![allow(dead_code, non_camel_case_types)]

extern crate libc;

use self::libc::{
    c_char,
    c_int,
    uid_t,
    gid_t,
};
#[cfg(any(target_os = "macos", target_os = "freebsd"))]
use self::libc::time_t;
#[cfg(target_os = "macos")]
use self::libc::int32_t;

use self::libc::funcs::posix88::unistd::getgroups;

use std::vec::Vec;

use std::os;
use std::ptr::{null_mut, read};

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
#[repr(C)]
pub struct c_passwd {
    pub pw_name:    *const c_char,    /* user name */
    pub pw_passwd:  *const c_char,    /* user name */
    pub pw_uid:     uid_t,      /* user uid */
    pub pw_gid:     gid_t,      /* user gid */
    pub pw_change:  time_t,
    pub pw_class:   *const c_char,
    pub pw_gecos:   *const c_char,
    pub pw_dir:     *const c_char,
    pub pw_shell:   *const c_char,
    pub pw_expire:  time_t
}

#[cfg(target_os = "linux")]
#[repr(C)]
pub struct c_passwd {
    pub pw_name:    *const c_char,    /* user name */
    pub pw_passwd:  *const c_char,    /* user name */
    pub pw_uid:     uid_t,      /* user uid */
    pub pw_gid:     gid_t,      /* user gid */
    pub pw_gecos:   *const c_char,
    pub pw_dir:     *const c_char,
    pub pw_shell:   *const c_char,
}

impl Copy for c_passwd {}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
#[repr(C)]
pub struct utsname {
    pub sysname: [c_char, ..256],
    pub nodename: [c_char, ..256],
    pub release: [c_char, ..256],
    pub version: [c_char, ..256],
    pub machine: [c_char, ..256]
}

#[cfg(target_os = "linux")]
#[repr(C)]
pub struct utsname {
    pub sysname: [c_char, ..65],
    pub nodename: [c_char, ..65],
    pub release: [c_char, ..65],
    pub version: [c_char, ..65],
    pub machine: [c_char, ..65],
    pub domainame: [c_char, ..65]
}

impl Copy for utsname {}

#[repr(C)]
pub struct c_group {
    pub gr_name:   *const c_char,  // group name
    pub gr_passwd: *const c_char,  // password
    pub gr_gid:    gid_t,    // group id
    pub gr_mem:    *const *const c_char, // member list
}

impl Copy for c_group {}

#[repr(C)]
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

impl Copy for c_tm {}

extern {
    pub fn getpwuid(uid: uid_t) -> *const c_passwd;
    pub fn getpwnam(login: *const c_char) -> *const c_passwd;
    pub fn getgrgid(gid: gid_t) -> *const c_group;
    pub fn getgrnam(name: *const c_char) -> *const c_group;
    pub fn getgrouplist(name: *const c_char,
                        gid: gid_t,
                        groups: *mut gid_t,
                        ngroups: *mut c_int) -> c_int;
}

#[cfg(target_os = "macos")]
extern {
    pub fn getgroupcount(name: *const c_char, gid: gid_t) -> int32_t;
}

pub fn get_pw_from_args(free: &Vec<String>) -> Option<c_passwd> {
    if free.len() == 1 {
        let username = free[0].as_slice();

        // Passed user as id
        if username.chars().all(|c| c.is_digit(10)) {
            let id = username.parse::<u32>().unwrap();
            let pw_pointer = unsafe { getpwuid(id as uid_t) };

            if pw_pointer.is_not_null() {
                Some(unsafe { read(pw_pointer) })
            } else {
                crash!(1, "{}: no such user", username);
            }

        // Passed the username as a string
        } else {
            let pw_pointer = unsafe {
                getpwnam(username.as_slice().to_c_str().into_inner() as *const libc::c_char)
            };
            if pw_pointer.is_not_null() {
                Some(unsafe { read(pw_pointer) })
            } else {
                crash!(1, "{}: no such user", username);
            }
        }
    } else {
        None
    }
}

pub fn get_group(groupname: &str) -> Option<c_group> {
    let group = if groupname.chars().all(|c| c.is_digit(10)) {
        unsafe { getgrgid(groupname.parse().unwrap()) }
    } else {
        unsafe { getgrnam(groupname.to_c_str().into_inner() as *const c_char) }
    };

    if group.is_not_null() {
        Some(unsafe { read(group) })
    }
    else {
        None
    }
}

pub fn get_group_list(name: *const c_char, gid: gid_t) -> Vec<gid_t> {
    let mut ngroups: c_int = 32;
    let mut groups: Vec<gid_t> = Vec::with_capacity(ngroups as uint);

    if unsafe { get_group_list_internal(name, gid, groups.as_mut_ptr(), &mut ngroups) } == -1 {
        groups.reserve(ngroups as uint);
        unsafe { get_group_list_internal(name, gid, groups.as_mut_ptr(), &mut ngroups); }
    } else {
        groups.truncate(ngroups as uint);
    }
    unsafe { groups.set_len(ngroups as uint); }

    groups
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[inline(always)]
unsafe fn get_group_list_internal(name: *const c_char, gid: gid_t, groups: *mut gid_t, grcnt: *mut c_int) -> c_int {
    getgrouplist(name, gid, groups, grcnt)
}

#[cfg(target_os = "macos")]
unsafe fn get_group_list_internal(name: *const c_char, gid: gid_t, groups: *mut gid_t, grcnt: *mut c_int) -> c_int {
    let ngroups = getgroupcount(name, gid);
    let oldsize = *grcnt;
    *grcnt = ngroups;
    if oldsize >= ngroups {
        getgrouplist(name, gid, groups, grcnt);
        0
    } else {
        -1
    }
}

pub fn get_groups() -> Result<Vec<gid_t>, uint> {
    let ngroups = unsafe { getgroups(0, null_mut()) };
    if ngroups == -1 {
        return Err(os::errno());
    }

    let mut groups = Vec::from_elem(ngroups as uint, 0 as gid_t);
    let ngroups = unsafe { getgroups(ngroups, groups.as_mut_ptr()) };
    if ngroups == -1 {
        Err(os::errno())
    } else {
        groups.truncate(ngroups as uint);
        Ok(groups)
    }
}

pub fn group(possible_pw: Option<c_passwd>, nflag: bool) {
    let groups = match possible_pw {
        Some(pw) => Ok(get_group_list(pw.pw_name, pw.pw_gid)),
        None => get_groups(),
    };

    match groups {
        Err(errno) =>
            crash!(1, "failed to get group list (errno={})", errno),
        Ok(groups) => {
            for &g in groups.iter() {
                if nflag {
                    let group = unsafe { getgrgid(g) };
                    if group.is_not_null() {
                        let name = unsafe {
                            String::from_raw_buf(read(group).gr_name as *const u8)
                        };
                        print!("{} ", name);
                    }
                } else {
                    print!("{} ", g);
                }
            }
            println!("");
        }
    }
}
