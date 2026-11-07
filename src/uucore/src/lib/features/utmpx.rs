// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore IDLEN logind ohos OFD setutxdb UTXDB WRLCK unlocker SETLK EAGAIN RDLCK UNLCK

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

#[cfg(not(target_os = "linux"))]
use std::ffi::CString;
use std::io::Result as IOResult;
use std::marker::PhantomData;
#[cfg(all(target_os = "linux", not(target_env = "musl")))]
use std::mem::size_of;
#[cfg(not(target_os = "linux"))]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
#[cfg(not(all(target_os = "linux", target_env = "musl")))]
use std::ptr;
use std::sync::{Mutex, MutexGuard};

#[cfg(all(target_os = "linux", feature = "systemd_logind"))]
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

    /// Iterate through all utmp records using the most recently selected path.
    ///
    /// On Linux with `systemd_logind` enabled, the default path selects systemd.
    /// A custom path selects traditional records, including on subsequent calls
    /// to this method. Call [`Self::iter_all_records_from`] with [`DEFAULT_FILE`]
    /// to select systemd again.
    ///
    /// Only one iterator may be active at a time; this blocks until it is dropped.
    pub fn iter_all_records() -> UtmpxIter {
        #[cfg(target_os = "linux")]
        return UtmpxIter::new_traditional(None);

        #[cfg(not(target_os = "linux"))]
        {
            let iter = UtmpxIter::new();
            // SAFETY: The iterator holds LOCK while accessing libc's global state.
            unsafe {
                setutxent();
            }
            iter
        }
    }

    /// Read a specific file, remembering it for subsequent iterators.
    ///
    /// On Linux with `systemd_logind` enabled, [`DEFAULT_FILE`] selects systemd
    /// instead of reading the file. Other paths select traditional records.
    ///
    /// Only one iterator may be active at a time; failures produce no records.
    pub fn iter_all_records_from<P: AsRef<Path>>(path: P) -> UtmpxIter {
        #[cfg(target_os = "linux")]
        return UtmpxIter::new_traditional(Some(path.as_ref()));

        #[cfg(not(target_os = "linux"))]
        {
            let iter = UtmpxIter::new();
            let filename = CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();
            // SAFETY: The iterator holds LOCK, and filename remains valid.
            unsafe {
                #[cfg(target_os = "freebsd")]
                setutxdb(libc::UTXDB_ACTIVE, filename.as_ptr());
                #[cfg(not(target_os = "freebsd"))]
                #[cfg_attr(target_env = "ohos", allow(deprecated))]
                utmpxname(filename.as_ptr());
                #[cfg_attr(target_env = "ohos", allow(deprecated))]
                setutxent();
            }
            iter
        }
    }
}

// On some systems these functions are not thread-safe. On others they're
// thread-local. Therefore we use a mutex to allow only one guard to exist at
// a time, and make sure UtmpxIter cannot be sent across threads.
//
// I believe the only technical memory unsafety that could happen is a data
// race while copying the data out of the pointer returned by getutxent(), but
// ordinary race conditions are also very much possible.
// The selected path shares the lock with libc's global utmp state.
static LOCK: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Open without waiting for a FIFO writer, then validate the opened file.
#[cfg(target_os = "linux")]
fn open_traditional_utmp(path: &Path) -> Option<std::fs::File> {
    #[cfg(target_env = "musl")]
    {
        // musl's utmp APIs are stubs, even when the file is populated.
        let _ = path;
        None
    }

    #[cfg(not(target_env = "musl"))]
    {
        use std::os::unix::fs::OpenOptionsExt;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .ok()?;
        file.metadata().ok()?.is_file().then_some(file)
    }
}

// Bound lock contention waits across one iteration.
#[cfg(all(target_os = "linux", not(target_env = "musl")))]
const UTMP_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(all(target_os = "linux", not(target_env = "musl")))]
fn lock_utmp(
    file: &std::fs::File,
    lock: &libc::flock,
    deadline: std::time::Instant,
) -> IOResult<()> {
    use std::os::fd::AsRawFd;
    use std::time::{Duration, Instant};

    loop {
        // SAFETY: file owns a live descriptor and lock is initialized.
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETLK, lock) } >= 0 {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        let interrupted = error.raw_os_error() == Some(libc::EINTR);
        if !interrupted && !matches!(error.raw_os_error(), Some(libc::EAGAIN | libc::EACCES)) {
            return Err(error);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        if !interrupted {
            std::thread::sleep(remaining.min(Duration::from_millis(10)));
        }
    }
}

/// Iterator of login records
pub struct UtmpxIter {
    #[allow(dead_code)]
    guard: MutexGuard<'static, Option<PathBuf>>,
    /// Ensure [`UtmpxIter`] is !Send. Technically redundant because [`MutexGuard`]
    /// is also !Send.
    phantom: PhantomData<std::rc::Rc<()>>,
    #[cfg(all(target_os = "linux", feature = "systemd_logind"))]
    systemd_iter: Option<systemd_logind::SystemdUtmpxIter>,
    #[cfg(target_os = "linux")]
    traditional_file: Option<std::fs::File>,
    #[cfg(all(target_os = "linux", not(target_env = "musl")))]
    lock_wait_remaining: std::time::Duration,
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
            #[cfg(all(target_os = "linux", feature = "systemd_logind"))]
            systemd_iter: None,
            #[cfg(target_os = "linux")]
            traditional_file: None,
            #[cfg(all(target_os = "linux", not(target_env = "musl")))]
            lock_wait_remaining: UTMP_LOCK_TIMEOUT,
        }
    }

    #[cfg(target_os = "linux")]
    fn new_traditional(path: Option<&Path>) -> Self {
        let mut iter = Self::new();
        if let Some(path) = path {
            *iter.guard = Some(path.to_path_buf());
        }
        let path = iter.guard.as_deref().unwrap_or(Path::new(DEFAULT_FILE));
        #[cfg(feature = "systemd_logind")]
        if path == Path::new(DEFAULT_FILE) {
            iter.systemd_iter = Some(
                systemd_logind::SystemdUtmpxIter::new()
                    .unwrap_or_else(|_| systemd_logind::SystemdUtmpxIter::empty()),
            );
            return iter;
        }
        iter.traditional_file = open_traditional_utmp(path);
        iter
    }
}

#[cfg(all(test, target_os = "linux", not(target_env = "musl")))]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::mem::offset_of;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    use std::time::Duration;

    // Path selections persist after an iterator is dropped.
    static ITERATOR_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn utmpx_record(user: &str) -> Vec<u8> {
        let mut bytes = vec![0; size_of::<utmpx>()];
        let record_type = USER_PROCESS.to_ne_bytes();
        let type_offset = offset_of!(utmpx, ut_type);
        bytes[type_offset..type_offset + record_type.len()].copy_from_slice(&record_type);
        let user_offset = offset_of!(utmpx, ut_user);
        assert!(user.len() <= UT_NAMESIZE);
        bytes[user_offset..user_offset + user.len()].copy_from_slice(user.as_bytes());
        bytes
    }

    fn write_utmpx_records(path: &Path, users: &[&str]) {
        let bytes: Vec<u8> = users.iter().flat_map(|user| utmpx_record(user)).collect();
        std::fs::write(path, bytes).unwrap();
    }

    fn set_ofd_lock(file: &File, lock_type: libc::c_int) {
        // SAFETY: flock contains only integer fields, all initially zero.
        let mut lock: libc::flock = unsafe { std::mem::zeroed() };
        lock.l_type = lock_type as _;
        lock.l_whence = libc::SEEK_SET as _;
        // SAFETY: file owns a live descriptor and lock is initialized.
        let result = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_OFD_SETLK, &lock) };
        assert_eq!(result, 0, "{}", std::io::Error::last_os_error());
    }

    #[test]
    fn traditional_paths_persist_across_iterators() {
        let _guard = ITERATOR_TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let custom_path = directory.path().join("custom-utmp");
        let fallback_path = directory.path().join("fallback-utmp");
        write_utmpx_records(&custom_path, &["custom"]);
        // An incomplete trailing record must not produce another login entry.
        std::fs::OpenOptions::new()
            .append(true)
            .open(&custom_path)
            .unwrap()
            .write_all(&[0])
            .unwrap();
        write_utmpx_records(&fallback_path, &["fallback"]);

        let custom_users = Utmpx::iter_all_records_from(&custom_path)
            .map(|record| record.user())
            .collect::<Vec<_>>();
        let repeated_custom_users = Utmpx::iter_all_records()
            .map(|record| record.user())
            .collect::<Vec<_>>();
        let fallback_users = Utmpx::iter_all_records_from(&fallback_path)
            .map(|record| record.user())
            .collect::<Vec<_>>();
        let repeated_fallback_users = Utmpx::iter_all_records()
            .map(|record| record.user())
            .collect::<Vec<_>>();

        // Restore the default before the temporary files are removed.
        let default_iter = Utmpx::iter_all_records_from(DEFAULT_FILE);
        #[cfg(feature = "systemd_logind")]
        assert!(default_iter.systemd_iter.is_some());
        drop(default_iter);
        #[cfg(feature = "systemd_logind")]
        assert!(Utmpx::iter_all_records().systemd_iter.is_some());

        assert_eq!(custom_users, ["custom"]);
        assert_eq!(repeated_custom_users, custom_users);
        assert_eq!(fallback_users, ["fallback"]);
        assert_eq!(repeated_fallback_users, fallback_users);
    }

    #[test]
    fn traditional_utmp_opens_only_regular_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");

        assert!(open_traditional_utmp(&path).is_none());
        assert!(open_traditional_utmp(directory.path()).is_none());

        File::create(&path).unwrap();
        assert_eq!(
            open_traditional_utmp(&path)
                .unwrap()
                .metadata()
                .unwrap()
                .len(),
            0
        );

        write_utmpx_records(&path, &["user"]);
        assert_eq!(
            open_traditional_utmp(&path)
                .unwrap()
                .metadata()
                .unwrap()
                .len(),
            size_of::<utmpx>() as u64
        );
    }

    #[test]
    fn traditional_iterator_keeps_file_after_fifo_replacement() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let _guard = ITERATOR_TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        write_utmpx_records(&path, &["first", "second"]);
        let mut iter = Utmpx::iter_all_records_from(&path);

        std::fs::remove_file(&path).unwrap();
        let fifo_path = CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: fifo_path is a live, null-terminated path.
        assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);
        // Supply a different record so reopening the FIFO fails the assertions
        // without waiting indefinitely for a writer or for record data.
        let mut replacement = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&path)
            .unwrap();
        replacement.write_all(&utmpx_record("replacement")).unwrap();

        assert_eq!(iter.next().unwrap().user(), "first");
        assert_eq!(iter.next().unwrap().user(), "second");
        assert!(iter.next().is_none());
        drop(iter);
        drop(Utmpx::iter_all_records_from(DEFAULT_FILE));
    }

    #[test]
    fn traditional_lock_wait_budget_is_shared_across_records() {
        let _guard = ITERATOR_TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        write_utmpx_records(&path, &["first", "second"]);
        let writer = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        let mut iter = Utmpx::iter_all_records_from(&path);
        // Keep this test short; the who integration test checks the ten-second
        // production budget using a writer in another process.
        let budget = Duration::from_secs(2);
        iter.lock_wait_remaining = budget;

        // OFD locks conflict with the iterator's process-associated locks even
        // within this process, unlike two process-associated locks.
        set_ofd_lock(&writer, libc::F_WRLCK);
        std::thread::scope(|scope| {
            let unlocker = scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(500));
                set_ofd_lock(&writer, libc::F_UNLCK);
            });
            assert_eq!(iter.next().unwrap().user(), "first");
            unlocker.join().unwrap();
        });
        let remaining = iter.lock_wait_remaining;
        assert!(remaining < budget);
        assert!(!remaining.is_zero());

        set_ofd_lock(&writer, libc::F_WRLCK);
        std::thread::scope(|scope| {
            let unlocker = scope.spawn(|| {
                // Release eventually so a blocking or reset-per-record reader
                // returns the second record and fails instead of hanging.
                std::thread::sleep(remaining + Duration::from_secs(1));
                set_ofd_lock(&writer, libc::F_UNLCK);
            });
            assert!(iter.next().is_none());
            unlocker.join().unwrap();
        });
        assert!(iter.lock_wait_remaining.is_zero());
        assert!(iter.next().is_none());
        drop(iter);
        drop(Utmpx::iter_all_records_from(DEFAULT_FILE));
    }
}

/// Wrapper type that can hold either traditional utmpx records or systemd records
pub enum UtmpxRecord {
    Traditional(Box<Utmpx>),
    #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
    Systemd(systemd_logind::SystemdUtmpxCompat),
}

impl UtmpxRecord {
    /// A.K.A. `ut.ut_type`
    pub fn record_type(&self) -> i16 {
        match self {
            Self::Traditional(utmpx) => utmpx.record_type(),
            #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.record_type(),
        }
    }

    /// A.K.A. `ut.ut_pid`
    pub fn pid(&self) -> i32 {
        match self {
            Self::Traditional(utmpx) => utmpx.pid(),
            #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.pid(),
        }
    }

    /// A.K.A. `ut.ut_id`
    pub fn terminal_suffix(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.terminal_suffix(),
            #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.terminal_suffix(),
        }
    }

    /// A.K.A. `ut.ut_user`
    pub fn user(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.user(),
            #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.user(),
        }
    }

    /// A.K.A. `ut.ut_host`
    pub fn host(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.host(),
            #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.host(),
        }
    }

    /// A.K.A. `ut.ut_line`
    pub fn tty_device(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.tty_device(),
            #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.tty_device(),
        }
    }

    /// A.K.A. `ut.ut_tv`
    pub fn login_time(&self) -> time::OffsetDateTime {
        match self {
            Self::Traditional(utmpx) => utmpx.login_time(),
            #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.login_time(),
        }
    }

    /// A.K.A. `ut.ut_exit`
    ///
    /// Return (`e_termination`, `e_exit`)
    pub fn exit_status(&self) -> (i16, i16) {
        match self {
            Self::Traditional(utmpx) => utmpx.exit_status(),
            #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.exit_status(),
        }
    }

    /// check if the record is a user process
    pub fn is_user_process(&self) -> bool {
        match self {
            Self::Traditional(utmpx) => utmpx.is_user_process(),
            #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
            Self::Systemd(systemd) => systemd.is_user_process(),
        }
    }

    /// Canonicalize host name using DNS
    pub fn canon_host(&self) -> IOResult<String> {
        match self {
            Self::Traditional(utmpx) => utmpx.canon_host(),
            #[cfg(all(feature = "systemd_logind", target_os = "linux"))]
            Self::Systemd(systemd) => Ok(systemd.canon_host()),
        }
    }
}

impl Iterator for UtmpxIter {
    type Item = UtmpxRecord;
    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(target_os = "linux")]
        {
            #[cfg(feature = "systemd_logind")]
            if let Some(ref mut systemd_iter) = self.systemd_iter {
                // Use the selected systemd source exclusively.
                return systemd_iter.next().map(UtmpxRecord::Systemd);
            }
            #[cfg(not(target_env = "musl"))]
            {
                use std::io::Read;
                use std::os::fd::AsRawFd;

                let file = self.traditional_file.as_mut()?;
                // Cooperate with utmp writers while copying a complete record.
                // SAFETY: flock contains only integer fields, all initially zero.
                let mut lock: libc::flock = unsafe { std::mem::zeroed() };
                lock.l_type = libc::F_RDLCK as _;
                lock.l_whence = libc::SEEK_SET as _;
                let started = std::time::Instant::now();
                let acquired = lock_utmp(file, &lock, started + self.lock_wait_remaining);
                self.lock_wait_remaining =
                    self.lock_wait_remaining.saturating_sub(started.elapsed());
                if acquired.is_err() {
                    self.traditional_file = None;
                    return None;
                }
                let mut bytes = [0_u8; size_of::<utmpx>()];
                let result = file.read_exact(&mut bytes);
                lock.l_type = libc::F_UNLCK as _;
                // SAFETY: The descriptor and lock remain live through the unlock.
                let unlocked = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETLK, &lock) } >= 0;
                if result.is_err() || !unlocked {
                    self.traditional_file = None;
                    return None;
                }
                // SAFETY: Linux utmp files contain native utmpx records. Every
                // field is an integer or an integer array, so all bit patterns
                // are valid. read_unaligned does not require an aligned buffer.
                let inner = unsafe { ptr::read_unaligned(bytes.as_ptr().cast::<utmpx>()) };
                Some(UtmpxRecord::Traditional(Box::new(Utmpx { inner })))
            }
            #[cfg(target_env = "musl")]
            {
                None
            }
        }

        // Traditional utmp path
        #[cfg(not(target_os = "linux"))]
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

#[cfg(not(target_os = "linux"))]
impl Drop for UtmpxIter {
    fn drop(&mut self) {
        unsafe {
            #[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
            endutxent();
        }
    }
}
