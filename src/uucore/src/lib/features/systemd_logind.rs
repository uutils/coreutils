// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore logind libsystemd btime unref RAII

//! Systemd-logind support for reading login records
//!
//! This module provides systemd-logind based implementation for reading
//! login records as an alternative to traditional utmp/utmpx files.
//! When the systemd-logind feature is enabled and systemd is available,
//! this will be used instead of traditional utmp files.

use std::ffi::CStr;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{UResult, USimpleError};
use crate::utmpx;

/// FFI bindings for libsystemd login and D-Bus functions
mod ffi {
    use std::ffi::c_char;
    use std::os::raw::{c_int, c_uint};

    #[link(name = "systemd")]
    unsafe extern "C" {
        pub fn sd_get_sessions(sessions: *mut *mut *mut c_char) -> c_int;
        pub fn sd_session_get_uid(session: *const c_char, uid: *mut c_uint) -> c_int;
        pub fn sd_session_get_start_time(session: *const c_char, usec: *mut u64) -> c_int;
        pub fn sd_session_get_tty(session: *const c_char, tty: *mut *mut c_char) -> c_int;
        pub fn sd_session_get_remote_host(
            session: *const c_char,
            remote_host: *mut *mut c_char,
        ) -> c_int;
        pub fn sd_session_get_display(session: *const c_char, display: *mut *mut c_char) -> c_int;
        pub fn sd_session_get_type(session: *const c_char, session_type: *mut *mut c_char)
        -> c_int;
        pub fn sd_session_get_seat(session: *const c_char, seat: *mut *mut c_char) -> c_int;

    }
}

/// Safe wrapper functions for libsystemd FFI calls
mod login {
    use super::ffi;
    use std::ffi::{CStr, CString};
    use std::ptr;
    use std::time::SystemTime;

    /// Get all active sessions
    pub fn get_sessions() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut sessions_ptr: *mut *mut i8 = ptr::null_mut();

        let result = unsafe { ffi::sd_get_sessions(&mut sessions_ptr) };

        if result < 0 {
            return Err(format!("sd_get_sessions failed: {}", result).into());
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

                unsafe { libc::free(session_ptr as *mut libc::c_void) };
                i += 1;
            }

            unsafe { libc::free(sessions_ptr as *mut libc::c_void) };
        }

        Ok(sessions)
    }

    /// Get UID for a session
    pub fn get_session_uid(session_id: &str) -> Result<u32, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut uid: std::os::raw::c_uint = 0;

        let result = unsafe { ffi::sd_session_get_uid(session_cstring.as_ptr(), &mut uid) };

        if result < 0 {
            return Err(format!(
                "sd_session_get_uid failed for session '{}': {}",
                session_id, result
            )
            .into());
        }

        Ok(uid)
    }

    /// Get start time for a session (in microseconds since Unix epoch)
    pub fn get_session_start_time(session_id: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut usec: u64 = 0;

        let result = unsafe { ffi::sd_session_get_start_time(session_cstring.as_ptr(), &mut usec) };

        if result < 0 {
            return Err(format!(
                "sd_session_get_start_time failed for session '{}': {}",
                session_id, result
            )
            .into());
        }

        Ok(usec)
    }

    /// Get TTY for a session
    pub fn get_session_tty(session_id: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut tty_ptr: *mut i8 = ptr::null_mut();

        let result = unsafe { ffi::sd_session_get_tty(session_cstring.as_ptr(), &mut tty_ptr) };

        if result < 0 {
            return Err(format!(
                "sd_session_get_tty failed for session '{}': {}",
                session_id, result
            )
            .into());
        }

        if tty_ptr.is_null() {
            return Ok(None);
        }

        let tty_cstr = unsafe { CStr::from_ptr(tty_ptr) };
        let tty_string = tty_cstr.to_string_lossy().into_owned();

        unsafe { libc::free(tty_ptr as *mut libc::c_void) };

        Ok(Some(tty_string))
    }

    /// Get remote host for a session
    pub fn get_session_remote_host(
        session_id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut host_ptr: *mut i8 = ptr::null_mut();

        let result =
            unsafe { ffi::sd_session_get_remote_host(session_cstring.as_ptr(), &mut host_ptr) };

        if result < 0 {
            return Err(format!(
                "sd_session_get_remote_host failed for session '{}': {}",
                session_id, result
            )
            .into());
        }

        if host_ptr.is_null() {
            return Ok(None);
        }

        let host_cstr = unsafe { CStr::from_ptr(host_ptr) };
        let host_string = host_cstr.to_string_lossy().into_owned();

        unsafe { libc::free(host_ptr as *mut libc::c_void) };

        Ok(Some(host_string))
    }

    /// Get display for a session
    pub fn get_session_display(
        session_id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut display_ptr: *mut i8 = ptr::null_mut();

        let result =
            unsafe { ffi::sd_session_get_display(session_cstring.as_ptr(), &mut display_ptr) };

        if result < 0 {
            return Err(format!(
                "sd_session_get_display failed for session '{}': {}",
                session_id, result
            )
            .into());
        }

        if display_ptr.is_null() {
            return Ok(None);
        }

        let display_cstr = unsafe { CStr::from_ptr(display_ptr) };
        let display_string = display_cstr.to_string_lossy().into_owned();

        unsafe { libc::free(display_ptr as *mut libc::c_void) };

        Ok(Some(display_string))
    }

    /// Get type for a session
    pub fn get_session_type(
        session_id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut type_ptr: *mut i8 = ptr::null_mut();

        let result = unsafe { ffi::sd_session_get_type(session_cstring.as_ptr(), &mut type_ptr) };

        if result < 0 {
            return Err(format!(
                "sd_session_get_type failed for session '{}': {}",
                session_id, result
            )
            .into());
        }

        if type_ptr.is_null() {
            return Ok(None);
        }

        let type_cstr = unsafe { CStr::from_ptr(type_ptr) };
        let type_string = type_cstr.to_string_lossy().into_owned();

        unsafe { libc::free(type_ptr as *mut libc::c_void) };

        Ok(Some(type_string))
    }

    /// Get seat for a session
    pub fn get_session_seat(
        session_id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let session_cstring = CString::new(session_id)?;
        let mut seat_ptr: *mut i8 = ptr::null_mut();

        let result = unsafe { ffi::sd_session_get_seat(session_cstring.as_ptr(), &mut seat_ptr) };

        if result < 0 {
            return Err(format!(
                "sd_session_get_seat failed for session '{}': {}",
                session_id, result
            )
            .into());
        }

        if seat_ptr.is_null() {
            return Ok(None);
        }

        let seat_cstr = unsafe { CStr::from_ptr(seat_ptr) };
        let seat_string = seat_cstr.to_string_lossy().into_owned();

        unsafe { libc::free(seat_ptr as *mut libc::c_void) };

        Ok(Some(seat_string))
    }

    /// Get system boot time using systemd random-seed file fallback
    ///
    /// TODO: This replicates GNU coreutils' fallback behavior for compatibility.
    /// GNU coreutils uses the mtime of /var/lib/systemd/random-seed as a heuristic for boot time
    /// when utmp is unavailable, rather than querying systemd's authoritative KernelTimestamp.
    /// This creates inconsistency: `uptime -s` shows the actual kernel boot time
    /// while `who -b` shows ~1 minute later when systemd services start.
    ///
    /// Ideally, both should use the same source (KernelTimestamp) for semantic consistency.
    /// Consider proposing to GNU coreutils to use systemd's KernelTimestamp property instead.
    pub fn get_boot_time() -> Result<SystemTime, Box<dyn std::error::Error>> {
        use std::fs;

        let metadata = fs::metadata("/var/lib/systemd/random-seed")
            .map_err(|e| format!("Failed to read /var/lib/systemd/random-seed: {}", e))?;

        metadata
            .modified()
            .map_err(|e| format!("Failed to get modification time: {}", e).into())
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
    pub fn login_time_offset(&self) -> utmpx::time::OffsetDateTime {
        let duration = self
            .login_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let ts_nanos: i128 = (duration.as_nanos()).try_into().unwrap_or(0);
        let local_offset = utmpx::time::OffsetDateTime::now_local()
            .map_or_else(|_| utmpx::time::UtcOffset::UTC, |v| v.offset());
        utmpx::time::OffsetDateTime::from_unix_timestamp_nanos(ts_nanos)
            .unwrap_or_else(|_| {
                utmpx::time::OffsetDateTime::now_local()
                    .unwrap_or_else(|_| utmpx::time::OffsetDateTime::now_utc())
            })
            .to_offset(local_offset)
    }
}

/// Read login records from systemd-logind using safe wrapper functions
/// This matches the approach used by GNU coreutils read_utmp_from_systemd()
pub fn read_login_records() -> UResult<Vec<SystemdLoginRecord>> {
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
    let mut sessions = login::get_sessions()
        .map_err(|e| USimpleError::new(1, format!("Failed to get systemd sessions: {e}")))?;

    // Sort sessions consistently for reproducible output (reverse for TTY sessions first)
    sessions.sort();
    sessions.reverse();

    // Iterate through all sessions
    for session_id in sessions {
        // Get session UID using safe wrapper
        let uid = match login::get_session_uid(&session_id) {
            Ok(uid) => uid,
            Err(_) => continue,
        };

        // Get username from UID
        let user = unsafe {
            let passwd = libc::getpwuid(uid);
            if passwd.is_null() {
                format!("{}", uid) // fallback to UID if username not found
            } else {
                CStr::from_ptr((*passwd).pw_name)
                    .to_string_lossy()
                    .into_owned()
            }
        };

        // Get start time using safe wrapper
        let start_time = login::get_session_start_time(&session_id)
            .map(|usec| UNIX_EPOCH + std::time::Duration::from_micros(usec))
            .unwrap_or(UNIX_EPOCH); // fallback to epoch if unavailable

        // Get TTY using safe wrapper
        let mut tty = login::get_session_tty(&session_id)
            .ok()
            .flatten()
            .unwrap_or_default();

        // Get seat using safe wrapper
        let mut seat = login::get_session_seat(&session_id)
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
        let remote_host = login::get_session_remote_host(&session_id)
            .ok()
            .flatten()
            .unwrap_or_default();

        // Get display using safe wrapper (for GUI sessions)
        let display = login::get_session_display(&session_id)
            .ok()
            .flatten()
            .unwrap_or_default();

        // Get session type using safe wrapper (currently unused but available)
        let _session_type = login::get_session_type(&session_id)
            .ok()
            .flatten()
            .unwrap_or_default();

        // Determine host (use remote_host if available)
        let host = if remote_host.is_empty() {
            String::new()
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

        // Create a record for the seat if it's not empty.
        // The seat is prefixed with '?' to match GNU's output.
        if !seat.is_empty() {
            let seat_formatted = format!("?{}", seat);
            records.push(SystemdLoginRecord {
                user: user.clone(),
                session_id: session_id.clone(),
                seat_or_tty: seat_formatted,
                raw_device: seat.clone(), // Store raw seat name for device access
                host: host.clone(),
                login_time: start_time,
                pid: 0,
                session_leader_pid: 0,
                record_type: SystemdRecordType::UserProcess,
            });
        }

        if !tty.is_empty() {
            let tty_formatted = if tty.starts_with("tty") {
                format!("*{}", tty)
            } else {
                tty.clone()
            };

            records.push(SystemdLoginRecord {
                user: user.clone(),
                session_id: session_id.clone(),
                seat_or_tty: tty_formatted,
                raw_device: tty.clone(), // Store raw TTY for device access
                host: host.clone(),
                login_time: start_time,
                pid: 0, // systemd doesn't directly provide session leader PID in this context
                session_leader_pid: 0,
                record_type: SystemdRecordType::UserProcess,
            });
        }

        // If only display session, create a fallback record
        if tty.is_empty() && seat.is_empty() && !display.is_empty() {
            records.push(SystemdLoginRecord {
                user,
                session_id: session_id.clone(),
                seat_or_tty: display,
                raw_device: String::new(), // No raw device for display sessions
                host,
                login_time: start_time,
                pid: 0,
                session_leader_pid: 0,
                record_type: SystemdRecordType::UserProcess,
            });
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
        SystemdUtmpxCompat { record }
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
        if !self.record.raw_device.is_empty() {
            self.record.raw_device.clone()
        } else {
            self.record.seat_or_tty.clone()
        }
    }

    /// Login time
    pub fn login_time(&self) -> utmpx::time::OffsetDateTime {
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
    pub fn canon_host(&self) -> std::io::Result<String> {
        // Simple implementation - just return the host as-is
        // Could be enhanced with DNS lookup like the original
        Ok(self.record.host.clone())
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
        Ok(SystemdUtmpxIter {
            records,
            current_index: 0,
        })
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
