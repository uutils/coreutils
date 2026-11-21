// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore logind libsystemd btime unref RAII testuser GETPW sysconf

//! Systemd-logind support for reading login records
//!
//! This module provides systemd-logind based implementation for reading
//! login records as an alternative to traditional utmp/utmpx files.
//! When the systemd-logind feature is enabled and systemd is available,
//! this will be used instead of traditional utmp files.

use core::ffi::CStr;
use std::mem::MaybeUninit;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{UResult, USimpleError};

/// Dynamically loaded FFI bindings for libsystemd login functions.
mod ffi {
    use core::ffi::{c_char, c_int, c_uint};
    use libloading::Library;
    use std::path::Path;

    type SdGetSessions = unsafe extern "C" fn(*mut *mut *mut c_char) -> c_int;
    type SdSessionGetUid = unsafe extern "C" fn(*const c_char, *mut c_uint) -> c_int;
    type SdSessionGetStartTime = unsafe extern "C" fn(*const c_char, *mut u64) -> c_int;
    type SdSessionGetLeader = unsafe extern "C" fn(*const c_char, *mut c_int) -> c_int;
    type SdSessionGetString = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_int;

    pub(super) struct SystemdLoginApi {
        // The library must remain loaded while any of these function pointers are used.
        _library: Library,
        pub(super) sd_get_sessions: SdGetSessions,
        pub(super) sd_session_get_uid: SdSessionGetUid,
        pub(super) sd_session_get_start_time: Option<SdSessionGetStartTime>,
        pub(super) sd_session_get_leader: Option<SdSessionGetLeader>,
        pub(super) sd_session_get_tty: SdSessionGetString,
        pub(super) sd_session_get_remote_host: SdSessionGetString,
        pub(super) sd_session_get_display: SdSessionGetString,
        pub(super) sd_session_get_seat: SdSessionGetString,
    }

    impl SystemdLoginApi {
        pub(super) fn load() -> Result<Self, Box<dyn std::error::Error>> {
            Self::load_from(Path::new("libsystemd.so.0"))
        }

        fn load_from(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
            // SAFETY: The loaded symbols use the signatures declared by libsystemd's
            // stable public API. The Library is retained in Self for at least as long
            // as the copied function pointers can be called.
            unsafe {
                let library = Library::new(path)?;
                let sd_get_sessions = *library.get::<SdGetSessions>(b"sd_get_sessions\0")?;
                let sd_session_get_uid =
                    *library.get::<SdSessionGetUid>(b"sd_session_get_uid\0")?;
                // This function was added in systemd 254; older libraries can
                // still provide the rest of the session information.
                let sd_session_get_start_time = library
                    .get::<SdSessionGetStartTime>(b"sd_session_get_start_time\0")
                    .ok()
                    .map(|symbol| *symbol);
                let sd_session_get_leader = library
                    .get::<SdSessionGetLeader>(b"sd_session_get_leader\0")
                    .ok()
                    .map(|symbol| *symbol);
                let sd_session_get_tty =
                    *library.get::<SdSessionGetString>(b"sd_session_get_tty\0")?;
                let sd_session_get_remote_host =
                    *library.get::<SdSessionGetString>(b"sd_session_get_remote_host\0")?;
                let sd_session_get_display =
                    *library.get::<SdSessionGetString>(b"sd_session_get_display\0")?;
                let sd_session_get_seat =
                    *library.get::<SdSessionGetString>(b"sd_session_get_seat\0")?;

                Ok(Self {
                    _library: library,
                    sd_get_sessions,
                    sd_session_get_uid,
                    sd_session_get_start_time,
                    sd_session_get_leader,
                    sd_session_get_tty,
                    sd_session_get_remote_host,
                    sd_session_get_display,
                    sd_session_get_seat,
                })
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::SystemdLoginApi;
        use std::path::Path;

        #[test]
        fn missing_library_is_reported() {
            assert!(
                SystemdLoginApi::load_from(Path::new("libsystemd-uutils-does-not-exist.so"))
                    .is_err()
            );
        }

        #[test]
        fn invalid_library_is_reported() {
            let library = tempfile::NamedTempFile::new().unwrap();
            assert!(SystemdLoginApi::load_from(library.path()).is_err());
        }

        #[test]
        #[cfg(all(target_env = "gnu", not(target_feature = "crt-static")))]
        fn missing_login_symbols_are_reported() {
            let error = SystemdLoginApi::load_from(Path::new("libc.so.6"))
                .err()
                .unwrap();
            assert!(error.to_string().contains("sd_get_sessions"), "{error}");
        }
    }
}

/// Safe wrapper functions for libsystemd FFI calls
mod login {
    use super::ffi;
    use core::ffi::{CStr, c_char, c_int};
    use std::ffi::CString;
    use std::ptr;
    use std::time::SystemTime;

    /// Get all active sessions
    pub fn get_sessions(
        api: &ffi::SystemdLoginApi,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut sessions_ptr: *mut *mut c_char = ptr::null_mut();

        // SAFETY: api retains the library, and sessions_ptr is a writable output.
        let result = unsafe { (api.sd_get_sessions)(&raw mut sessions_ptr) };

        if result < 0 {
            return Err(format!("sd_get_sessions failed: {result}").into());
        }

        let mut sessions = Vec::new();
        if !sessions_ptr.is_null() {
            let mut i = 0;
            loop {
                let session_ptr = unsafe { *sessions_ptr.add(i) };
                if session_ptr.is_null() {
                    break;
                }

                let session_cstr = unsafe { CStr::from_ptr(session_ptr) };
                sessions.push(session_cstr.to_string_lossy().into_owned());

                unsafe { libc::free(session_ptr.cast()) };
                i += 1;
            }

            unsafe { libc::free(sessions_ptr.cast()) };
        }

        Ok(sessions)
    }

    /// Get UID for a session
    pub fn get_session_uid(
        api: &ffi::SystemdLoginApi,
        session_id: &str,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut uid: core::ffi::c_uint = 0;

        // SAFETY: api retains the library; the session is NUL-terminated and uid is writable.
        let result = unsafe { (api.sd_session_get_uid)(session_cstring.as_ptr(), &raw mut uid) };

        if result < 0 {
            return Err(
                format!("sd_session_get_uid failed for session '{session_id}': {result}").into(),
            );
        }

        Ok(uid)
    }

    /// Get the session's start time in microseconds since the Unix epoch.
    pub fn get_session_start_time(api: &ffi::SystemdLoginApi, session_id: &str) -> Option<u64> {
        let get_start_time = api.sd_session_get_start_time?;
        let session_cstring = CString::new(session_id).ok()?;
        let mut usec: u64 = 0;

        // SAFETY: api retains the library; the session is NUL-terminated and usec is writable.
        let result = unsafe { get_start_time(session_cstring.as_ptr(), &raw mut usec) };

        (result >= 0).then_some(usec)
    }

    pub fn get_session_leader(api: &ffi::SystemdLoginApi, session_id: &str) -> Option<u32> {
        let get_leader = api.sd_session_get_leader?;
        let session = CString::new(session_id).ok()?;
        let mut leader = 0;
        // SAFETY: session is NUL-terminated and leader is a writable pid_t.
        let result = unsafe { get_leader(session.as_ptr(), &raw mut leader) };
        (result >= 0 && leader > 0).then_some(leader as u32)
    }

    /// Read a string property of a session, such as its TTY or seat, with one of
    /// the `sd_session_get_*` getters. An error or an unset property gives an
    /// empty string.
    pub fn get_session_string(
        session_id: &str,
        getter: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_int,
    ) -> String {
        let Ok(session_cstring) = CString::new(session_id) else {
            return String::new();
        };
        let mut value_ptr: *mut c_char = ptr::null_mut();

        // SAFETY: `session_cstring` is NUL-terminated and outlives the call; the
        // getter only stores a pointer in `value_ptr`.
        let result = unsafe { getter(session_cstring.as_ptr(), &raw mut value_ptr) };
        if result < 0 || value_ptr.is_null() {
            return String::new();
        }

        // SAFETY: on success `value_ptr` is a NUL-terminated string allocated by
        // libsystemd, which we own and free once copied.
        let value = unsafe { CStr::from_ptr(value_ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { libc::free(value_ptr.cast()) };
        value
    }

    /// Get system boot time using systemd random-seed file fallback
    ///
    /// TODO: This replicates GNU coreutils' fallback behavior for compatibility.
    /// GNU coreutils uses the mtime of /var/lib/systemd/random-seed as a heuristic for boot time
    /// when utmp is unavailable, rather than querying systemd's authoritative `KernelTimestamp`.
    /// This creates inconsistency: `uptime -s` shows the actual kernel boot time
    /// while `who -b` shows ~1 minute later when systemd services start.
    ///
    /// Ideally, both should use the same source (`KernelTimestamp`) for semantic consistency.
    /// Consider proposing to GNU coreutils to use systemd's `KernelTimestamp` property instead.
    pub fn get_boot_time() -> Result<SystemTime, Box<dyn std::error::Error>> {
        use std::fs;

        let metadata = fs::metadata("/var/lib/systemd/random-seed")
            .map_err(|e| format!("Failed to read /var/lib/systemd/random-seed: {e}"))?;

        metadata
            .modified()
            .map_err(|e| format!("Failed to get modification time: {e}").into())
    }
}

/// Login record compatible with utmpx structure
#[derive(Debug, Clone)]
pub struct SystemdLoginRecord {
    pub user: String,
    pub session_id: String,
    pub seat_or_tty: String,
    pub raw_device: String,
    pub host: String,
    /// The login timestamp, if available.
    pub login_time: Option<SystemTime>,
    pub pid: u32,
    pub session_leader_pid: u32,
    pub record_type: SystemdRecordType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemdRecordType {
    UserProcess = 7,  // USER_PROCESS
    LoginProcess = 6, // LOGIN_PROCESS
    BootTime = 2,     // BOOT_TIME
}

impl SystemdLoginRecord {
    /// Check if this is a user process record
    pub fn is_user_process(&self) -> bool {
        !self.user.is_empty() && self.record_type == SystemdRecordType::UserProcess
    }
}

/// A logind session with no login time or session leader PID.
#[cfg(any(test, feature = "systemd_logind_test"))]
#[doc(hidden)]
pub fn test_session_record() -> SystemdLoginRecord {
    SystemdLoginRecord {
        user: "user".into(),
        session_id: "legacy".into(),
        seat_or_tty: "pts/test".into(),
        raw_device: "pts/test".into(),
        host: "alias.test".into(),
        login_time: None,
        pid: 0,
        session_leader_pid: 0,
        record_type: SystemdRecordType::UserProcess,
    }
}

/// Read login records from systemd-logind using safe wrapper functions
/// This matches the approach used by GNU coreutils `read_utmp_from_systemd()`
pub fn read_login_records() -> UResult<Vec<SystemdLoginRecord>> {
    let api = ffi::SystemdLoginApi::load().map_err(|e| {
        USimpleError::new(
            1,
            crate::translate!("systemd-logind-error-load-library", "error" => e),
        )
    })?;
    read_login_records_with_api(&api)
}

#[derive(Clone, Default)]
struct SessionInfo {
    user: String,
    tty: String,
    seat: String,
    remote_host: String,
    display: String,
    start_time: Option<u64>,
    leader: Option<u32>,
}

/// The safe boundary between sd-login and record assembly.
trait LoginApi {
    fn sessions(&self) -> UResult<Vec<String>>;
    fn session(&self, id: &str) -> Option<SessionInfo>;
}

impl LoginApi for ffi::SystemdLoginApi {
    fn sessions(&self) -> UResult<Vec<String>> {
        login::get_sessions(self)
            .map_err(|e| USimpleError::new(1, format!("Failed to get systemd sessions: {e}")))
    }

    fn session(&self, session_id: &str) -> Option<SessionInfo> {
        let uid = login::get_session_uid(self, session_id).ok()?;
        // SAFETY: passwd and result are writable outputs, and buf remains alive
        // while getpwuid_r fills them. Only a successful, non-NULL result is read.
        let user = unsafe {
            let mut passwd = MaybeUninit::<libc::passwd>::uninit();

            // Get recommended buffer size, fall back if indeterminate
            let buf_size = {
                let size = libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX);
                if size == -1 {
                    16384 // Value was indeterminate, use fallback from getpwuid_r man page
                } else {
                    size as usize
                }
            };
            let mut buf = vec![0u8; buf_size];
            let mut result: *mut libc::passwd = std::ptr::null_mut();

            let ret = libc::getpwuid_r(
                uid,
                passwd.as_mut_ptr(),
                buf.as_mut_ptr().cast(),
                buf.len(),
                &raw mut result,
            );

            if ret == 0 && !result.is_null() {
                let passwd = passwd.assume_init();
                CStr::from_ptr(passwd.pw_name)
                    .to_string_lossy()
                    .into_owned()
            } else {
                format!("{uid}") // fallback to UID if username not found
            }
        };

        Some(SessionInfo {
            user,
            tty: login::get_session_string(session_id, self.sd_session_get_tty),
            seat: login::get_session_string(session_id, self.sd_session_get_seat),
            remote_host: login::get_session_string(session_id, self.sd_session_get_remote_host),
            display: login::get_session_string(session_id, self.sd_session_get_display),
            start_time: login::get_session_start_time(self, session_id),
            leader: login::get_session_leader(self, session_id),
        })
    }
}

fn known_login_time(timestamp: SystemTime) -> Option<SystemTime> {
    // Treat the epoch as unknown, including a zero start time from sd-login.
    (timestamp > UNIX_EPOCH).then_some(timestamp)
}

fn read_login_records_with_api(api: &impl LoginApi) -> UResult<Vec<SystemdLoginRecord>> {
    let mut records = Vec::new();

    // Add boot time record first
    if let Ok(boot_time) = login::get_boot_time() {
        let boot_record = SystemdLoginRecord {
            user: "reboot".to_string(),
            session_id: "boot".to_string(),
            seat_or_tty: "~".to_string(), // Traditional boot time indicator
            raw_device: String::new(),
            host: String::new(),
            login_time: known_login_time(boot_time),
            pid: 0,
            session_leader_pid: 0,
            record_type: SystemdRecordType::BootTime,
        };
        records.push(boot_record);
    }

    // Get all active sessions using safe wrapper
    let mut sessions = api.sessions()?;

    // Sort sessions consistently for reproducible output (reverse for TTY sessions first)
    sessions.sort();
    sessions.reverse();

    // Iterate through all sessions
    for session_id in sessions {
        let Some(SessionInfo {
            user,
            mut tty,
            mut seat,
            remote_host,
            display,
            start_time,
            leader,
        }) = api.session(&session_id)
        else {
            continue;
        };
        let start_time = start_time
            .and_then(|usec| UNIX_EPOCH.checked_add(Duration::from_micros(usec)))
            .and_then(known_login_time);

        // Strip any existing prefixes from systemd values (if any)
        if tty.starts_with('?') {
            tty = tty[1..].to_string();
        }
        if seat.starts_with('?') {
            seat = seat[1..].to_string();
        }

        // Determine host (use remote_host if available)
        // If host is local (non-remote) we use display,
        let host = if remote_host.is_empty() {
            display.clone()
        } else {
            remote_host
        };

        // Skip sessions that have neither TTY nor seat (e.g., manager sessions)
        if tty.is_empty() && seat.is_empty() && display.is_empty() {
            continue;
        }

        // A single session can be associated with both a TTY and a seat.
        // GNU `who` and `pinky` create separate records for each.
        // We replicate that behavior here.
        // Order: seat first, then TTY to match expected output

        // Helper closure to create a record
        let create_record = |seat_or_tty: String,
                             raw_device: String,
                             user: String,
                             session_id: String,
                             host: String| {
            SystemdLoginRecord {
                user,
                session_id,
                seat_or_tty,
                raw_device,
                host,
                login_time: start_time,
                pid: leader.unwrap_or(0),
                session_leader_pid: leader.unwrap_or(0),
                record_type: SystemdRecordType::UserProcess,
            }
        };

        // Create records based on available seat/tty/display
        if !seat.is_empty() && !tty.is_empty() {
            // Both seat and tty - need 2 records, clone for first.
            // The seat is prefixed with '?' to match GNU's output.
            let seat_formatted = format!("?{seat}");
            records.push(create_record(
                seat_formatted,
                seat,
                user.clone(),
                session_id.clone(),
                host.clone(),
            ));

            let tty_formatted = if tty.starts_with("tty") {
                format!("*{tty}")
            } else {
                tty.clone()
            };
            records.push(create_record(tty_formatted, tty, user, session_id, host)); // Move for second (and last) record
        } else if !seat.is_empty() {
            // Only seat
            let seat_formatted = format!("?{seat}");
            records.push(create_record(seat_formatted, seat, user, session_id, host));
        } else if !tty.is_empty() {
            // Only tty
            let tty_formatted = if tty.starts_with("tty") {
                format!("*{tty}")
            } else {
                tty.clone()
            };
            records.push(create_record(tty_formatted, tty, user, session_id, host));
        } else if !display.is_empty() {
            // Only display
            // No raw device for display sessions
            records.push(create_record(
                display,
                String::new(),
                user,
                session_id,
                host,
            ));
        }
    }

    Ok(records)
}

/// Wrapper to provide utmpx-compatible interface for a single record
pub struct SystemdUtmpxCompat {
    record: SystemdLoginRecord,
}

impl SystemdUtmpxCompat {
    /// Create new instance from a [`SystemdLoginRecord`]
    pub fn new(record: SystemdLoginRecord) -> Self {
        Self { record }
    }

    /// A.K.A. `ut.ut_type`
    pub fn record_type(&self) -> i16 {
        self.record.record_type as i16
    }

    /// The session leader PID, if available.
    pub fn pid(&self) -> Option<i32> {
        i32::try_from(self.record.pid).ok().filter(|pid| *pid > 0)
    }

    /// A.K.A. `ut.ut_id`
    pub fn terminal_suffix(&self) -> String {
        // Extract last part of session ID or use session ID
        self.record.session_id.clone()
    }

    /// A.K.A. `ut.ut_user`
    pub fn user(&self) -> String {
        self.record.user.clone()
    }

    /// A.K.A. `ut.ut_host`
    pub fn host(&self) -> String {
        self.record.host.clone()
    }

    /// A.K.A. `ut.ut_line`
    pub fn tty_device(&self) -> String {
        // Return raw device name for device access if available, otherwise formatted seat_or_tty
        if self.record.raw_device.is_empty() {
            self.record.seat_or_tty.clone()
        } else {
            self.record.raw_device.clone()
        }
    }

    /// The login time in the local offset, if available and representable.
    pub fn login_time(&self) -> Option<time::OffsetDateTime> {
        let duration = self.record.login_time?.duration_since(UNIX_EPOCH).ok()?;
        let ts_nanos: i128 = duration.as_nanos().try_into().ok()?;
        let datetime = time::OffsetDateTime::from_unix_timestamp_nanos(ts_nanos).ok()?;
        let local_offset = time::OffsetDateTime::now_local()
            .map_or_else(|_| time::UtcOffset::UTC, time::OffsetDateTime::offset);
        Some(datetime.to_offset(local_offset))
    }

    /// Exit status (not available from systemd)
    pub fn exit_status(&self) -> (i16, i16) {
        (0, 0) // Not available from systemd
    }

    /// Check if this is a user process record
    pub fn is_user_process(&self) -> bool {
        self.record.is_user_process()
    }

    /// Canonical host name
    pub fn canon_host(&self) -> String {
        // Simple implementation - just return the host as-is
        // Could be enhanced with DNS lookup like the original
        self.record.host.clone()
    }
}

/// Container for reading multiple systemd records
pub struct SystemdUtmpxIter {
    records: Vec<SystemdLoginRecord>,
    current_index: usize,
}

impl SystemdUtmpxIter {
    /// Create new instance and read records from systemd-logind
    pub fn new() -> UResult<Self> {
        let records = read_login_records()?;
        Ok(Self {
            records,
            current_index: 0,
        })
    }

    /// Create empty iterator (for when systemd initialization fails)
    pub fn empty() -> Self {
        Self {
            records: Vec::new(),
            current_index: 0,
        }
    }

    /// Get next record (similar to getutxent)
    pub fn next_record(&mut self) -> Option<SystemdUtmpxCompat> {
        if self.current_index >= self.records.len() {
            return None;
        }

        let record = self.records[self.current_index].clone();
        self.current_index += 1;

        Some(SystemdUtmpxCompat::new(record))
    }

    /// Get all records at once
    pub fn get_all_records(&self) -> Vec<SystemdUtmpxCompat> {
        self.records
            .iter()
            .cloned()
            .map(SystemdUtmpxCompat::new)
            .collect()
    }

    /// Reset iterator to beginning
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Get number of records
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl Iterator for SystemdUtmpxIter {
    type Item = SystemdUtmpxCompat;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_record()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Login {
        records: Vec<(String, SessionInfo)>,
        fail: bool,
    }

    impl Login {
        fn new(time: Option<u64>, leader: Option<u32>) -> Self {
            Self {
                records: vec![(
                    "legacy".into(),
                    SessionInfo {
                        user: "user".into(),
                        tty: "pts/test".into(),
                        remote_host: "alias.test".into(),
                        start_time: time,
                        leader,
                        ..SessionInfo::default()
                    },
                )],
                ..Self::default()
            }
        }
    }

    impl LoginApi for Login {
        fn sessions(&self) -> UResult<Vec<String>> {
            if self.fail {
                return Err(USimpleError::new(1, "session enumeration failed"));
            }
            Ok(self.records.iter().map(|(id, _)| id.clone()).collect())
        }

        fn session(&self, id: &str) -> Option<SessionInfo> {
            self.records
                .iter()
                .find(|(name, _)| name == id)
                .map(|(_, info)| info.clone())
        }
    }

    #[test]
    fn enumeration_errors_are_propagated() {
        let login = Login {
            fail: true,
            ..Login::default()
        };
        assert!(read_login_records_with_api(&login).is_err());
    }

    #[test]
    fn sessions_without_a_terminal_are_skipped() {
        let mut login = Login::new(None, None);
        login.records[0].1.tty.clear();
        let records = read_login_records_with_api(&login).unwrap();
        assert!(!records.iter().any(SystemdLoginRecord::is_user_process));
    }

    #[test]
    fn seat_and_tty_keep_their_metadata() {
        let mut login = Login::new(Some(1_700_000_000_000_000), Some(31337));
        login.records[0].1.seat = "?seat0".into();
        login.records[0].1.tty = "?tty1".into();
        let records = read_login_records_with_api(&login)
            .unwrap()
            .into_iter()
            .filter(SystemdLoginRecord::is_user_process)
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].seat_or_tty, "?seat0");
        assert_eq!(records[1].seat_or_tty, "*tty1");
        assert!(
            records
                .iter()
                .all(|r| r.pid == 31337 && r.host == "alias.test")
        );
    }

    #[rstest::rstest]
    #[case(
        Some(1_700_000_000_000_123),
        Some(31337),
        Some(1_700_000_000_000_123_000),
        Some(31337)
    )]
    #[case(
        Some(1_700_000_000_000_123),
        None,
        Some(1_700_000_000_000_123_000),
        None
    )]
    #[case(None, Some(31337), None, Some(31337))]
    #[case(None, None, None, None)]
    #[case(Some(0), None, None, None)]
    fn session_metadata_is_preserved(
        #[case] time: Option<u64>,
        #[case] leader: Option<u32>,
        #[case] expected_timestamp: Option<i128>,
        #[case] expected_pid: Option<i32>,
    ) {
        let login = Login::new(time, leader);
        let record = read_login_records_with_api(&login)
            .unwrap()
            .into_iter()
            .find(SystemdLoginRecord::is_user_process)
            .unwrap();
        assert_eq!(record.session_leader_pid, record.pid);
        let compat = SystemdUtmpxCompat::new(record);
        assert_eq!(compat.pid(), expected_pid);
        assert_eq!(
            compat
                .login_time()
                .map(time::OffsetDateTime::unix_timestamp_nanos),
            expected_timestamp
        );
    }

    #[test]
    fn test_empty_iterator() {
        let mut iter = SystemdUtmpxIter::empty();

        assert_eq!(iter.len(), 0);
        assert!(iter.is_empty());
        assert!(iter.next().is_none());
        assert!(iter.next_record().is_none());
    }

    #[test]
    fn test_iterator_with_mock_data() {
        let mock_records = vec![
            test_session_record(),
            SystemdLoginRecord {
                session_id: "session2".into(),
                ..test_session_record()
            },
        ];

        let mut iter = SystemdUtmpxIter {
            records: mock_records,
            current_index: 0,
        };

        assert_eq!(iter.len(), 2);
        assert!(!iter.is_empty());

        // Test iterator behavior
        let first = iter.next();
        assert!(first.is_some());

        let second = iter.next();
        assert!(second.is_some());

        let third = iter.next();
        assert!(third.is_none());

        // Iterator should be exhausted
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_get_all_records() {
        let iter = SystemdUtmpxIter {
            records: vec![test_session_record()],
            current_index: 0,
        };

        let all_records = iter.get_all_records();
        assert_eq!(all_records.len(), 1);
    }

    #[test]
    fn test_systemd_record_conversion() {
        let record = SystemdLoginRecord {
            seat_or_tty: "?seat0".into(),
            raw_device: "seat0".into(),
            ..test_session_record()
        };

        let compat = SystemdUtmpxCompat::new(record);

        assert_eq!(compat.user(), "user");
        assert_eq!(compat.tty_device().as_str(), "seat0");
        assert_eq!(compat.host(), "alias.test");
    }
}
