// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore IDLEN logind

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
#[cfg(target_os = "linux")]
use std::mem::size_of;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::sync::{Mutex, MutexGuard};

#[cfg(target_os = "linux")]
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
#[cfg(any(
    target_vendor = "apple",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "cygwin"
))]
#[cfg_attr(target_env = "musl", allow(deprecated))]
pub use libc::utmpxname;

/// # Safety
/// Just fixed the clippy warning. Please add description here.
#[cfg(target_os = "freebsd")]
pub unsafe extern "C" fn utmpxname(_file: *const libc::c_char) -> libc::c_int {
    0
}

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

    pub const SHUTDOWN_TIME: usize = 11;

    pub use libc::_UTX_HOSTSIZE as UT_HOSTSIZE;
    pub use libc::_UTX_IDSIZE as UT_IDSIZE;
    pub use libc::_UTX_LINESIZE as UT_LINESIZE;
    pub use libc::_UTX_USERSIZE as UT_NAMESIZE;

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

    pub use libc::UT_HOSTSIZE;
    pub use libc::UT_IDLEN;
    pub use libc::UT_LINESIZE;
    pub use libc::UT_NAMESIZE;

    pub use libc::BOOT_TIME;
    pub use libc::DEAD_PROCESS;
    pub use libc::INIT_PROCESS;
    pub use libc::LOGIN_PROCESS;
    pub use libc::NEW_TIME;
    pub use libc::OLD_TIME;
    pub use libc::RUN_LVL;
    pub use libc::USER_PROCESS;
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
    /// A.K.A. ut.ut_type
    pub fn record_type(&self) -> i16 {
        self.ut_type()
    }
    /// A.K.A. ut.ut_pid
    pub fn pid(&self) -> i32 {
        self.inner.ut_pid
    }
    /// A.K.A. ut.ut_id
    pub fn terminal_suffix(&self) -> String {
        chars2string!(self.inner.ut_id)
    }
    ///  A.K.A. ut.ut_user / ut.ut_name (NetBSD)
    pub fn user(&self) -> String {
        self.ut_user()
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
        let local_offset = time::OffsetDateTime::now_local()
            .map_or_else(|_| time::UtcOffset::UTC, time::OffsetDateTime::offset);
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

        Ok(host)
    }

    /// Iterate through all the utmp records.
    ///
    /// This will use the default location, or the path [`Utmpx::iter_all_records_from`]
    /// was most recently called with.
    ///
    /// On Linux, this will use a populated traditional utmp file when available.
    /// If the default utmp file is missing or empty, it will try systemd-logind
    /// and fall back to traditional utmp if systemd-logind is unavailable.
    ///
    /// Only one instance of [`UtmpxIter`] may be active at a time. This
    /// function will block as long as one is still active. Beware!
    pub fn iter_all_records() -> UtmpxIter {
        #[cfg(target_os = "linux")]
        {
            // The usability check inspects DEFAULT_FILE, while iteration reads
            // libc's process-global path, which is sticky across
            // `iter_all_records_from` calls. These can only disagree if a caller
            // previously selected a custom path and then calls this function;
            // no in-tree caller does that.
            if traditional_utmp_is_usable(Path::new(DEFAULT_FILE)) {
                let iter = UtmpxIter::new();
                unsafe {
                    #[cfg_attr(target_env = "musl", allow(deprecated))]
                    setutxent();
                }
                iter
            } else {
                UtmpxIter::new_systemd_with_fallback_path(None)
            }
        }

        #[cfg(not(target_os = "linux"))]
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
    /// On Linux, if the path matches the default utmp file and that file is
    /// missing or empty, this will try systemd-logind first and fall back to
    /// that file if systemd-logind is unavailable.
    ///
    /// The same caveats as for [`Utmpx::iter_all_records`] apply.
    pub fn iter_all_records_from<P: AsRef<Path>>(path: P) -> UtmpxIter {
        #[cfg(target_os = "linux")]
        {
            if path.as_ref() == Path::new(DEFAULT_FILE)
                && !traditional_utmp_is_usable(path.as_ref())
            {
                return UtmpxIter::new_systemd_with_fallback_path(Some(path.as_ref()));
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
static LOCK: Mutex<()> = Mutex::new(());

/// Whether the traditional utmp file can serve login records: it must exist,
/// be a regular file holding at least one record, and be readable.
///
/// A present-but-empty file (e.g. a non-systemd system with no logins yet) is
/// treated as unusable, so each invocation pays one failed dlopen of
/// libsystemd before falling back; this keeps the common systemd case correct
/// at a negligible cost elsewhere.
#[cfg(target_os = "linux")]
fn traditional_utmp_is_usable(path: &Path) -> bool {
    #[cfg(target_env = "musl")]
    {
        let _ = path;
        false
    }

    #[cfg(not(target_env = "musl"))]
    {
        std::fs::metadata(path).is_ok_and(|metadata| {
            // Stat before opening: opening a non-regular file could block
            // (e.g. a FIFO with no writer).
            metadata.is_file()
                && metadata.len() >= size_of::<utmpx>() as u64
                && std::fs::File::open(path).is_ok()
        })
    }
}

/// Iterator of login records
pub struct UtmpxIter {
    #[allow(dead_code)]
    guard: MutexGuard<'static, ()>,
    /// Ensure UtmpxIter is !Send. Technically redundant because MutexGuard
    /// is also !Send.
    phantom: PhantomData<std::rc::Rc<()>>,
    #[cfg(target_os = "linux")]
    systemd_iter: Option<systemd_logind::SystemdUtmpxIter>,
}

impl UtmpxIter {
    fn new() -> Self {
        // PoisonErrors can safely be ignored
        let guard = LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Self {
            guard,
            phantom: PhantomData,
            #[cfg(target_os = "linux")]
            systemd_iter: None,
        }
    }

    #[cfg(target_os = "linux")]
    fn new_systemd_with_fallback_path(fallback_path: Option<&Path>) -> Self {
        Self::new_systemd_with_fallback_path_using(
            fallback_path,
            systemd_logind::SystemdUtmpxIter::new,
        )
    }

    #[cfg(target_os = "linux")]
    fn new_systemd_with_fallback_path_using<F, E>(
        fallback_path: Option<&Path>,
        new_systemd_iter: F,
    ) -> Self
    where
        F: FnOnce() -> Result<systemd_logind::SystemdUtmpxIter, E>,
    {
        // PoisonErrors can safely be ignored
        let guard = LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Ok(iter) = new_systemd_iter() {
            Self {
                guard,
                phantom: PhantomData,
                systemd_iter: Some(iter),
            }
        } else {
            // Fall back to traditional utmp when systemd-logind is unavailable.
            // Callers only reach this constructor after determining that the
            // traditional file is missing or empty, so re-reading it yields no
            // records — matching GNU coreutils, which also prints nothing when
            // /var/run/utmp is absent.
            unsafe {
                if let Some(path) = fallback_path {
                    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
                    #[cfg_attr(target_env = "musl", allow(deprecated))]
                    utmpxname(path.as_ptr());
                }
                #[cfg_attr(target_env = "musl", allow(deprecated))]
                setutxent();
            }
            Self {
                guard,
                phantom: PhantomData,
                systemd_iter: None,
            }
        }
    }
}

#[cfg(all(test, target_os = "linux", not(target_env = "musl")))]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::mem::zeroed;

    fn write_utmpx_record(path: &Path, user: &str) {
        // SAFETY: A zero-filled utmpx value is valid, and the initialized value is
        // written as bytes without outliving it.
        let mut record: utmpx = unsafe { zeroed() };
        record.ut_type = USER_PROCESS;
        for (destination, source) in record.ut_user.iter_mut().zip(user.bytes()) {
            *destination = source as _;
        }

        // SAFETY: `record` remains alive for the duration of `write_all`, and the
        // byte slice covers exactly its initialized object representation.
        let bytes = unsafe {
            std::slice::from_raw_parts((&raw const record).cast::<u8>(), size_of::<utmpx>())
        };
        File::create(path).unwrap().write_all(bytes).unwrap();
    }

    #[test]
    fn systemd_failure_uses_explicit_fallback_path() {
        let directory = tempfile::tempdir().unwrap();
        let custom_path = directory.path().join("custom-utmp");
        let fallback_path = directory.path().join("fallback-utmp");
        write_utmpx_record(&custom_path, "custom");
        write_utmpx_record(&fallback_path, "fallback");

        let custom_record = Utmpx::iter_all_records_from(&custom_path)
            .next()
            .expect("custom utmp record");
        assert_eq!(custom_record.user(), "custom");

        // LOCK is released here with libc's global path still set to
        // `custom_path`, so a parallel test iterating utmp records would
        // observe it (none does today). Keeping the first iterator alive
        // instead would deadlock: the constructor below reacquires the
        // non-reentrant LOCK.
        let mut fallback_iter =
            UtmpxIter::new_systemd_with_fallback_path_using(Some(&fallback_path), || {
                Err::<systemd_logind::SystemdUtmpxIter, ()>(())
            });
        let fallback_record = fallback_iter.next().expect("fallback utmp record");
        assert_eq!(fallback_record.user(), "fallback");

        // Restore libc's process-global utmp path for other tests. This must
        // happen before dropping `fallback_iter`: the iterator still holds
        // LOCK, so no other thread can observe the intermediate path.
        let default_path = CString::new(DEFAULT_FILE).unwrap();
        unsafe {
            utmpxname(default_path.as_ptr());
        }
        drop(fallback_iter);
    }

    #[test]
    fn traditional_utmp_requires_at_least_one_record() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");

        File::create(&path).unwrap();
        assert!(!traditional_utmp_is_usable(&path));

        write_utmpx_record(&path, "user");
        assert!(traditional_utmp_is_usable(&path));
    }
}

/// Wrapper type that can hold either traditional utmpx records or systemd records
pub enum UtmpxRecord {
    Traditional(Box<Utmpx>),
    #[cfg(target_os = "linux")]
    Systemd(systemd_logind::SystemdUtmpxCompat),
}

impl UtmpxRecord {
    /// A.K.A. ut.ut_type
    pub fn record_type(&self) -> i16 {
        match self {
            Self::Traditional(utmpx) => utmpx.record_type(),
            #[cfg(target_os = "linux")]
            Self::Systemd(systemd) => systemd.record_type(),
        }
    }

    /// A.K.A. ut.ut_pid
    pub fn pid(&self) -> i32 {
        match self {
            Self::Traditional(utmpx) => utmpx.pid(),
            #[cfg(target_os = "linux")]
            Self::Systemd(systemd) => systemd.pid(),
        }
    }

    /// A.K.A. ut.ut_id
    pub fn terminal_suffix(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.terminal_suffix(),
            #[cfg(target_os = "linux")]
            Self::Systemd(systemd) => systemd.terminal_suffix(),
        }
    }

    /// A.K.A. ut.ut_user
    pub fn user(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.user(),
            #[cfg(target_os = "linux")]
            Self::Systemd(systemd) => systemd.user(),
        }
    }

    /// A.K.A. ut.ut_host
    pub fn host(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.host(),
            #[cfg(target_os = "linux")]
            Self::Systemd(systemd) => systemd.host(),
        }
    }

    /// A.K.A. ut.ut_line
    pub fn tty_device(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.tty_device(),
            #[cfg(target_os = "linux")]
            Self::Systemd(systemd) => systemd.tty_device(),
        }
    }

    /// A.K.A. ut.ut_tv
    pub fn login_time(&self) -> time::OffsetDateTime {
        match self {
            Self::Traditional(utmpx) => utmpx.login_time(),
            #[cfg(target_os = "linux")]
            Self::Systemd(systemd) => systemd.login_time(),
        }
    }

    /// A.K.A. ut.ut_exit
    ///
    /// Return (e_termination, e_exit)
    pub fn exit_status(&self) -> (i16, i16) {
        match self {
            Self::Traditional(utmpx) => utmpx.exit_status(),
            #[cfg(target_os = "linux")]
            Self::Systemd(systemd) => systemd.exit_status(),
        }
    }

    /// check if the record is a user process
    pub fn is_user_process(&self) -> bool {
        match self {
            Self::Traditional(utmpx) => utmpx.is_user_process(),
            #[cfg(target_os = "linux")]
            Self::Systemd(systemd) => systemd.is_user_process(),
        }
    }

    /// Canonicalize host name using DNS
    pub fn canon_host(&self) -> IOResult<String> {
        match self {
            Self::Traditional(utmpx) => utmpx.canon_host(),
            #[cfg(target_os = "linux")]
            Self::Systemd(systemd) => Ok(systemd.canon_host()),
        }
    }
}

impl Iterator for UtmpxIter {
    type Item = UtmpxRecord;
    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(target_os = "linux")]
        {
            if let Some(ref mut systemd_iter) = self.systemd_iter {
                // Once a systemd iterator was successfully created, use it exclusively.
                // If systemd initialization failed, `systemd_iter` is None and we use
                // traditional utmp below.
                return systemd_iter.next().map(UtmpxRecord::Systemd);
            }
        }

        // Traditional utmp path
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
                    inner: ptr::read(res.cast_const()),
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
