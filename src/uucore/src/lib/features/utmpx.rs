// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
//! Aims to provide platform-independent methods to obtain login records
//!
//! **ONLY** support linux, macos and freebsd for the time being
//!
//! # Examples:
//!
//! ```
//! use uucore::utmpx::Utmpx;
//! for ut in Utmpx::iter_all_records() {
//!     if ut.is_user_process() {
//!         println!("{}: {}", ut.host(), ut.user())
//!     }
//! }
//! ```
//!
//! Specifying the path to login record:
//!
//! ```
//! use uucore::utmpx::Utmpx;
//! for ut in Utmpx::iter_all_records().read_from("/some/where/else") {
//!     if ut.is_user_process() {
//!         println!("{}: {}", ut.host(), ut.user())
//!     }
//! }
//! ```

pub extern crate time;
use self::time::{Timespec, Tm};

use std::ffi::CString;
use std::io::Error as IOError;
use std::io::Result as IOResult;
use std::ptr;

pub use self::ut::*;
use libc::utmpx;
// pub use libc::getutxid;
// pub use libc::getutxline;
// pub use libc::pututxline;
pub use libc::endutxent;
pub use libc::getutxent;
pub use libc::setutxent;
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use libc::utmpxname;
#[cfg(target_os = "freebsd")]
pub unsafe extern "C" fn utmpxname(_file: *const libc::c_char) -> libc::c_int {
    0
}

// In case the c_char array doesn't end with NULL
macro_rules! chars2string {
    ($arr:expr) => {
        $arr.iter()
            .take_while(|i| **i > 0)
            .map(|&i| i as u8 as char)
            .collect::<String>()
    };
}

#[cfg(target_os = "linux")]
mod ut {
    pub static DEFAULT_FILE: &str = "/var/run/utmp";

    pub use libc::__UT_HOSTSIZE as UT_HOSTSIZE;
    pub use libc::__UT_LINESIZE as UT_LINESIZE;
    pub use libc::__UT_NAMESIZE as UT_NAMESIZE;
    pub const UT_IDSIZE: usize = 4;

    pub use libc::ACCOUNTING;
    pub use libc::BOOT_TIME;
    pub use libc::DEAD_PROCESS;
    pub use libc::EMPTY;
    pub use libc::INIT_PROCESS;
    pub use libc::LOGIN_PROCESS;
    pub use libc::NEW_TIME;
    pub use libc::OLD_TIME;
    pub use libc::RUN_LVL;
    pub use libc::USER_PROCESS;
}

#[cfg(target_os = "macos")]
mod ut {
    pub static DEFAULT_FILE: &str = "/var/run/utmpx";

    pub use libc::_UTX_HOSTSIZE as UT_HOSTSIZE;
    pub use libc::_UTX_IDSIZE as UT_IDSIZE;
    pub use libc::_UTX_LINESIZE as UT_LINESIZE;
    pub use libc::_UTX_USERSIZE as UT_NAMESIZE;

    pub use libc::ACCOUNTING;
    pub use libc::BOOT_TIME;
    pub use libc::DEAD_PROCESS;
    pub use libc::EMPTY;
    pub use libc::INIT_PROCESS;
    pub use libc::LOGIN_PROCESS;
    pub use libc::NEW_TIME;
    pub use libc::OLD_TIME;
    pub use libc::RUN_LVL;
    pub use libc::SHUTDOWN_TIME;
    pub use libc::SIGNATURE;
    pub use libc::USER_PROCESS;
}

#[cfg(target_os = "freebsd")]
mod ut {
    pub static DEFAULT_FILE: &str = "";

    pub const UT_LINESIZE: usize = 16;
    pub const UT_NAMESIZE: usize = 32;
    pub const UT_IDSIZE: usize = 8;
    pub const UT_HOSTSIZE: usize = 128;

    pub use libc::BOOT_TIME;
    pub use libc::DEAD_PROCESS;
    pub use libc::EMPTY;
    pub use libc::INIT_PROCESS;
    pub use libc::LOGIN_PROCESS;
    pub use libc::NEW_TIME;
    pub use libc::OLD_TIME;
    pub use libc::SHUTDOWN_TIME;
    pub use libc::USER_PROCESS;
}

pub struct Utmpx {
    inner: utmpx,
}

impl Utmpx {
    /// A.K.A. ut.ut_type
    pub fn record_type(&self) -> i16 {
        self.inner.ut_type as i16
    }
    /// A.K.A. ut.ut_pid
    pub fn pid(&self) -> i32 {
        self.inner.ut_pid as i32
    }
    /// A.K.A. ut.ut_id
    pub fn terminal_suffix(&self) -> String {
        chars2string!(self.inner.ut_id)
    }
    /// A.K.A. ut.ut_user
    pub fn user(&self) -> String {
        chars2string!(self.inner.ut_user)
    }
    /// A.K.A. ut.ut_host
    pub fn host(&self) -> String {
        chars2string!(self.inner.ut_host)
    }
    /// A.K.A. ut.ut_line
    pub fn tty_device(&self) -> String {
        chars2string!(self.inner.ut_line)
    }
    /// A.K.A. ut.ut_tv
    pub fn login_time(&self) -> Tm {
        time::at(Timespec::new(
            self.inner.ut_tv.tv_sec as i64,
            self.inner.ut_tv.tv_usec as i32,
        ))
    }
    /// A.K.A. ut.ut_exit
    ///
    /// Return (e_termination, e_exit)
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn exit_status(&self) -> (i16, i16) {
        (self.inner.ut_exit.e_termination, self.inner.ut_exit.e_exit)
    }
    /// A.K.A. ut.ut_exit
    ///
    /// Return (0, 0) on Non-Linux platform
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    pub fn exit_status(&self) -> (i16, i16) {
        (0, 0)
    }
    /// Consumes the `Utmpx`, returning the underlying C struct utmpx
    pub fn into_inner(self) -> utmpx {
        self.inner
    }
    pub fn is_user_process(&self) -> bool {
        !self.user().is_empty() && self.record_type() == USER_PROCESS
    }

    /// Canonicalize host name using DNS
    pub fn canon_host(&self) -> IOResult<String> {
        const AI_CANONNAME: libc::c_int = 0x2;
        let host = self.host();
        let host = host.split(':').next().unwrap();
        let hints = libc::addrinfo {
            ai_flags: AI_CANONNAME,
            ai_family: 0,
            ai_socktype: 0,
            ai_protocol: 0,
            ai_addrlen: 0,
            ai_addr: ptr::null_mut(),
            ai_canonname: ptr::null_mut(),
            ai_next: ptr::null_mut(),
        };
        let c_host = CString::new(host).unwrap();
        let mut res = ptr::null_mut();
        let status = unsafe {
            libc::getaddrinfo(
                c_host.as_ptr(),
                ptr::null(),
                &hints as *const _,
                &mut res as *mut _,
            )
        };
        if status == 0 {
            let info: libc::addrinfo = unsafe { ptr::read(res as *const _) };
            // http://lists.gnu.org/archive/html/bug-coreutils/2006-09/msg00300.html
            // says Darwin 7.9.0 getaddrinfo returns 0 but sets
            // res->ai_canonname to NULL.
            let ret = if info.ai_canonname.is_null() {
                Ok(String::from(host))
            } else {
                Ok(unsafe { CString::from_raw(info.ai_canonname).into_string().unwrap() })
            };
            unsafe {
                libc::freeaddrinfo(res);
            }
            ret
        } else {
            Err(IOError::last_os_error())
        }
    }
    pub fn iter_all_records() -> UtmpxIter {
        UtmpxIter
    }
}

/// Iterator of login records
pub struct UtmpxIter;

impl UtmpxIter {
    /// Sets the name of the utmpx-format file for the other utmpx functions to access.
    ///
    /// If not set, default record file will be used(file path depends on the target OS)
    pub fn read_from(self, f: &str) -> Self {
        let res = unsafe {
            let cstr = CString::new(f).unwrap();
            utmpxname(cstr.as_ptr())
        };
        if res != 0 {
            println!("Warning: {}", IOError::last_os_error());
        }
        unsafe {
            setutxent();
        }
        self
    }
}

impl Iterator for UtmpxIter {
    type Item = Utmpx;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let res = getutxent();
            if !res.is_null() {
                Some(Utmpx {
                    inner: ptr::read(res as *const _),
                })
            } else {
                endutxent();
                None
            }
        }
    }
}
