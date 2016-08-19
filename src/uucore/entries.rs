// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Get password/group file entry
//!
//! # Examples:
//!
//! ```
//! use uucore::entries::{self, Locate};
//! assert_eq!("root", entries::uid2usr(0).unwrap());
//! assert_eq!(0, entries::usr2uid("root").unwrap());
//! assert!(entries::gid2grp(0).is_ok());
//! assert!(entries::grp2gid("root").is_ok());
//!
//! assert!(entries::Passwd::locate(0).is_ok());
//! assert!(entries::Passwd::locate("0").is_ok());
//! assert!(entries::Passwd::locate("root").is_ok());
//!
//! assert!(entries::Group::locate(0).is_ok());
//! assert!(entries::Group::locate("0").is_ok());
//! assert!(entries::Group::locate("root").is_ok());
//! ```

#[cfg(any(target_os = "freebsd", target_os = "macos"))]
use libc::time_t;
use libc::{uid_t, gid_t, c_char, c_int};
use libc::{passwd, group, getpwnam, getpwuid, getgrnam, getgrgid, getgroups};

use ::std::ptr;
use ::std::io::ErrorKind;
use ::std::io::Error as IOError;
use ::std::io::Result as IOResult;
use ::std::ffi::{CStr, CString};
use ::std::borrow::Cow;

extern "C" {
    fn getgrouplist(name: *const c_char, gid: gid_t, groups: *mut gid_t, ngroups: *mut c_int) -> c_int;
}

pub fn get_groups() -> IOResult<Vec<gid_t>> {
    let ngroups = unsafe { getgroups(0, ptr::null_mut()) };
    if ngroups == -1 {
        return Err(IOError::last_os_error())
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

pub struct Passwd {
    inner: passwd,
}

macro_rules! cstr2cow {
    ($v:expr) => (
        unsafe { CStr::from_ptr($v).to_string_lossy() }
    )
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
    #[cfg(any(target_os = "freebsd", target_os = "macos"))]
    pub fn user_access_class(&self) -> Cow<str> {
        cstr2cow!(self.inner.pw_class)
    }

    /// AKA passwd.pw_change
    #[cfg(any(target_os = "freebsd", target_os = "macos"))]
    pub fn passwd_change_time(&self) -> time_t {
        self.inner.pw_change
    }

    /// AKA passwd.pw_expire
    #[cfg(any(target_os = "freebsd", target_os = "macos"))]
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
    fn locate(key: K) -> IOResult<Self> where Self: ::std::marker::Sized;
}

macro_rules! f {
    ($fnam:ident, $fid:ident, $t:ident, $st:ident) => (
        impl Locate<$t> for $st {
            fn locate(k: $t) -> IOResult<Self> {
                unsafe {
                    let data = $fid(k);
                    if !data.is_null() {
                        Ok($st {
                            inner: ptr::read(data as *const _)
                        })
                    } else {
                        Err(IOError::new(ErrorKind::NotFound, format!("No such id: {}", k)))
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
                            inner: unsafe {ptr::read(data as *const _)}
                        })
                    } else {
                        Err(IOError::new(ErrorKind::NotFound, format!("No such id: {}", id)))
                    }
                } else {
                    unsafe {
                        let data = $fnam(CString::new(k).unwrap().as_ptr());
                        if !data.is_null() {
                            Ok($st {
                                inner: ptr::read(data as *const _)
                            })
                        } else {
                            Err(IOError::new(ErrorKind::NotFound, format!("Not found: {}", k)))
                        }
                    }
                }
            }
        }
    )
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
