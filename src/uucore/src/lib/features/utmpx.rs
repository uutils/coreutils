// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore IDLEN logind ohos setutxdb UTXDB

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

#[cfg(all(feature = "systemd-logind", target_os = "linux"))]
use crate::features::systemd_logind;

pub use self::ut::*;

// See the FAQ at https://wiki.musl-libc.org/faq#Q:-Why-is-the-utmp/wtmp-functionality-only-implemented-as-stubs?
// Musl implements only stubs for the utmp functions, and the libc crate issues a deprecation warning about this.
// However, calling these stubs is the correct approach to maintain consistent behavior with GNU coreutils.
#[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
pub use libc::endutxent;
#[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
pub use libc::getutxent;
#[cfg(target_os = "freebsd")]
use libc::setutxdb;
#[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
pub use libc::setutxent;
use libc::utmpx;
#[cfg(any(
    target_vendor = "apple",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "cygwin"
))]
#[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
pub use libc::utmpxname;

use crate::libc; // import macros from `../../macros.rs`

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

    #[cfg(target_env = "gnu")]
    pub use libc::__UT_HOSTSIZE as UT_HOSTSIZE;
    #[cfg(not(target_env = "gnu"))]
    pub use libc::UT_HOSTSIZE;

    #[cfg(target_env = "gnu")]
    pub use libc::__UT_LINESIZE as UT_LINESIZE;
    #[cfg(not(target_env = "gnu"))]
    pub use libc::UT_LINESIZE;

    #[cfg(target_env = "gnu")]
    pub use libc::__UT_NAMESIZE as UT_NAMESIZE;
    #[cfg(not(target_env = "gnu"))]
    pub use libc::UT_NAMESIZE;

    pub const UT_IDSIZE: usize = 4;

    pub use libc::{
        ACCOUNTING, BOOT_TIME, DEAD_PROCESS, EMPTY, INIT_PROCESS, LOGIN_PROCESS, NEW_TIME,
        OLD_TIME, RUN_LVL, USER_PROCESS,
    };
}

#[cfg(target_vendor = "apple")]
mod ut {
    pub static DEFAULT_FILE: &str = "/var/run/utmpx";

    pub use libc::{
        _UTX_HOSTSIZE as UT_HOSTSIZE, _UTX_IDSIZE as UT_IDSIZE, _UTX_LINESIZE as UT_LINESIZE,
        _UTX_USERSIZE as UT_NAMESIZE,
    };

    pub use libc::{
        ACCOUNTING, BOOT_TIME, DEAD_PROCESS, EMPTY, INIT_PROCESS, LOGIN_PROCESS, NEW_TIME,
        OLD_TIME, RUN_LVL, SHUTDOWN_TIME, SIGNATURE, USER_PROCESS,
    };
}

#[cfg(target_os = "freebsd")]
mod ut {
    pub static DEFAULT_FILE: &str = "";

    pub const UT_LINESIZE: usize = 16;
    pub const UT_NAMESIZE: usize = 32;
    pub const UT_IDSIZE: usize = 8;
    pub const UT_HOSTSIZE: usize = 128;

    pub use libc::{
        BOOT_TIME, DEAD_PROCESS, EMPTY, INIT_PROCESS, LOGIN_PROCESS, NEW_TIME, OLD_TIME,
        SHUTDOWN_TIME, USER_PROCESS,
    };
}

#[cfg(target_os = "netbsd")]
mod ut {
    pub static DEFAULT_FILE: &str = "/var/run/utmpx";

    pub const SHUTDOWN_TIME: usize = 11;

    pub use libc::{
        _UTX_HOSTSIZE as UT_HOSTSIZE, _UTX_IDSIZE as UT_IDSIZE, _UTX_LINESIZE as UT_LINESIZE,
        _UTX_USERSIZE as UT_NAMESIZE,
    };

    pub use libc::ACCOUNTING;
    pub const BOOT_TIME: i16 = libc::BOOT_TIME as i16;
    pub const DEAD_PROCESS: i16 = libc::DEAD_PROCESS as i16;
    pub const EMPTY: i16 = libc::EMPTY as i16;
    pub const INIT_PROCESS: i16 = libc::INIT_PROCESS as i16;
    pub const LOGIN_PROCESS: i16 = libc::LOGIN_PROCESS as i16;
    pub const NEW_TIME: i16 = libc::NEW_TIME as i16;
    pub const OLD_TIME: i16 = libc::OLD_TIME as i16;
    pub const RUN_LVL: i16 = libc::RUN_LVL as i16;
    pub const SIGNATURE: i16 = libc::SIGNATURE as i16;
    pub const USER_PROCESS: i16 = libc::USER_PROCESS as i16;
}

#[cfg(target_os = "cygwin")]
mod ut {
    pub static DEFAULT_FILE: &str = "";

    pub use libc::{UT_HOSTSIZE, UT_IDLEN, UT_LINESIZE, UT_NAMESIZE};

    pub use libc::{
        BOOT_TIME, DEAD_PROCESS, INIT_PROCESS, LOGIN_PROCESS, NEW_TIME, OLD_TIME, RUN_LVL,
        USER_PROCESS,
    };
}

/// A login record
pub struct Utmpx {
    inner: utmpx,
}

#[cfg(target_os = "netbsd")]
impl Utmpx {
    fn ut_type(&self) -> i16 {
        self.inner.ut_type as i16
    }
    fn ut_user(&self) -> String {
        chars2string!(self.inner.ut_name)
    }
}

#[cfg(not(target_os = "netbsd"))]
impl Utmpx {
    fn ut_type(&self) -> i16 {
        self.inner.ut_type
    }
    fn ut_user(&self) -> String {
        chars2string!(self.inner.ut_user)
    }
}

impl Utmpx {
    /// A.K.A. `ut.ut_type`
    pub fn record_type(&self) -> i16 {
        self.ut_type()
    }
    /// A.K.A. `ut.ut_pid`
    pub fn pid(&self) -> i32 {
        self.inner.ut_pid
    }
    /// A.K.A. `ut.ut_id`
    pub fn terminal_suffix(&self) -> String {
        chars2string!(self.inner.ut_id)
    }
    ///  A.K.A. `ut.ut_user` / `ut.ut_name` (NetBSD)
    pub fn user(&self) -> String {
        self.ut_user()
    }
    /// A.K.A. `ut.ut_host`
    pub fn host(&self) -> String {
        chars2string!(self.inner.ut_host)
    }
    /// A.K.A. `ut.ut_line`
    pub fn tty_device(&self) -> String {
        chars2string!(self.inner.ut_line)
    }
    /// A.K.A. `ut.ut_tv`
    pub fn login_time(&self) -> time::OffsetDateTime {
        #[allow(clippy::unnecessary_cast)]
        let ts_nanos: i128 = (1_000_000_000_i64 * self.inner.ut_tv.tv_sec as i64
            + 1_000_i64 * self.inner.ut_tv.tv_usec as i64)
            .into();
        let local_offset = time::OffsetDateTime::now_local()
            .map_or_else(|_| time::UtcOffset::UTC, time::OffsetDateTime::offset);
        time::OffsetDateTime::from_unix_timestamp_nanos(ts_nanos)
            .unwrap()
            .to_offset(local_offset)
    }
    /// A.K.A. `ut.ut_exit`
    ///
    /// Return (`e_termination`, `e_exit`)
    #[cfg(target_os = "linux")]
    pub fn exit_status(&self) -> (i16, i16) {
        (self.inner.ut_exit.e_termination, self.inner.ut_exit.e_exit)
    }
    /// A.K.A. `ut.ut_exit`
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

        Ok(host)
    }

    /// Read records from the most recently selected path, or the default file.
    /// Only one iterator may be active at a time.
    pub fn iter_all_records() -> UtmpxIter {
        UtmpxIter::new(None)
    }

    /// Select a login file for this and subsequent iterators.
    /// Linux falls back to logind only when the default file has no complete record.
    pub fn iter_all_records_from<P: AsRef<Path>>(path: P) -> UtmpxIter {
        UtmpxIter::new(Some(path.as_ref()))
    }
}

// Serialize libc's utmp state and track whether its selected path is DEFAULT_FILE.
// The iterator stays on its creating thread because that state may be thread-local.
static LOCK: Mutex<bool> = Mutex::new(true);

#[cfg(all(target_os = "linux", feature = "systemd-logind"))]
fn has_traditional_records(path: &Path) -> bool {
    // musl's utmp functions are stubs, even for populated files.
    #[cfg(target_env = "musl")]
    {
        let _ = path;
        false
    }
    #[cfg(not(target_env = "musl"))]
    {
        use std::os::unix::fs::OpenOptionsExt;

        // Probe readability without waiting for a FIFO writer. libc still reads
        // the records and handles their locking.
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .and_then(|file| file.metadata())
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() >= size_of::<utmpx>() as u64)
    }
}

/// Iterator of login records
pub struct UtmpxIter {
    _guard: MutexGuard<'static, bool>,
    /// Keep the iterator on the thread that accesses libc's utmp state.
    phantom: PhantomData<std::rc::Rc<()>>,
    #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
    systemd_iter: Option<systemd_logind::SystemdUtmpxIter>,
}

impl UtmpxIter {
    fn new(path: Option<&Path>) -> Self {
        let mut guard = LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(path) = path {
            let filename = CString::new(path.as_os_str().as_bytes()).unwrap();
            // SAFETY: LOCK protects libc's selected file, and filename is NUL-terminated.
            unsafe {
                #[cfg(target_os = "freebsd")]
                setutxdb(libc::UTXDB_ACTIVE, filename.as_ptr());
                #[cfg(not(target_os = "freebsd"))]
                #[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
                utmpxname(filename.as_ptr());
            }
            *guard = path == Path::new(DEFAULT_FILE);
        }

        #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
        let systemd_iter = {
            if *guard && !has_traditional_records(Path::new(DEFAULT_FILE)) {
                systemd_logind::SystemdUtmpxIter::new().ok()
            } else {
                None
            }
        };
        #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
        if let Some(systemd_iter) = systemd_iter {
            return Self {
                _guard: guard,
                phantom: PhantomData,
                systemd_iter: Some(systemd_iter),
            };
        }

        // SAFETY: The iterator holds LOCK while resetting libc's utmp cursor.
        unsafe {
            #[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
            setutxent();
        }
        Self {
            _guard: guard,
            phantom: PhantomData,
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            systemd_iter: None,
        }
    }
}

#[cfg(all(test, target_os = "linux", feature = "systemd-logind"))]
#[path = "utmpx/tests.rs"]
mod tests;

/// Wrapper type that can hold either traditional utmpx records or systemd records
pub enum UtmpxRecord {
    Traditional(Box<Utmpx>),
    #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
    Systemd(systemd_logind::SystemdUtmpxCompat),
}

impl UtmpxRecord {
    /// A.K.A. `ut.ut_type`
    pub fn record_type(&self) -> i16 {
        match self {
            Self::Traditional(utmpx) => utmpx.record_type(),
            #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.record_type(),
        }
    }

    /// A.K.A. `ut.ut_pid`
    pub fn pid(&self) -> i32 {
        match self {
            Self::Traditional(utmpx) => utmpx.pid(),
            #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.pid(),
        }
    }

    /// A.K.A. `ut.ut_id`
    pub fn terminal_suffix(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.terminal_suffix(),
            #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.terminal_suffix(),
        }
    }

    /// A.K.A. `ut.ut_user`
    pub fn user(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.user(),
            #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.user(),
        }
    }

    /// A.K.A. `ut.ut_host`
    pub fn host(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.host(),
            #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.host(),
        }
    }

    /// A.K.A. `ut.ut_line`
    pub fn tty_device(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.tty_device(),
            #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.tty_device(),
        }
    }

    /// A.K.A. `ut.ut_tv`
    pub fn login_time(&self) -> time::OffsetDateTime {
        match self {
            Self::Traditional(utmpx) => utmpx.login_time(),
            #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.login_time(),
        }
    }

    /// A.K.A. `ut.ut_exit`
    ///
    /// Return (`e_termination`, `e_exit`)
    pub fn exit_status(&self) -> (i16, i16) {
        match self {
            Self::Traditional(utmpx) => utmpx.exit_status(),
            #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.exit_status(),
        }
    }

    /// check if the record is a user process
    pub fn is_user_process(&self) -> bool {
        match self {
            Self::Traditional(utmpx) => utmpx.is_user_process(),
            #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.is_user_process(),
        }
    }

    /// Canonicalize host name using DNS
    pub fn canon_host(&self) -> IOResult<String> {
        match self {
            Self::Traditional(utmpx) => utmpx.canon_host(),
            #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
            Self::Systemd(systemd) => Ok(systemd.canon_host()),
        }
    }
}

impl Iterator for UtmpxIter {
    type Item = UtmpxRecord;
    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(all(feature = "systemd-logind", target_os = "linux"))]
        {
            if let Some(ref mut systemd_iter) = self.systemd_iter {
                // We have a systemd iterator - use it exclusively (never fall back to traditional utmp)
                return systemd_iter.next().map(UtmpxRecord::Systemd);
            }
        }

        // Traditional utmp path
        unsafe {
            #[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
            let res = getutxent();
            if res.is_null() {
                None
            } else {
                // The data behind this pointer will be replaced by the next
                // call to getutxent(), so we have to read it now.
                // All the strings live inline in the struct as arrays, which
                // makes things easier.
                Some(UtmpxRecord::Traditional(Box::new(Utmpx {
                    inner: ptr::read(res.cast_const()),
                })))
            }
        }
    }
}

impl Drop for UtmpxIter {
    fn drop(&mut self) {
        unsafe {
            #[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
            endutxent();
        }
    }
}
