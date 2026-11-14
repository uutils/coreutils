// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore logind libsystemd btime unref RAII testuser GETPW sysconf

//! Systemd-logind support for reading login records
//!
//! This module provides systemd-logind based implementation for reading
//! login records as an alternative to traditional utmp/utmpx files.
//! On Linux, this is used when the traditional utmp file is unavailable or
//! empty. If systemd-logind is unavailable, callers fall back to the traditional
//! implementation.

use core::ffi::CStr;
use std::mem::MaybeUninit;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{UResult, USimpleError};
use crate::utmpx::ReadOptions;

mod bus;

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
            Self::load_from("libsystemd.so.0")
        }

        fn load_from(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
            // SAFETY: The loaded symbols use the signatures declared by libsystemd's
            // stable public API. The Library is retained in Self for at least as long
            // as the copied function pointers can be called.
            unsafe {
                let library = Library::new(path.as_ref())?;
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

        #[test]
        fn missing_library_is_reported() {
            assert!(SystemdLoginApi::load_from("libsystemd-uutils-does-not-exist.so").is_err());
        }

        #[test]
        fn invalid_library_is_reported() {
            let library = tempfile::NamedTempFile::new().unwrap();
            assert!(SystemdLoginApi::load_from(library.path()).is_err());
        }

        #[test]
        #[cfg(all(target_env = "gnu", not(target_feature = "crt-static")))]
        fn missing_login_symbols_are_reported() {
            let error = SystemdLoginApi::load_from("libc.so.6").err().unwrap();
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

        let result = unsafe { (api.sd_session_get_uid)(session_cstring.as_ptr(), &raw mut uid) };

        if result < 0 {
            return Err(
                format!("sd_session_get_uid failed for session '{session_id}': {result}").into(),
            );
        }

        Ok(uid)
    }

    /// Get start time for a session (in microseconds since Unix epoch)
    pub fn get_session_start_time(
        api: &ffi::SystemdLoginApi,
        session_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let get_start_time = api
            .sd_session_get_start_time
            .ok_or("sd_session_get_start_time is unavailable")?;
        let session_cstring = CString::new(session_id)?;
        let mut usec: u64 = 0;

        let result = unsafe { get_start_time(session_cstring.as_ptr(), &raw mut usec) };

        if result < 0 || usec == 0 {
            return Err(format!(
                "sd_session_get_start_time failed for session '{session_id}': {result}",
            )
            .into());
        }

        Ok(usec)
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
    /// The Unix epoch represents an unavailable session timestamp.
    /// Use [`Self::try_login_time_offset`] to distinguish missing values.
    pub login_time: SystemTime,
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

    /// Get login time as [`time::OffsetDateTime`] compatible with utmpx
    pub fn try_login_time_offset(&self) -> Option<time::OffsetDateTime> {
        let duration = self.login_time.duration_since(UNIX_EPOCH).ok()?;
        if duration.is_zero() {
            return None;
        }
        let ts_nanos: i128 = duration.as_nanos().try_into().ok()?;
        let datetime = time::OffsetDateTime::from_unix_timestamp_nanos(ts_nanos).ok()?;
        Some(crate::utmpx::local_login_time(datetime))
    }

    /// Get the login time, or the Unix epoch when it is unavailable.
    /// Use [`Self::try_login_time_offset`] when displaying timestamps.
    pub fn login_time_offset(&self) -> time::OffsetDateTime {
        self.try_login_time_offset()
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
    }
}

/// Read login records from systemd-logind using safe wrapper functions
/// This matches the approach used by GNU coreutils `read_utmp_from_systemd()`
pub fn read_login_records() -> UResult<Vec<SystemdLoginRecord>> {
    read_login_records_with_options(ReadOptions::default())
}

/// Read records with explicit requirements for optional session metadata.
/// The boot record is still obtained regardless of these options.
pub fn read_login_records_with_options(options: ReadOptions) -> UResult<Vec<SystemdLoginRecord>> {
    let api = ffi::SystemdLoginApi::load()
        .map_err(|e| USimpleError::new(1, format!("Failed to load libsystemd: {e}")))?;
    read_login_records_with_api(&api, options, bus::session_details)
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
    fn session(&self, id: &str, options: ReadOptions) -> Option<SessionInfo>;
}

impl LoginApi for ffi::SystemdLoginApi {
    fn sessions(&self) -> UResult<Vec<String>> {
        login::get_sessions(self)
            .map_err(|e| USimpleError::new(1, format!("Failed to get systemd sessions: {e}")))
    }

    fn session(&self, session_id: &str, options: ReadOptions) -> Option<SessionInfo> {
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
            start_time: options
                .session_time
                .then(|| login::get_session_start_time(self, session_id).ok())
                .flatten(),
            leader: options
                .session_pid
                .then(|| login::get_session_leader(self, session_id))
                .flatten(),
        })
    }
}

fn read_login_records_with_api(
    api: &impl LoginApi,
    options: ReadOptions,
    session_details: impl FnOnce(
        Vec<(String, ReadOptions)>,
    ) -> std::collections::HashMap<String, bus::SessionDetails>,
) -> UResult<Vec<SystemdLoginRecord>> {
    let mut records = Vec::new();
    let mut missing_details = Vec::new();

    // Add boot time record first
    if let Ok(boot_time) = login::get_boot_time() {
        let boot_record = SystemdLoginRecord {
            user: "reboot".to_string(),
            session_id: "boot".to_string(),
            seat_or_tty: "~".to_string(), // Traditional boot time indicator
            raw_device: String::new(),
            host: String::new(),
            login_time: boot_time,
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
        }) = api.session(&session_id, options)
        else {
            continue;
        };
        let start_time = start_time
            .filter(|usec| *usec > 0)
            .and_then(|usec| UNIX_EPOCH.checked_add(std::time::Duration::from_micros(usec)))
            .unwrap_or(UNIX_EPOCH);

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

        let missing = ReadOptions {
            session_time: options.session_time && start_time == UNIX_EPOCH,
            session_pid: options.session_pid && leader.is_none(),
        };
        if missing.session_time || missing.session_pid {
            missing_details.push((session_id.clone(), missing));
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

    let details = session_details(missing_details);
    for record in &mut records {
        if let Some(details) = details.get(&record.session_id) {
            if record.login_time == UNIX_EPOCH {
                record.login_time = details
                    .start_time
                    .and_then(|usec| UNIX_EPOCH.checked_add(std::time::Duration::from_micros(usec)))
                    .unwrap_or(UNIX_EPOCH);
            }
            if record.pid == 0 {
                record.pid = details.leader.unwrap_or_default();
                record.session_leader_pid = record.pid;
            }
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

    /// A.K.A. `ut.ut_pid`
    pub fn try_pid(&self) -> Option<i32> {
        i32::try_from(self.record.pid).ok().filter(|pid| *pid > 0)
    }

    /// Session leader PID, or zero when unavailable.
    pub fn pid(&self) -> i32 {
        self.try_pid().unwrap_or_default()
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

    /// Login time
    pub fn login_time(&self) -> time::OffsetDateTime {
        self.record.login_time_offset()
    }

    /// Login time, or None when neither sd-login nor D-Bus provided one.
    pub fn try_login_time(&self) -> Option<time::OffsetDateTime> {
        self.record.try_login_time_offset()
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
        self.try_canon_host().unwrap_or_else(|_| self.host())
    }

    /// Canonical host name, propagating DNS errors.
    pub fn try_canon_host(&self) -> std::io::Result<String> {
        crate::utmpx::canonicalize_host(self.record.host.clone())
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
        Self::with_options(ReadOptions::default())
    }

    /// Create an iterator with explicit session metadata requirements.
    pub fn with_options(options: ReadOptions) -> UResult<Self> {
        let records = read_login_records_with_options(options)?;
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
#[path = "systemd_logind/tests.rs"]
mod regression_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_metadata_preserves_legacy_api_types() {
        let record = SystemdLoginRecord {
            session_id: "missing".into(),
            user: "user".into(),
            seat_or_tty: "pts/0".into(),
            raw_device: "pts/0".into(),
            host: String::new(),
            login_time: UNIX_EPOCH,
            pid: 0,
            session_leader_pid: 0,
            record_type: SystemdRecordType::UserProcess,
        };
        let timestamp: SystemTime = record.login_time;
        assert_eq!(timestamp, UNIX_EPOCH);
        let offset: time::OffsetDateTime = record.login_time_offset();
        assert_eq!(offset, time::OffsetDateTime::UNIX_EPOCH);
        assert_eq!(record.try_login_time_offset(), None);
        let compat = SystemdUtmpxCompat::new(record);
        let pid: i32 = compat.pid();
        let timestamp: time::OffsetDateTime = compat.login_time();
        let host: String = compat.canon_host();
        assert_eq!(pid, 0);
        assert_eq!(timestamp, time::OffsetDateTime::UNIX_EPOCH);
        assert!(host.is_empty());
        assert_eq!(compat.try_pid(), None);
        assert_eq!(compat.try_login_time(), None);
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
        // Create iterator with mock records
        let mock_records = vec![
            SystemdLoginRecord {
                session_id: "session1".to_string(),
                user: "user1".to_string(),
                seat_or_tty: "tty1".to_string(),
                raw_device: "tty1".to_string(),
                host: "host1".to_string(),
                login_time: UNIX_EPOCH,
                pid: 1234,
                session_leader_pid: 1234,
                record_type: SystemdRecordType::UserProcess,
            },
            SystemdLoginRecord {
                session_id: "session2".to_string(),
                user: "user2".to_string(),
                seat_or_tty: "pts/0".to_string(),
                raw_device: "pts/0".to_string(),
                host: "host2".to_string(),
                login_time: UNIX_EPOCH,
                pid: 5678,
                session_leader_pid: 5678,
                record_type: SystemdRecordType::UserProcess,
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
        let mock_records = vec![SystemdLoginRecord {
            session_id: "session1".to_string(),
            user: "user1".to_string(),
            seat_or_tty: "tty1".to_string(),
            raw_device: "tty1".to_string(),
            host: "host1".to_string(),
            login_time: UNIX_EPOCH,
            pid: 1234,
            session_leader_pid: 1234,
            record_type: SystemdRecordType::UserProcess,
        }];

        let iter = SystemdUtmpxIter {
            records: mock_records,
            current_index: 0,
        };

        let all_records = iter.get_all_records();
        assert_eq!(all_records.len(), 1);
    }

    #[test]
    fn test_systemd_record_conversion() {
        // Test that SystemdLoginRecord converts correctly to SystemdUtmpxCompat
        let record = SystemdLoginRecord {
            session_id: "c1".to_string(),
            user: "testuser".to_string(),
            seat_or_tty: "seat0".to_string(),
            raw_device: "seat0".to_string(),
            host: "localhost".to_string(),
            login_time: UNIX_EPOCH + std::time::Duration::from_secs(1000),
            pid: 9999,
            session_leader_pid: 9999,
            record_type: SystemdRecordType::UserProcess,
        };

        let compat = SystemdUtmpxCompat::new(record);

        // Test the actual conversion logic
        assert_eq!(compat.user(), "testuser");
        assert_eq!(compat.tty_device().as_str(), "seat0");
        assert_eq!(compat.host(), "localhost");
    }
}
