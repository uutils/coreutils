// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) Passwd cstr fnam gecos ngroups egid

//! Get password/group file entry
//!
//! # Examples:
//!
//! ```
//! use uucore::entries::{self, Locate};
//!
//! let root_group = if cfg!(any(target_os = "linux", target_os = "android")) {
//!     "root"
//! } else {
//!     "wheel"
//! };
//!
//! assert_eq!("root", entries::uid2usr(0).unwrap());
//! assert_eq!(0, entries::usr2uid("root").unwrap());
//! assert!(entries::gid2grp(0).is_ok());
//! assert!(entries::grp2gid(root_group).is_ok());
//!
//! assert!(entries::Passwd::locate(0).is_ok());
//! assert!(entries::Passwd::locate("0").is_ok());
//! assert!(entries::Passwd::locate("root").is_ok());
//!
//! assert!(entries::Group::locate(0).is_ok());
//! assert!(entries::Group::locate("0").is_ok());
//! assert!(entries::Group::locate(root_group).is_ok());
//! ```

#[cfg(any(target_os = "freebsd", target_vendor = "apple"))]
use libc::time_t;
use libc::{c_char, c_int, gid_t, uid_t};
use libc::{getgrgid, getgrnam, getgroups, getpwnam, getpwuid, group, passwd};

use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::io::Error as IOError;
use std::io::ErrorKind;
use std::io::Result as IOResult;
use std::ptr;

extern "C" {
    fn getgrouplist(
        name: *const c_char,
        gid: gid_t,
        groups: *mut gid_t,
        ngroups: *mut c_int,
    ) -> c_int;
}

pub fn get_groups() -> IOResult<Vec<gid_t>> {
    let ngroups = unsafe { getgroups(0, ptr::null_mut()) };
    if ngroups == -1 {
        return Err(IOError::last_os_error());
    }
    let mut groups = Vec::with_capacity(ngroups as usize);
    let ngroups = unsafe { getgroups(ngroups, groups.as_mut_ptr()) };
    if ngroups == -1 {
        Err(IOError::last_os_error())
    } else {
        unsafe {
            groups.set_len(ngroups as usize);
        }
        Ok(groups)
    }
}

/// The list of group IDs returned from GNU's `groups` and GNU's `id --groups`
/// starts with the effective group ID (egid).
/// This is a wrapper for `get_groups()` to mimic this behavior.
///
/// If `arg_id` is `None` (default), `get_groups_gnu` moves the effective
/// group id (egid) to the first entry in the returned Vector.
/// If `arg_id` is `Some(x)`, `get_groups_gnu` moves the id with value `x`
/// to the first entry in the returned Vector. This might be necessary
/// for `id --groups --real` if `gid` and `egid` are not equal.
///
/// From: https://www.man7.org/linux/man-pages/man3/getgroups.3p.html
/// As implied by the definition of supplementary groups, the
/// effective group ID may appear in the array returned by
/// getgroups() or it may be returned only by getegid().  Duplication
/// may exist, but the application needs to call getegid() to be sure
/// of getting all of the information. Various implementation
/// variations and administrative sequences cause the set of groups
/// appearing in the result of getgroups() to vary in order and as to
/// whether the effective group ID is included, even when the set of
/// groups is the same (in the mathematical sense of ``set''). (The
/// history of a process and its parents could affect the details of
/// the result.)
pub fn get_groups_gnu(arg_id: Option<u32>) -> IOResult<Vec<gid_t>> {
    let mut groups = get_groups()?;
    let egid = arg_id.unwrap_or_else(crate::features::process::getegid);
    if !groups.is_empty() && *groups.first().unwrap() == egid {
        return Ok(groups);
    } else if let Some(index) = groups.iter().position(|&x| x == egid) {
        groups.remove(index);
    }
    groups.insert(0, egid);
    Ok(groups)
}

#[derive(Copy, Clone)]
pub struct Passwd {
    inner: passwd,
}

macro_rules! cstr2cow {
    ($v:expr) => {
        unsafe { CStr::from_ptr($v).to_string_lossy() }
    };
}

impl Passwd {
    /// AKA passwd.pw_name
    pub fn name(&self) -> Cow<str> {
        cstr2cow!(self.inner.pw_name)
    }

    /// AKA passwd.pw_uid
    pub fn uid(&self) -> uid_t {
        self.inner.pw_uid
    }

    /// AKA passwd.pw_gid
    pub fn gid(&self) -> gid_t {
        self.inner.pw_gid
    }

    /// AKA passwd.pw_gecos
    pub fn user_info(&self) -> Cow<str> {
        cstr2cow!(self.inner.pw_gecos)
    }

    /// AKA passwd.pw_shell
    pub fn user_shell(&self) -> Cow<str> {
        cstr2cow!(self.inner.pw_shell)
    }

    /// AKA passwd.pw_dir
    pub fn user_dir(&self) -> Cow<str> {
        cstr2cow!(self.inner.pw_dir)
    }

    /// AKA passwd.pw_passwd
    pub fn user_passwd(&self) -> Cow<str> {
        cstr2cow!(self.inner.pw_passwd)
    }

    /// AKA passwd.pw_class
    #[cfg(any(target_os = "freebsd", target_vendor = "apple"))]
    pub fn user_access_class(&self) -> Cow<str> {
        cstr2cow!(self.inner.pw_class)
    }

    /// AKA passwd.pw_change
    #[cfg(any(target_os = "freebsd", target_vendor = "apple"))]
    pub fn passwd_change_time(&self) -> time_t {
        self.inner.pw_change
    }

    /// AKA passwd.pw_expire
    #[cfg(any(target_os = "freebsd", target_vendor = "apple"))]
    pub fn expiration(&self) -> time_t {
        self.inner.pw_expire
    }

    pub fn as_inner(&self) -> &passwd {
        &self.inner
    }

    pub fn into_inner(self) -> passwd {
        self.inner
    }

    pub fn belongs_to(&self) -> Vec<gid_t> {
        let mut ngroups: c_int = 8;
        let mut groups = Vec::with_capacity(ngroups as usize);
        let gid = self.inner.pw_gid;
        let name = self.inner.pw_name;
        unsafe {
            if getgrouplist(name, gid, groups.as_mut_ptr(), &mut ngroups) == -1 {
                groups.resize(ngroups as usize, 0);
                getgrouplist(name, gid, groups.as_mut_ptr(), &mut ngroups);
            }
            groups.set_len(ngroups as usize);
        }
        groups.truncate(ngroups as usize);
        groups
    }
}

pub struct Group {
    inner: group,
}

impl Group {
    /// AKA group.gr_name
    pub fn name(&self) -> Cow<str> {
        cstr2cow!(self.inner.gr_name)
    }

    /// AKA group.gr_gid
    pub fn gid(&self) -> gid_t {
        self.inner.gr_gid
    }

    pub fn as_inner(&self) -> &group {
        &self.inner
    }

    pub fn into_inner(self) -> group {
        self.inner
    }
}

/// Fetch desired entry.
pub trait Locate<K> {
    fn locate(key: K) -> IOResult<Self>
    where
        Self: ::std::marker::Sized;
}

macro_rules! f {
    ($fnam:ident, $fid:ident, $t:ident, $st:ident) => {
        impl Locate<$t> for $st {
            fn locate(k: $t) -> IOResult<Self> {
                unsafe {
                    let data = $fid(k);
                    if !data.is_null() {
                        Ok($st {
                            inner: ptr::read(data as *const _),
                        })
                    } else {
                        Err(IOError::new(
                            ErrorKind::NotFound,
                            format!("No such id: {}", k),
                        ))
                    }
                }
            }
        }

        impl<'a> Locate<&'a str> for $st {
            fn locate(k: &'a str) -> IOResult<Self> {
                if let Ok(id) = k.parse::<$t>() {
                    let data = unsafe { $fid(id) };
                    if !data.is_null() {
                        Ok($st {
                            inner: unsafe { ptr::read(data as *const _) },
                        })
                    } else {
                        Err(IOError::new(
                            ErrorKind::NotFound,
                            format!("No such id: {}", id),
                        ))
                    }
                } else {
                    unsafe {
                        let data = $fnam(CString::new(k).unwrap().as_ptr());
                        if !data.is_null() {
                            Ok($st {
                                inner: ptr::read(data as *const _),
                            })
                        } else {
                            Err(IOError::new(
                                ErrorKind::NotFound,
                                format!("Not found: {}", k),
                            ))
                        }
                    }
                }
            }
        }
    };
}

f!(getpwnam, getpwuid, uid_t, Passwd);
f!(getgrnam, getgrgid, gid_t, Group);

#[inline]
pub fn uid2usr(id: uid_t) -> IOResult<String> {
    Passwd::locate(id).map(|p| p.name().into_owned())
}

#[inline]
pub fn gid2grp(id: gid_t) -> IOResult<String> {
    Group::locate(id).map(|p| p.name().into_owned())
}

#[inline]
pub fn usr2uid(name: &str) -> IOResult<uid_t> {
    Passwd::locate(name).map(|p| p.uid())
}

#[inline]
pub fn grp2gid(name: &str) -> IOResult<gid_t> {
    Group::locate(name).map(|p| p.gid())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_entries_get_groups_gnu() {
        if let Ok(mut groups) = get_groups() {
            if let Some(last) = groups.pop() {
                groups.insert(0, last);
                assert_eq!(get_groups_gnu(Some(last)).unwrap(), groups);
            }
        }
    }
}
