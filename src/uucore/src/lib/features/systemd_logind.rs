// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore logind libsystemd btime unref RAII testuser GETPW sysconf

//! Systemd-logind support for reading login records
//!
//! This module provides systemd-logind based implementation for reading
//! login records as an alternative to traditional utmp/utmpx files.
//! On Linux, this is used when the traditional utmp file is unavailable or
//! empty. If systemd-logind is unavailable, callers fall back to the traditional
//! implementation.

use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{UResult, USimpleError};

/// Dynamically loaded FFI bindings for libsystemd login functions.
mod ffi {
    use libloading::Library;
    use std::ffi::c_char;
    use std::os::raw::{c_int, c_uint};
    use std::path::Path;

    type SdGetSessions = unsafe extern "C" fn(*mut *mut *mut c_char) -> c_int;
    type SdSessionGetUid = unsafe extern "C" fn(*const c_char, *mut c_uint) -> c_int;
    type SdSessionGetStartTime = unsafe extern "C" fn(*const c_char, *mut u64) -> c_int;
    type SdSessionGetString = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_int;

    pub(super) struct SystemdLoginApi {
        // The library must remain loaded while any of these function pointers are used.
        _library: Library,
        pub(super) sd_get_sessions: SdGetSessions,
        pub(super) sd_session_get_uid: SdSessionGetUid,
        pub(super) sd_session_get_start_time: SdSessionGetStartTime,
        pub(super) sd_session_get_tty: SdSessionGetString,
        pub(super) sd_session_get_remote_host: SdSessionGetString,
        pub(super) sd_session_get_display: SdSessionGetString,
        pub(super) sd_session_get_type: SdSessionGetString,
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
                let sd_session_get_start_time =
                    *library.get::<SdSessionGetStartTime>(b"sd_session_get_start_time\0")?;
                let sd_session_get_tty =
                    *library.get::<SdSessionGetString>(b"sd_session_get_tty\0")?;
                let sd_session_get_remote_host =
                    *library.get::<SdSessionGetString>(b"sd_session_get_remote_host\0")?;
                let sd_session_get_display =
                    *library.get::<SdSessionGetString>(b"sd_session_get_display\0")?;
                let sd_session_get_type =
                    *library.get::<SdSessionGetString>(b"sd_session_get_type\0")?;
                let sd_session_get_seat =
                    *library.get::<SdSessionGetString>(b"sd_session_get_seat\0")?;

                Ok(Self {
                    _library: library,
                    sd_get_sessions,
                    sd_session_get_uid,
                    sd_session_get_start_time,
                    sd_session_get_tty,
                    sd_session_get_remote_host,
                    sd_session_get_display,
                    sd_session_get_type,
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
    }
}

/// Safe wrapper functions for libsystemd FFI calls
mod login {
    use super::ffi;
    use std::ffi::{CStr, CString};
    use std::ptr;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Get all active sessions
    pub fn get_sessions(
        api: &ffi::SystemdLoginApi,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut sessions_ptr: *mut *mut libc::c_char = ptr::null_mut();

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
        let mut uid: std::os::raw::c_uint = 0;

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
        let session_cstring = CString::new(session_id)?;
        let mut usec: u64 = 0;

        let result =
            unsafe { (api.sd_session_get_start_time)(session_cstring.as_ptr(), &raw mut usec) };

        if result < 0 {
            return Err(format!(
                "sd_session_get_start_time failed for session '{session_id}': {result}",
            )
            .into());
        }

        Ok(usec)
    }

    /// Get TTY for a session
    pub fn get_session_tty(
        api: &ffi::SystemdLoginApi,
        session_id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut tty_ptr: *mut libc::c_char = ptr::null_mut();

        let result =
            unsafe { (api.sd_session_get_tty)(session_cstring.as_ptr(), &raw mut tty_ptr) };

        if result < 0 {
            return Err(
                format!("sd_session_get_tty failed for session '{session_id}': {result}").into(),
            );
        }

        if tty_ptr.is_null() {
            return Ok(None);
        }

        let tty_cstr = unsafe { CStr::from_ptr(tty_ptr) };
        let tty_string = tty_cstr.to_string_lossy().into_owned();

        unsafe { libc::free(tty_ptr.cast()) };

        Ok(Some(tty_string))
    }

    /// Get remote host for a session
    pub fn get_session_remote_host(
        api: &ffi::SystemdLoginApi,
        session_id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut host_ptr: *mut libc::c_char = ptr::null_mut();

        let result = unsafe {
            (api.sd_session_get_remote_host)(session_cstring.as_ptr(), &raw mut host_ptr)
        };

        if result < 0 {
            return Err(format!(
                "sd_session_get_remote_host failed for session '{session_id}': {result}",
            )
            .into());
        }

        if host_ptr.is_null() {
            return Ok(None);
        }

        let host_cstr = unsafe { CStr::from_ptr(host_ptr) };
        let host_string = host_cstr.to_string_lossy().into_owned();

        unsafe { libc::free(host_ptr.cast()) };

        Ok(Some(host_string))
    }

    /// Get display for a session
    pub fn get_session_display(
        api: &ffi::SystemdLoginApi,
        session_id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut display_ptr: *mut libc::c_char = ptr::null_mut();

        let result =
            unsafe { (api.sd_session_get_display)(session_cstring.as_ptr(), &raw mut display_ptr) };

        if result < 0 {
            return Err(format!(
                "sd_session_get_display failed for session '{session_id}': {result}",
            )
            .into());
        }

        if display_ptr.is_null() {
            return Ok(None);
        }

        let display_cstr = unsafe { CStr::from_ptr(display_ptr) };
        let display_string = display_cstr.to_string_lossy().into_owned();

        unsafe { libc::free(display_ptr.cast()) };

        Ok(Some(display_string))
    }

    /// Get type for a session
    pub fn get_session_type(
        api: &ffi::SystemdLoginApi,
        session_id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut type_ptr: *mut libc::c_char = ptr::null_mut();

        let result =
            unsafe { (api.sd_session_get_type)(session_cstring.as_ptr(), &raw mut type_ptr) };

        if result < 0 {
            return Err(
                format!("sd_session_get_type failed for session '{session_id}': {result}").into(),
            );
        }

        if type_ptr.is_null() {
            return Ok(None);
        }

        let type_cstr = unsafe { CStr::from_ptr(type_ptr) };
        let type_string = type_cstr.to_string_lossy().into_owned();

        unsafe { libc::free(type_ptr.cast()) };

        Ok(Some(type_string))
    }

    /// Get seat for a session
    pub fn get_session_seat(
        api: &ffi::SystemdLoginApi,
        session_id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut seat_ptr: *mut libc::c_char = ptr::null_mut();

        let result =
            unsafe { (api.sd_session_get_seat)(session_cstring.as_ptr(), &raw mut seat_ptr) };

        if result < 0 {
            return Err(
                format!("sd_session_get_seat failed for session '{session_id}': {result}").into(),
            );
        }

        if seat_ptr.is_null() {
            return Ok(None);
        }

        let seat_cstr = unsafe { CStr::from_ptr(seat_ptr) };
        let seat_string = seat_cstr.to_string_lossy().into_owned();

        unsafe { libc::free(seat_ptr.cast()) };

        Ok(Some(seat_string))
    }

    pub(super) fn boot_time_from_proc_stat(contents: &str) -> Option<SystemTime> {
        contents.lines().find_map(|line| {
            line.strip_prefix("btime ")
                .and_then(|seconds| seconds.parse::<u64>().ok())
                .map(|seconds| UNIX_EPOCH + std::time::Duration::from_secs(seconds))
        })
    }

    /// Get the system boot time using the kernel's `/proc/stat` value.
    pub fn get_boot_time() -> Result<SystemTime, Box<dyn std::error::Error>> {
        let proc_stat = std::fs::read_to_string("/proc/stat")
            .map_err(|e| format!("Failed to read /proc/stat: {e}"))?;
        boot_time_from_proc_stat(&proc_stat)
            .ok_or_else(|| "Failed to find btime in /proc/stat".into())
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

    /// Get login time as time::OffsetDateTime compatible with utmpx
    pub fn login_time_offset(&self) -> time::OffsetDateTime {
        let duration = self
            .login_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let ts_nanos: i128 = (duration.as_nanos()).try_into().unwrap_or(0);
        let local_offset = time::OffsetDateTime::now_local()
            .map_or_else(|_| time::UtcOffset::UTC, time::OffsetDateTime::offset);
        time::OffsetDateTime::from_unix_timestamp_nanos(ts_nanos)
            .unwrap_or_else(|_| {
                time::OffsetDateTime::now_local()
                    .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
            })
            .to_offset(local_offset)
    }
}

/// Read login records from systemd-logind using safe wrapper functions
/// This matches the approach used by GNU coreutils read_utmp_from_systemd()
pub fn read_login_records() -> UResult<Vec<SystemdLoginRecord>> {
    let api = ffi::SystemdLoginApi::load()
        .map_err(|e| USimpleError::new(1, format!("Failed to load libsystemd: {e}")))?;
    let mut records = Vec::new();

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
    let mut sessions = login::get_sessions(&api)
        .map_err(|e| USimpleError::new(1, format!("Failed to get systemd sessions: {e}")))?;

    // Sort sessions consistently for reproducible output (reverse for TTY sessions first)
    sessions.sort();
    sessions.reverse();

    // Iterate through all sessions
    for session_id in sessions {
        // Get session UID using safe wrapper
        let Ok(uid) = login::get_session_uid(&api, &session_id) else {
            continue;
        };

        // Get username from UID
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

        // Get start time using safe wrapper, fallback to epoch if unavailable
        let start_time = login::get_session_start_time(&api, &session_id)
            .map_or(UNIX_EPOCH, |usec| {
                UNIX_EPOCH + std::time::Duration::from_micros(usec)
            });

        // Get TTY using safe wrapper
        let mut tty = login::get_session_tty(&api, &session_id)
            .ok()
            .flatten()
            .unwrap_or_default();

        // Get seat using safe wrapper
        let mut seat = login::get_session_seat(&api, &session_id)
            .ok()
            .flatten()
            .unwrap_or_default();

        // Strip any existing prefixes from systemd values (if any)
        if tty.starts_with('?') {
            tty = tty[1..].to_string();
        }
        if seat.starts_with('?') {
            seat = seat[1..].to_string();
        }

        // Get remote host using safe wrapper
        let remote_host = login::get_session_remote_host(&api, &session_id)
            .ok()
            .flatten()
            .unwrap_or_default();

        // Get display using safe wrapper (for GUI sessions)
        let display = login::get_session_display(&api, &session_id)
            .ok()
            .flatten()
            .unwrap_or_default();

        // Get session type using safe wrapper (currently unused but available)
        let _session_type = login::get_session_type(&api, &session_id)
            .ok()
            .flatten()
            .unwrap_or_default();

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
                pid: 0, // systemd doesn't directly provide session leader PID in this context
                session_leader_pid: 0,
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
    /// Create new instance from a SystemdLoginRecord
    pub fn new(record: SystemdLoginRecord) -> Self {
        Self { record }
    }

    /// A.K.A. ut.ut_type
    pub fn record_type(&self) -> i16 {
        self.record.record_type as i16
    }

    /// A.K.A. ut.ut_pid
    pub fn pid(&self) -> i32 {
        self.record.pid as i32
    }

    /// A.K.A. ut.ut_id
    pub fn terminal_suffix(&self) -> String {
        // Extract last part of session ID or use session ID
        self.record.session_id.clone()
    }

    /// A.K.A. ut.ut_user
    pub fn user(&self) -> String {
        self.record.user.clone()
    }

    /// A.K.A. ut.ut_host
    pub fn host(&self) -> String {
        self.record.host.clone()
    }

    /// A.K.A. ut.ut_line
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

    #[test]
    fn test_boot_time_from_proc_stat() {
        let boot_time = login::boot_time_from_proc_stat(
            "cpu  1 2 3 4 5 6 7 8 9 10\nbtime 1234567890\nprocesses 42\n",
        );

        assert_eq!(
            boot_time,
            Some(UNIX_EPOCH + std::time::Duration::from_secs(1_234_567_890))
        );
        assert!(login::boot_time_from_proc_stat("cpu 1 2 3\n").is_none());
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
