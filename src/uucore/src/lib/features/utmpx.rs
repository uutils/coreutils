// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore logind

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
//! for ut in Utmpx::iter_all_records_from("/some/where/else") {
//!     if ut.is_user_process() {
//!         println!("{}: {}", ut.host(), ut.user())
//!     }
//! }
//! ```

pub extern crate time;

use std::ffi::CString;
use std::io::Result as IOResult;
use std::marker::PhantomData;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::sync::{Mutex, MutexGuard};

#[cfg(feature = "feat_systemd_logind")]
use crate::features::systemd_logind;

pub use self::ut::*;

// See the FAQ at https://wiki.musl-libc.org/faq#Q:-Why-is-the-utmp/wtmp-functionality-only-implemented-as-stubs?
// Musl implements only stubs for the utmp functions, and the libc crate issues a deprecation warning about this.
// However, calling these stubs is the correct approach to maintain consistent behavior with GNU coreutils.
#[cfg_attr(target_env = "musl", allow(deprecated))]
pub use libc::endutxent;
#[cfg_attr(target_env = "musl", allow(deprecated))]
pub use libc::getutxent;
#[cfg_attr(target_env = "musl", allow(deprecated))]
pub use libc::setutxent;
use libc::utmpx;
#[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "netbsd"))]
#[cfg_attr(target_env = "musl", allow(deprecated))]
pub use libc::utmpxname;

/// # Safety
/// Just fixed the clippy warning. Please add description here.
#[cfg(target_os = "freebsd")]
pub unsafe extern "C" fn utmpxname(_file: *const libc::c_char) -> libc::c_int {
    0
}

use crate::*; // import macros from `../../macros.rs`

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

    #[cfg(not(target_env = "musl"))]
    pub use libc::__UT_HOSTSIZE as UT_HOSTSIZE;
    #[cfg(target_env = "musl")]
    pub use libc::UT_HOSTSIZE;

    #[cfg(not(target_env = "musl"))]
    pub use libc::__UT_LINESIZE as UT_LINESIZE;
    #[cfg(target_env = "musl")]
    pub use libc::UT_LINESIZE;

    #[cfg(not(target_env = "musl"))]
    pub use libc::__UT_NAMESIZE as UT_NAMESIZE;
    #[cfg(target_env = "musl")]
    pub use libc::UT_NAMESIZE;

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

#[cfg(target_vendor = "apple")]
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

#[cfg(target_os = "netbsd")]
mod ut {
    pub static DEFAULT_FILE: &str = "/var/run/utmpx";

    pub const ACCOUNTING: usize = 9;
    pub const SHUTDOWN_TIME: usize = 11;

    pub use libc::_UTX_HOSTSIZE as UT_HOSTSIZE;
    pub use libc::_UTX_IDSIZE as UT_IDSIZE;
    pub use libc::_UTX_LINESIZE as UT_LINESIZE;
    pub use libc::_UTX_USERSIZE as UT_NAMESIZE;

    pub use libc::ACCOUNTING;
    pub use libc::DEAD_PROCESS;
    pub use libc::EMPTY;
    pub use libc::INIT_PROCESS;
    pub use libc::LOGIN_PROCESS;
    pub use libc::NEW_TIME;
    pub use libc::OLD_TIME;
    pub use libc::RUN_LVL;
    pub use libc::SIGNATURE;
    pub use libc::USER_PROCESS;
}

/// A login record
pub struct Utmpx {
    inner: utmpx,
}

impl Utmpx {
    /// A.K.A. ut.ut_type
    pub fn record_type(&self) -> i16 {
        self.inner.ut_type
    }
    /// A.K.A. ut.ut_pid
    pub fn pid(&self) -> i32 {
        self.inner.ut_pid
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
    pub fn login_time(&self) -> time::OffsetDateTime {
        #[allow(clippy::unnecessary_cast)]
        let ts_nanos: i128 = (1_000_000_000_i64 * self.inner.ut_tv.tv_sec as i64
            + 1_000_i64 * self.inner.ut_tv.tv_usec as i64)
            .into();
        let local_offset =
            time::OffsetDateTime::now_local().map_or_else(|_| time::UtcOffset::UTC, |v| v.offset());
        time::OffsetDateTime::from_unix_timestamp_nanos(ts_nanos)
            .unwrap()
            .to_offset(local_offset)
    }
    /// A.K.A. ut.ut_exit
    ///
    /// Return (e_termination, e_exit)
    #[cfg(target_os = "linux")]
    pub fn exit_status(&self) -> (i16, i16) {
        (self.inner.ut_exit.e_termination, self.inner.ut_exit.e_exit)
    }
    /// A.K.A. ut.ut_exit
    ///
    /// Return (0, 0) on Non-Linux platform
    #[cfg(not(target_os = "linux"))]
    pub fn exit_status(&self) -> (i16, i16) {
        (0, 0)
    }
    /// Consumes the `Utmpx`, returning the underlying C struct utmpx
    pub fn into_inner(self) -> utmpx {
        self.inner
    }
    /// check if the record is a user process
    pub fn is_user_process(&self) -> bool {
        !self.user().is_empty() && self.record_type() == USER_PROCESS
    }

    /// Canonicalize host name using DNS
    pub fn canon_host(&self) -> IOResult<String> {
        let host = self.host();

        let (hostname, display) = host.split_once(':').unwrap_or((&host, ""));

        if !hostname.is_empty() {
            use dns_lookup::{AddrInfoHints, getaddrinfo};

            const AI_CANONNAME: i32 = 0x2;
            let hints = AddrInfoHints {
                flags: AI_CANONNAME,
                ..AddrInfoHints::default()
            };
            if let Ok(sockets) = getaddrinfo(Some(hostname), None, Some(hints)) {
                let sockets = sockets.collect::<IOResult<Vec<_>>>()?;
                for socket in sockets {
                    if let Some(ai_canonname) = socket.canonname {
                        return Ok(if display.is_empty() {
                            ai_canonname
                        } else {
                            format!("{ai_canonname}:{display}")
                        });
                    }
                }
            } else {
                // GNU coreutils has this behavior
                return Ok(hostname.to_string());
            }
        }

        Ok(host.to_string())
    }

    /// Iterate through all the utmp records.
    ///
    /// This will use the default location, or the path [`Utmpx::iter_all_records_from`]
    /// was most recently called with.
    ///
    /// On systems with systemd-logind feature enabled at compile time,
    /// this will use systemd-logind instead of traditional utmp files.
    ///
    /// Only one instance of [`UtmpxIter`] may be active at a time. This
    /// function will block as long as one is still active. Beware!
    pub fn iter_all_records() -> UtmpxIter {
        #[cfg(feature = "feat_systemd_logind")]
        {
            // Use systemd-logind instead of traditional utmp when feature is enabled
            UtmpxIter::new_systemd()
        }

        #[cfg(not(feature = "feat_systemd_logind"))]
        {
            let iter = UtmpxIter::new();
            unsafe {
                // This can technically fail, and it would be nice to detect that,
                // but it doesn't return anything so we'd have to do nasty things
                // with errno.
                #[cfg_attr(target_env = "musl", allow(deprecated))]
                setutxent();
            }
            iter
        }
    }

    /// Iterate through all the utmp records from a specific file.
    ///
    /// No failure is reported or detected.
    ///
    /// This function affects subsequent calls to [`Utmpx::iter_all_records`].
    ///
    /// On systems with systemd-logind feature enabled at compile time,
    /// if the path matches the default utmp file, this will use systemd-logind
    /// instead of traditional utmp files.
    ///
    /// The same caveats as for [`Utmpx::iter_all_records`] apply.
    pub fn iter_all_records_from<P: AsRef<Path>>(path: P) -> UtmpxIter {
        #[cfg(feature = "feat_systemd_logind")]
        {
            // Use systemd-logind for default utmp file when feature is enabled
            if path.as_ref().to_str() == Some(DEFAULT_FILE) {
                return UtmpxIter::new_systemd();
            }
        }

        let iter = UtmpxIter::new();
        let path = CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();
        unsafe {
            // In glibc, utmpxname() only fails if there's not enough memory
            // to copy the string.
            // Solaris returns 1 on success instead of 0. Supposedly there also
            // exist systems where it returns void.
            // GNU who on Debian seems to output nothing if an invalid filename
            // is specified, no warning or anything.
            // So this function is pretty crazy and we don't try to detect errors.
            // Not much we can do besides pray.
            #[cfg_attr(target_env = "musl", allow(deprecated))]
            utmpxname(path.as_ptr());
            #[cfg_attr(target_env = "musl", allow(deprecated))]
            setutxent();
        }
        iter
    }
}

// On some systems these functions are not thread-safe. On others they're
// thread-local. Therefore we use a mutex to allow only one guard to exist at
// a time, and make sure UtmpxIter cannot be sent across threads.
//
// I believe the only technical memory unsafety that could happen is a data
// race while copying the data out of the pointer returned by getutxent(), but
// ordinary race conditions are also very much possible.
static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Iterator of login records
pub struct UtmpxIter {
    #[allow(dead_code)]
    guard: MutexGuard<'static, ()>,
    /// Ensure UtmpxIter is !Send. Technically redundant because MutexGuard
    /// is also !Send.
    phantom: PhantomData<std::rc::Rc<()>>,
    #[cfg(feature = "feat_systemd_logind")]
    systemd_iter: Option<systemd_logind::SystemdUtmpxIter>,
}

impl UtmpxIter {
    fn new() -> Self {
        // PoisonErrors can safely be ignored
        let guard = LOCK.lock().unwrap_or_else(|err| err.into_inner());
        Self {
            guard,
            phantom: PhantomData,
            #[cfg(feature = "feat_systemd_logind")]
            systemd_iter: None,
        }
    }

    #[cfg(feature = "feat_systemd_logind")]
    fn new_systemd() -> Self {
        // PoisonErrors can safely be ignored
        let guard = LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let systemd_iter = match systemd_logind::SystemdUtmpxIter::new() {
            Ok(iter) => Some(iter),
            Err(e) => {
                if Path::new("/run/systemd/system").exists() {
                    // systemd is running, but we failed to get records.
                    // This is likely a problem (e.g., D-Bus permissions).
                    // We print an error and return an empty iterator to avoid
                    // falling back to a likely-empty utmp file.
                    show_error!("failed to contact systemd-logind: {e}");
                    Some(systemd_logind::SystemdUtmpxIter {
                        records: vec![],
                        current_index: 0,
                    })
                } else {
                    // systemd is not running, so we expect this to fail.
                    // Return None to allow fallback to traditional utmp.
                    None
                }
            }
        };
        Self {
            guard,
            phantom: PhantomData,
            systemd_iter,
        }
    }
}

/// Wrapper type that can hold either traditional utmpx records or systemd records
pub enum UtmpxRecord {
    Traditional(Box<Utmpx>),
    #[cfg(feature = "feat_systemd_logind")]
    Systemd(systemd_logind::SystemdUtmpxCompat),
}

impl UtmpxRecord {
    /// A.K.A. ut.ut_type
    pub fn record_type(&self) -> i16 {
        match self {
            UtmpxRecord::Traditional(utmpx) => utmpx.record_type(),
            #[cfg(feature = "feat_systemd_logind")]
            UtmpxRecord::Systemd(systemd) => systemd.record_type(),
        }
    }

    /// A.K.A. ut.ut_pid
    pub fn pid(&self) -> i32 {
        match self {
            UtmpxRecord::Traditional(utmpx) => utmpx.pid(),
            #[cfg(feature = "feat_systemd_logind")]
            UtmpxRecord::Systemd(systemd) => systemd.pid(),
        }
    }

    /// A.K.A. ut.ut_id
    pub fn terminal_suffix(&self) -> String {
        match self {
            UtmpxRecord::Traditional(utmpx) => utmpx.terminal_suffix(),
            #[cfg(feature = "feat_systemd_logind")]
            UtmpxRecord::Systemd(systemd) => systemd.terminal_suffix(),
        }
    }

    /// A.K.A. ut.ut_user
    pub fn user(&self) -> String {
        match self {
            UtmpxRecord::Traditional(utmpx) => utmpx.user(),
            #[cfg(feature = "feat_systemd_logind")]
            UtmpxRecord::Systemd(systemd) => systemd.user(),
        }
    }

    /// A.K.A. ut.ut_host
    pub fn host(&self) -> String {
        match self {
            UtmpxRecord::Traditional(utmpx) => utmpx.host(),
            #[cfg(feature = "feat_systemd_logind")]
            UtmpxRecord::Systemd(systemd) => systemd.host(),
        }
    }

    /// A.K.A. ut.ut_line
    pub fn tty_device(&self) -> String {
        match self {
            UtmpxRecord::Traditional(utmpx) => utmpx.tty_device(),
            #[cfg(feature = "feat_systemd_logind")]
            UtmpxRecord::Systemd(systemd) => systemd.tty_device(),
        }
    }

    /// A.K.A. ut.ut_tv
    pub fn login_time(&self) -> time::OffsetDateTime {
        match self {
            UtmpxRecord::Traditional(utmpx) => utmpx.login_time(),
            #[cfg(feature = "feat_systemd_logind")]
            UtmpxRecord::Systemd(systemd) => systemd.login_time(),
        }
    }

    /// A.K.A. ut.ut_exit
    ///
    /// Return (e_termination, e_exit)
    pub fn exit_status(&self) -> (i16, i16) {
        match self {
            UtmpxRecord::Traditional(utmpx) => utmpx.exit_status(),
            #[cfg(feature = "feat_systemd_logind")]
            UtmpxRecord::Systemd(systemd) => systemd.exit_status(),
        }
    }

    /// check if the record is a user process
    pub fn is_user_process(&self) -> bool {
        match self {
            UtmpxRecord::Traditional(utmpx) => utmpx.is_user_process(),
            #[cfg(feature = "feat_systemd_logind")]
            UtmpxRecord::Systemd(systemd) => systemd.is_user_process(),
        }
    }

    /// Canonicalize host name using DNS
    pub fn canon_host(&self) -> IOResult<String> {
        match self {
            UtmpxRecord::Traditional(utmpx) => utmpx.canon_host(),
            #[cfg(feature = "feat_systemd_logind")]
            UtmpxRecord::Systemd(systemd) => systemd.canon_host(),
        }
    }
}

impl Iterator for UtmpxIter {
    type Item = UtmpxRecord;
    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "feat_systemd_logind")]
        {
            if let Some(ref mut systemd_iter) = self.systemd_iter {
                if let Some(systemd_record) = systemd_iter.next() {
                    return Some(UtmpxRecord::Systemd(systemd_record));
                }
            }
        }

        unsafe {
            #[cfg_attr(target_env = "musl", allow(deprecated))]
            let res = getutxent();
            if res.is_null() {
                None
            } else {
                // The data behind this pointer will be replaced by the next
                // call to getutxent(), so we have to read it now.
                // All the strings live inline in the struct as arrays, which
                // makes things easier.
                Some(UtmpxRecord::Traditional(Box::new(Utmpx {
                    inner: ptr::read(res as *const _),
                })))
            }
        }
    }
}

impl Drop for UtmpxIter {
    fn drop(&mut self) {
        unsafe {
            #[cfg_attr(target_env = "musl", allow(deprecated))]
            endutxent();
        }
    }
}
