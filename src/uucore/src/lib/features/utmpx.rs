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

use std::ffi::CString;
use std::io::Result as IOResult;
use std::marker::PhantomData;
#[cfg(target_os = "linux")]
use std::mem::size_of;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
#[cfg(not(all(target_os = "linux", target_env = "musl")))]
use std::ptr;
use std::sync::{Mutex, MutexGuard};

#[cfg(all(target_os = "linux", feature = "systemd-logind"))]
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

/// Optional session metadata to obtain when reading logind records.
/// The default requests both fields. Traditional utmp records and the boot
/// timestamp are unaffected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadOptions {
    /// Obtain a user session's login timestamp.
    pub session_time: bool,
    /// Obtain a user session's leader PID.
    pub session_pid: bool,
}

impl ReadOptions {
    /// Enumerate sessions without querying their login times or leader PIDs.
    pub const BASIC: Self = Self {
        session_time: false,
        session_pid: false,
    };
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            session_time: true,
            session_pid: true,
        }
    }
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
        local_login_time(time::OffsetDateTime::from_unix_timestamp_nanos(ts_nanos).unwrap())
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
        canonicalize_host(self.host())
    }

    /// Iterate through all utmp records using the most recently selected path.
    ///
    /// On Linux with `systemd-logind`, a default file without a complete record
    /// falls back to logind. Custom paths always select traditional records.
    /// Call [`Self::iter_all_records_from`] with [`DEFAULT_FILE`] to restore
    /// the default source selection rules.
    ///
    /// Only one iterator may be active at a time; this blocks until it is dropped.
    pub fn iter_all_records() -> UtmpxIter {
        Self::iter_all_records_with_options(ReadOptions::default())
    }

    /// Iterate through records, selecting which logind session details to obtain.
    /// The same locking and selected-path rules as [`Self::iter_all_records`] apply.
    pub fn iter_all_records_with_options(options: ReadOptions) -> UtmpxIter {
        let iter = UtmpxIter::new();
        #[cfg(target_os = "linux")]
        return iter.with_systemd_fallback(options);

        // SAFETY: The iterator holds LOCK while accessing libc's global utmp state.
        #[cfg(not(target_os = "linux"))]
        unsafe {
            #[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
            setutxent();
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = options;
            iter
        }
    }

    /// Read a specific file, remembering it for subsequent iterators.
    ///
    /// On Linux with `systemd-logind`, a default file without a complete record
    /// falls back to logind, then returns to the file if logind is unavailable.
    /// Other paths always select traditional records.
    ///
    /// Only one iterator may be active at a time; failures produce no records.
    pub fn iter_all_records_from<P: AsRef<Path>>(path: P) -> UtmpxIter {
        Self::iter_all_records_from_with_options(path, ReadOptions::default())
    }

    /// Read the selected file with explicit logind session metadata requirements.
    /// The same rules as [`Self::iter_all_records_from`] apply.
    pub fn iter_all_records_from_with_options<P: AsRef<Path>>(
        path: P,
        options: ReadOptions,
    ) -> UtmpxIter {
        let iter = UtmpxIter::new();
        #[cfg(target_os = "linux")]
        {
            let mut iter = iter;
            *iter.guard = Some(path.as_ref().to_path_buf());
            iter.with_systemd_fallback(options)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let filename = CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();
            // SAFETY: The iterator holds LOCK, and filename remains valid for these calls.
            unsafe {
                #[cfg(target_os = "freebsd")]
                setutxdb(libc::UTXDB_ACTIVE, filename.as_ptr());

                // These APIs report failures inconsistently across platforms.
                // Preserve the traditional reader's empty output on invalid paths.
                #[cfg(not(target_os = "freebsd"))]
                #[cfg_attr(target_env = "ohos", allow(deprecated))]
                utmpxname(filename.as_ptr());

                #[cfg_attr(target_env = "ohos", allow(deprecated))]
                setutxent();
            }
            let _ = options;
            iter
        }
    }
}

/// Apply the local UTC offset at the record's timestamp, including historical DST.
pub(crate) fn local_login_time(datetime: time::OffsetDateTime) -> time::OffsetDateTime {
    let offset = time::UtcOffset::local_offset_at(datetime).unwrap_or(time::UtcOffset::UTC);
    datetime.to_offset(offset)
}

pub(crate) fn canonicalize_host(host: String) -> IOResult<String> {
    use dns_lookup::{AddrInfoHints, getaddrinfo};

    const AI_CANONNAME: i32 = 0x2;
    let hints = AddrInfoHints {
        flags: AI_CANONNAME,
        ..AddrInfoHints::default()
    };
    canonicalize_host_with(host, |hostname| {
        getaddrinfo(Some(hostname), None, Some(hints))
            .map(|sockets| sockets.map(|socket| socket.map(|info| info.canonname)))
            .map_err(|error| std::io::Error::other(error.to_string()))
    })
}

fn canonicalize_host_with<I>(
    host: String,
    lookup: impl FnOnce(&str) -> IOResult<I>,
) -> IOResult<String>
where
    I: IntoIterator<Item = IOResult<Option<String>>>,
{
    let (hostname, display) = split_host_display(&host);
    if !hostname.is_empty() {
        if let Ok(sockets) = lookup(hostname) {
            let sockets = sockets.into_iter().collect::<IOResult<Vec<_>>>()?;
            if let Some(ai_canonname) = sockets.into_iter().flatten().next() {
                return Ok(if display.is_empty() {
                    ai_canonname
                } else {
                    format!("{ai_canonname}:{display}")
                });
            }
        } else {
            return Ok(hostname.to_string());
        }
    }
    Ok(host)
}

fn split_host_display(host: &str) -> (&str, &str) {
    // Colons inside an IPv6 address are not X11 display separators.
    let address = host
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
        .map_or(host, |(address, _)| address);
    let address = address
        .split_once('%')
        .map_or(address, |(address, _)| address);
    if address.parse::<std::net::Ipv6Addr>().is_ok() {
        return (host, "");
    }
    host.split_once(':').unwrap_or((host, ""))
}

// Serialize access to libc's utmp state and the selected path. UtmpxIter must
// stay on its creating thread because libc's utmp state may be thread-local.
static LOCK: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Open a regular file without waiting for FIFO writers. Read this descriptor
/// directly so all record-lock waits can share the iterator's budget.
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
        regular_utmp(file)
    }
}

#[cfg(all(target_os = "linux", not(target_env = "musl")))]
fn regular_utmp(file: std::fs::File) -> Option<std::fs::File> {
    file.metadata().ok()?.is_file().then_some(file)
}

// Bound the total time spent acquiring record locks across one iterator.
// A per-record timeout can otherwise be spent again for every entry.
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
    /// Ensure [`UtmpxIter`] is !Send. Technically redundant because [`MutexGuard`]
    /// is also !Send.
    phantom: PhantomData<std::rc::Rc<()>>,
    #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
    systemd_iter: Option<systemd_logind::SystemdUtmpxIter>,
    #[cfg(target_os = "linux")]
    traditional_file: Option<std::fs::File>,
    #[cfg(all(target_os = "linux", not(target_env = "musl")))]
    lock_wait_remaining: std::time::Duration,
    #[cfg(all(test, target_os = "linux", not(target_env = "musl")))]
    drop_observer: Option<tests::DropObserver>,
    // Close every file before another iterator can acquire process-wide locks.
    #[allow(dead_code)]
    guard: MutexGuard<'static, Option<PathBuf>>,
}

impl UtmpxIter {
    fn new() -> Self {
        // PoisonErrors can safely be ignored
        let guard = LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Self {
            phantom: PhantomData,
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            systemd_iter: None,
            #[cfg(target_os = "linux")]
            traditional_file: None,
            #[cfg(all(target_os = "linux", not(target_env = "musl")))]
            lock_wait_remaining: UTMP_LOCK_TIMEOUT,
            #[cfg(all(test, target_os = "linux", not(target_env = "musl")))]
            drop_observer: None,
            guard,
        }
    }

    #[cfg(target_os = "linux")]
    fn with_systemd_fallback(self, options: ReadOptions) -> Self {
        #[cfg(feature = "systemd-logind")]
        {
            self.with_login_reader(
                options,
                Path::new(DEFAULT_FILE),
                systemd_logind::SystemdUtmpxIter::with_options,
            )
        }
        #[cfg(not(feature = "systemd-logind"))]
        {
            let _ = options;
            let mut iter = self;
            let path = iter.guard.as_deref().unwrap_or(Path::new(DEFAULT_FILE));
            iter.traditional_file = open_traditional_utmp(path);
            iter
        }
    }

    #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
    fn with_login_reader(
        mut self,
        options: ReadOptions,
        default_path: &Path,
        read_logind: impl FnOnce(ReadOptions) -> crate::error::UResult<systemd_logind::SystemdUtmpxIter>,
    ) -> Self {
        let path = self.guard.as_deref().unwrap_or(default_path);
        self.traditional_file = open_traditional_utmp(path);
        // Availability, not record contents, determines which source wins.
        let usable = self
            .traditional_file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .is_some_and(|metadata| metadata.len() >= size_of::<utmpx>() as u64);
        if path == default_path && !usable {
            self.systemd_iter = read_logind(options).ok();
            self.traditional_file = if self.systemd_iter.is_none() {
                // The file may have appeared while logind was being tried.
                open_traditional_utmp(path)
            } else {
                None
            };
        }
        self
    }
}

#[cfg(all(test, target_os = "linux", not(target_env = "musl")))]
mod tests {
    use super::*;
    #[cfg(feature = "systemd-logind")]
    use std::cell::Cell;
    use std::fs::File;
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
    use std::time::Duration;

    // The selected utmp path persists between iterators. Keep tests that use
    // several iterators together so another test cannot replace that path.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    pub(super) struct DropObserver(Box<dyn Fn()>);

    impl Drop for DropObserver {
        fn drop(&mut self) {
            (self.0)();
        }
    }

    mod support {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/utmpx.rs"
        ));
    }
    use support::utmpx_record;

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
    fn traditional_iteration_reads_complete_records() {
        let _guard = TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        write_utmpx_records(&path, &["first", "second"]);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(&[0])
            .unwrap();

        let users = Utmpx::iter_all_records_from(&path)
            .map(|record| record.user())
            .collect::<Vec<_>>();
        drop(Utmpx::iter_all_records_from(DEFAULT_FILE));
        assert_eq!(users, ["first", "second"]);
    }

    #[test]
    fn traditional_paths_persist_across_iterators() {
        let _test_lock = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
            .map(|record| {
                let pid: i32 = record.pid();
                let timestamp: time::OffsetDateTime = record.login_time();
                assert_eq!(record.try_pid(), Some(pid));
                assert_eq!(record.try_login_time(), Some(timestamp));
                record.user()
            })
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
        assert_eq!(default_iter.guard.as_deref(), Some(Path::new(DEFAULT_FILE)));
        drop(default_iter);
        assert_eq!(
            Utmpx::iter_all_records().guard.as_deref(),
            Some(Path::new(DEFAULT_FILE))
        );

        assert_eq!(custom_users, ["custom"]);
        assert_eq!(repeated_custom_users, custom_users);
        assert_eq!(fallback_users, ["fallback"]);
        assert_eq!(repeated_fallback_users, fallback_users);
    }

    #[test]
    fn traditional_iterator_closes_file_before_unlocking() {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::MetadataExt;

        let _guard = TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        write_utmpx_records(&path, &["user"]);
        let mut iter = Utmpx::iter_all_records_from(&path);
        let file = iter.traditional_file.as_ref().unwrap();
        let fd_path = format!("/proc/self/fd/{}", file.as_raw_fd());
        let metadata = file.metadata().unwrap();
        let identity = (metadata.dev(), metadata.ino());

        // Observe destruction after the file field, before the mutex guard.
        iter.drop_observer = Some(DropObserver(Box::new(move || {
            let still_open = std::fs::metadata(&fd_path)
                .is_ok_and(|metadata| (metadata.dev(), metadata.ino()) == identity);
            assert!(!still_open, "utmp descriptor must be closed first");
            assert!(
                matches!(LOCK.try_lock(), Err(std::sync::TryLockError::WouldBlock)),
                "utmp mutex must remain locked while closing the descriptor"
            );
        })));
        drop(iter);
        drop(Utmpx::iter_all_records_from(DEFAULT_FILE));
    }

    #[test]
    fn traditional_iterator_closes_all_files() {
        use std::os::unix::fs::MetadataExt;

        let _guard = TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        write_utmpx_records(&path, &["user"]);
        let mut iter = Utmpx::iter_all_records_from(&path);
        let file = iter.traditional_file.as_ref().unwrap();
        let metadata = file.metadata().unwrap();
        let identity = (metadata.dev(), metadata.ino());
        // Read a record before checking that iteration leaves no descriptor open.
        assert_eq!(iter.next().unwrap().user(), "user");
        drop(iter);

        let still_open = std::fs::read_dir("/proc/self/fd")
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| {
                std::fs::metadata(entry.path())
                    .is_ok_and(|metadata| (metadata.dev(), metadata.ino()) == identity)
            });
        drop(Utmpx::iter_all_records_from(DEFAULT_FILE));
        assert!(!still_open, "all utmp descriptors must be closed");
    }

    #[test]
    fn traditional_iterator_restores_selected_path_for_libc() {
        let _guard = TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        write_utmpx_records(&path, &["first", "second"]);
        let mut iter = Utmpx::iter_all_records_from(&path);
        assert_eq!(iter.next().unwrap().user(), "first");
        drop(iter);

        let user = {
            let _guard = LOCK.lock().unwrap();
            // SAFETY: LOCK serializes libc's utmp state and the returned record
            // is copied before another libc call can replace its buffer.
            unsafe {
                setutxent();
                let record = getutxent();
                let user = if record.is_null() {
                    None
                } else {
                    Some(
                        Utmpx {
                            inner: ptr::read(record),
                        }
                        .user(),
                    )
                };
                endutxent();
                user
            }
        };
        drop(Utmpx::iter_all_records_from(DEFAULT_FILE));
        assert_eq!(user.as_deref(), Some("first"));
    }

    #[test]
    fn traditional_iterator_reopens_selected_path() {
        let _guard = TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        let replacement_path = directory.path().join("replacement");
        write_utmpx_records(&path, &["original"]);
        write_utmpx_records(&replacement_path, &["replacement"]);
        let users = Utmpx::iter_all_records_from(&path)
            .map(|record| record.user())
            .collect::<Vec<_>>();

        // A new iterator must resolve the saved pathname again, rather than
        // retaining the previous iterator's open descriptor.
        let _old_file = File::open(&path).unwrap();
        std::fs::rename(&replacement_path, &path).unwrap();
        let replacement_users = Utmpx::iter_all_records()
            .map(|record| record.user())
            .collect::<Vec<_>>();
        drop(Utmpx::iter_all_records_from(DEFAULT_FILE));
        assert_eq!(users, ["original"]);
        assert_eq!(replacement_users, ["replacement"]);
    }

    #[test]
    fn traditional_iterator_rejects_invalid_source_after_valid_file() {
        let _guard = TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        write_utmpx_records(&path, &["first", "second"]);

        for invalid_path in [
            directory.path().join("missing"),
            directory.path().to_owned(),
        ] {
            let mut iter = Utmpx::iter_all_records_from(&path);
            assert_eq!(iter.next().unwrap().user(), "first");
            drop(iter);
            let mut iter = Utmpx::iter_all_records_from(invalid_path);
            assert!(iter.next().is_none());
            assert!(iter.next().is_none());
        }
        drop(Utmpx::iter_all_records_from(DEFAULT_FILE));
    }

    #[test]
    fn traditional_utmp_opens_only_regular_files() {
        let _test_lock = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    fn replacing_an_open_file_with_a_fifo_keeps_the_original_record() {
        let _test_lock = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        write_utmpx_records(&path, &["original"]);
        let file = File::open(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        nix::unistd::mkfifo(&path, nix::sys::stat::Mode::S_IRUSR).unwrap();
        assert!(path.metadata().unwrap().file_type().is_fifo());

        // Validate after the replacement, just as the reader does after open.
        let mut iter = UtmpxIter::new();
        *iter.guard = None;
        iter.traditional_file = regular_utmp(file);
        assert_eq!(iter.next().unwrap().user(), "original");
        assert!(iter.next().is_none());
    }

    #[cfg(feature = "systemd-logind")]
    #[rstest::rstest]
    #[case("missing", true)]
    #[case("empty", true)]
    #[case("short", true)]
    #[case("fifo", true)]
    #[case("traditional", false)]
    #[case("zeros", false)]
    #[case("boot-only", false)]
    fn default_utmp_source_selection(#[case] contents: &str, #[case] logind: bool) {
        let _test_lock = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        match contents {
            "missing" => {}
            "empty" => {
                File::create(&path).unwrap();
            }
            "short" => std::fs::write(&path, [0]).unwrap(),
            "fifo" => nix::unistd::mkfifo(&path, nix::sys::stat::Mode::S_IRUSR).unwrap(),
            "zeros" => File::create(&path)
                .unwrap()
                .set_len(size_of::<utmpx>() as u64)
                .unwrap(),
            _ => {
                write_utmpx_records(&path, &["traditional"]);
                if contents == "boot-only" {
                    File::options()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .write_all(&BOOT_TIME.to_ne_bytes())
                        .unwrap();
                }
            }
        }
        let called = Cell::new(false);
        let mut iter = UtmpxIter::new();
        *iter.guard = None;
        let mut iter = iter.with_login_reader(ReadOptions::BASIC, &path, |options| {
            called.set(true);
            assert_eq!(options, ReadOptions::BASIC);
            Ok(systemd_logind::SystemdUtmpxIter::empty())
        });
        assert_eq!(called.get(), logind);
        assert_eq!(iter.systemd_iter.is_some(), logind);
        assert_eq!(iter.traditional_file.is_none(), logind);
        if contents == "traditional" {
            assert_eq!(iter.next().unwrap().user(), "traditional");
        }
    }

    #[cfg(feature = "systemd-logind")]
    #[test]
    fn explicit_utmp_path_does_not_try_logind() {
        let _test_lock = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("explicit-utmp");
        File::create(&path).unwrap();
        let mut iter = UtmpxIter::new();
        *iter.guard = Some(path);
        let mut iter = iter.with_login_reader(
            ReadOptions::default(),
            &directory.path().join("default-utmp"),
            |_| panic!("an explicit non-default file must not try logind"),
        );
        assert!(iter.next().is_none());
        *iter.guard = None;
    }

    #[cfg(feature = "systemd-logind")]
    #[rstest::rstest]
    #[case(false)]
    #[case(true)]
    fn failed_logind_reopens_utmp(#[case] initially_exists: bool) {
        let _test_lock = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        if initially_exists {
            File::create(&path).unwrap();
        }
        let mut iter = UtmpxIter::new();
        *iter.guard = None;
        let mut iter = iter.with_login_reader(ReadOptions::default(), &path, |_| {
            let replacement = directory.path().join("replacement");
            write_utmpx_records(&replacement, &["fallback"]);
            std::fs::rename(&replacement, &path).unwrap();
            Err(crate::error::USimpleError::new(1, "logind unavailable"))
        });
        assert!(iter.systemd_iter.is_none());
        assert_eq!(iter.next().unwrap().user(), "fallback");
        assert!(iter.next().is_none());
    }

    #[cfg(feature = "systemd-logind")]
    #[rstest::rstest]
    #[case(false)]
    #[case(true)]
    fn failed_logind_without_a_regular_utmp_is_empty(#[case] fifo: bool) {
        let _test_lock = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        if fifo {
            nix::unistd::mkfifo(&path, nix::sys::stat::Mode::S_IRUSR).unwrap();
        }
        let mut iter = UtmpxIter::new();
        *iter.guard = None;
        let mut iter = iter.with_login_reader(ReadOptions::default(), &path, |_| {
            Err(crate::error::USimpleError::new(1, "logind unavailable"))
        });
        assert!(iter.next().is_none());
    }

    #[test]
    fn traditional_iterator_keeps_file_after_fifo_replacement() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let _test_lock = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        let _guard = TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        write_utmpx_records(&path, &["first", "second"]);
        let writer = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        let mut iter = Utmpx::iter_all_records_from(&path);
        // Exercise the production wait policy with a short private budget.
        let budget = Duration::from_millis(500);
        iter.lock_wait_remaining = budget;

        // OFD locks conflict with the iterator's process-associated locks even
        // within this process, unlike two process-associated locks.
        set_ofd_lock(&writer, libc::F_WRLCK);
        std::thread::scope(|scope| {
            let unlocker = scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(50));
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
                std::thread::sleep(remaining + Duration::from_millis(100));
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

    #[test]
    fn exhausted_wait_budget_still_reads_uncontended_records() {
        let _guard = TEST_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("utmp");
        write_utmpx_records(&path, &["first", "second"]);
        let mut iter = Utmpx::iter_all_records_from(&path);
        iter.lock_wait_remaining = Duration::ZERO;

        assert_eq!(iter.next().unwrap().user(), "first");
        assert_eq!(iter.next().unwrap().user(), "second");
        assert!(iter.next().is_none());
        drop(iter);
        drop(Utmpx::iter_all_records_from(DEFAULT_FILE));
    }
}

/// Wrapper type that can hold either traditional utmpx records or systemd records
pub enum UtmpxRecord {
    Traditional(Box<Utmpx>),
    #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
    Systemd(systemd_logind::SystemdUtmpxCompat),
}

impl UtmpxRecord {
    /// A.K.A. `ut.ut_type`
    pub fn record_type(&self) -> i16 {
        match self {
            Self::Traditional(utmpx) => utmpx.record_type(),
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            Self::Systemd(systemd) => systemd.record_type(),
        }
    }

    /// A.K.A. `ut.ut_pid`
    /// Returns None when logind could not provide a session leader PID.
    #[cfg_attr(
        not(all(target_os = "linux", feature = "systemd-logind")),
        allow(clippy::unnecessary_wraps)
    )]
    pub fn try_pid(&self) -> Option<i32> {
        match self {
            Self::Traditional(utmpx) => Some(utmpx.pid()),
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            Self::Systemd(systemd) => systemd.try_pid(),
        }
    }

    /// A.K.A. `ut.ut_pid`. Returns zero when a logind PID is unavailable.
    /// Use [`Self::try_pid`] to distinguish an unavailable PID.
    pub fn pid(&self) -> i32 {
        self.try_pid().unwrap_or_default()
    }

    /// A.K.A. `ut.ut_id`
    pub fn terminal_suffix(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.terminal_suffix(),
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            Self::Systemd(systemd) => systemd.terminal_suffix(),
        }
    }

    /// A.K.A. `ut.ut_user`
    pub fn user(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.user(),
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            Self::Systemd(systemd) => systemd.user(),
        }
    }

    /// A.K.A. `ut.ut_host`
    pub fn host(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.host(),
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            Self::Systemd(systemd) => systemd.host(),
        }
    }

    /// A.K.A. `ut.ut_line`
    pub fn tty_device(&self) -> String {
        match self {
            Self::Traditional(utmpx) => utmpx.tty_device(),
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            Self::Systemd(systemd) => systemd.tty_device(),
        }
    }

    /// A.K.A. `ut.ut_tv`
    /// Returns None when a systemd record has no known login time.
    #[cfg_attr(
        not(all(target_os = "linux", feature = "systemd-logind")),
        allow(clippy::unnecessary_wraps)
    )]
    pub fn try_login_time(&self) -> Option<time::OffsetDateTime> {
        match self {
            Self::Traditional(utmpx) => Some(utmpx.login_time()),
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            Self::Systemd(systemd) => systemd.try_login_time(),
        }
    }

    /// A.K.A. `ut.ut_tv`. Returns the Unix epoch when a logind time is unavailable.
    /// Use [`Self::try_login_time`] when displaying a login time to users.
    pub fn login_time(&self) -> time::OffsetDateTime {
        self.try_login_time()
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
    }

    /// A.K.A. `ut.ut_exit`
    ///
    /// Return (`e_termination`, `e_exit`)
    pub fn exit_status(&self) -> (i16, i16) {
        match self {
            Self::Traditional(utmpx) => utmpx.exit_status(),
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            Self::Systemd(systemd) => systemd.exit_status(),
        }
    }

    /// check if the record is a user process
    pub fn is_user_process(&self) -> bool {
        match self {
            Self::Traditional(utmpx) => utmpx.is_user_process(),
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            Self::Systemd(systemd) => systemd.is_user_process(),
        }
    }

    /// Canonicalize host name using DNS
    pub fn canon_host(&self) -> IOResult<String> {
        match self {
            Self::Traditional(utmpx) => utmpx.canon_host(),
            #[cfg(all(target_os = "linux", feature = "systemd-logind"))]
            Self::Systemd(systemd) => systemd.try_canon_host(),
        }
    }
}

impl Iterator for UtmpxIter {
    type Item = UtmpxRecord;
    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(target_os = "linux")]
        {
            #[cfg(feature = "systemd-logind")]
            if let Some(ref mut systemd_iter) = self.systemd_iter {
                // Once a systemd iterator was successfully created, use it exclusively.
                // If systemd initialization failed, `systemd_iter` is None and we use
                // traditional utmp below.
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
                    // Closing also releases the lock if the explicit unlock failed.
                    self.traditional_file = None;
                }
                // An unlock failure must not discard a record already read.
                result.ok()?;
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

impl Drop for UtmpxIter {
    fn drop(&mut self) {
        // SAFETY: The iterator still holds LOCK while closing libc's file.
        unsafe {
            #[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
            endutxent();
        }

        #[cfg(target_os = "linux")]
        {
            // Preserve the selected pathname for callers of the re-exported libc API.
            let path = self.guard.as_deref().unwrap_or(Path::new(DEFAULT_FILE));
            if let Ok(path) = CString::new(path.as_os_str().as_bytes()) {
                // SAFETY: LOCK is held and path is a live, null-terminated path.
                #[cfg_attr(any(target_env = "musl", target_env = "ohos"), allow(deprecated))]
                unsafe {
                    utmpxname(path.as_ptr());
                }
            }
        }
    }
}


#[cfg(test)]
mod host_tests {
    use super::{canonicalize_host_with, split_host_display};

    #[rstest::rstest]
    #[case("alias.test", "alias.test", "canonical.test")]
    #[case("alias.test:0", "alias.test", "canonical.test:0")]
    #[case("192.0.2.1", "192.0.2.1", "canonical.test")]
    #[case("dead:beef::1", "dead:beef::1", "canonical.test")]
    #[case("[dead:beef::1]:0", "[dead:beef::1]:0", "canonical.test")]
    fn canonical_names_preserve_display_suffixes(
        #[case] host: &str,
        #[case] lookup: &str,
        #[case] expected: &str,
    ) {
        let result = canonicalize_host_with(host.into(), |name| {
            assert_eq!(name, lookup);
            Ok([Ok(None), Ok(Some("canonical.test".into()))])
        })
        .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn addresses_without_a_canonical_name_are_preserved() {
        for host in ["192.0.2.1", "dead:beef::1", "[dead:beef::1]:0"] {
            assert_eq!(
                canonicalize_host_with(host.into(), |_| Ok([Ok(None)])).unwrap(),
                host
            );
        }
    }

    #[rstest::rstest]
    #[case("192.0.2.1", "192.0.2.1", "")]
    #[case("dead:beef::1", "dead:beef::1", "")]
    #[case("fe80::1%eth0", "fe80::1%eth0", "")]
    #[case("[dead:beef::1]:0", "[dead:beef::1]:0", "")]
    #[case("host.example", "host.example", "")]
    #[case("host.example:0.1", "host.example", "0.1")]
    #[case(":0", "", "0")]
    fn ipv6_colons_are_not_display_separators(
        #[case] host: &str,
        #[case] hostname: &str,
        #[case] display: &str,
    ) {
        assert_eq!(split_host_display(host), (hostname, display));
    }
}
