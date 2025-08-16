// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore logind libsystemd zvariant ssuso zbus

//! Systemd-logind support for reading login records.
//!
//! This module provides systemd-logind based implementation for reading
//! login records as an alternative to traditional utmp/utmpx files.
//! When the systemd-logind feature is enabled and systemd is available,
//! this will be used instead of traditional utmp files. This implementation
//! uses `zbus` to communicate with `systemd-logind` over D-Bus in pure Rust,
//! avoiding a dependency on `libsystemd`.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{UResult, USimpleError};
use crate::utmpx;
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::{OwnedObjectPath, Value},
};

/// Login record compatible with utmpx structure
#[derive(Debug, Clone)]
pub struct SystemdLoginRecord {
    pub user: String,
    pub session_id: String,
    pub seat_or_tty: String,
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

/// Read login records from systemd-logind using the D-Bus interface.
/// This matches the approach used by GNU coreutils read_utmp_from_systemd(),
/// but uses a pure Rust D-Bus implementation.
pub fn read_login_records() -> UResult<Vec<SystemdLoginRecord>> {
    let connection = Connection::system()
        .map_err(|e| USimpleError::new(1, format!("Failed to connect to D-Bus: {e}")))?;

    let proxy = Proxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .map_err(|e| USimpleError::new(1, format!("Failed to create D-Bus proxy: {e}")))?;

    // ListSessions returns `a(ssuso)`: array of (session_id, user_id, user_name, seat_id, session_object_path)
    // In zbus 4.x, call() returns the body directly.
    let sessions: Vec<(String, u32, String, String, OwnedObjectPath)> =
        proxy.call("ListSessions", &()).map_err(|e| {
            USimpleError::new(
                1,
                format!("Failed to call ListSessions or parse response: {e}"),
            )
        })?;

    let mut records = Vec::new();

    // Get boot time
    // D-Bus source for boot time is the 'KernelTimestamp'
    // property from the main systemd manager interface.
    let boot_time = match Proxy::new(
        &connection,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    ) {
        Ok(systemd_proxy) => systemd_proxy
            .get_property::<u64>("KernelTimestamp")
            .ok()
            .map(|t| UNIX_EPOCH + std::time::Duration::from_micros(t)),
        Err(_) => None,
    };

    if let Some(boot_time) = boot_time {
        records.push(SystemdLoginRecord {
            user: "reboot".to_string(),
            session_id: "".to_string(),
            seat_or_tty: "~".to_string(),
            host: "".to_string(),
            login_time: boot_time,
            pid: 0,
            session_leader_pid: 0,
            record_type: SystemdRecordType::BootTime,
        });
    }

    for (session_id, _uid, user_name, seat_id, session_path) in sessions {
        let session_proxy = Proxy::new(
            &connection,
            "org.freedesktop.login1",
            session_path.as_ref(), // Use the object path from ListSessions
            "org.freedesktop.login1.Session",
        )
        .map_err(|e| {
            USimpleError::new(
                1,
                format!("Failed to create session proxy for '{session_id}': {e}"),
            )
        })?;

        // Helper to get properties and handle potential errors
        let get_prop = |prop_name| {
            session_proxy.get_property::<Value>(prop_name).map_err(|e| {
                USimpleError::new(
                    1,
                    format!("Failed to get property '{prop_name}' for session '{session_id}': {e}"),
                )
            })
        };

        let start_time_usec: u64 = get_prop("Timestamp")?
            .try_into()
            .map_err(|e| USimpleError::new(1, format!("Invalid Timestamp value: {e}")))?;
        let start_time = UNIX_EPOCH + std::time::Duration::from_micros(start_time_usec);

        let tty: String = get_prop("TTY")?
            .try_into()
            .map_err(|e| USimpleError::new(1, format!("Invalid TTY value: {e}")))?;
        let remote_host: String = get_prop("RemoteHost")?
            .try_into()
            .map_err(|e| USimpleError::new(1, format!("Invalid RemoteHost value: {e}")))?;

        let leader_pid: u32 = get_prop("Leader")?
            .try_into()
            .map_err(|e| USimpleError::new(1, format!("Invalid Leader PID value: {e}")))?;

        // A single session can be associated with both a TTY and a seat.
        // GNU `who` and `pinky` create separate records for each.
        // We replicate that behavior here.

        if !tty.is_empty() {
            records.push(SystemdLoginRecord {
                user: user_name.clone(),
                session_id: session_id.clone(),
                seat_or_tty: tty,
                host: remote_host.clone(),
                login_time: start_time,
                pid: leader_pid,
                session_leader_pid: leader_pid,
                record_type: SystemdRecordType::UserProcess,
            });
        }

        // Also create a record for the seat if it's not empty.
        // The seat is prefixed with '?' to match GNU's output.
        if !seat_id.is_empty() {
            records.push(SystemdLoginRecord {
                user: user_name.clone(),
                session_id: session_id.clone(),
                seat_or_tty: format!("?{seat_id}"),
                host: remote_host.clone(),
                login_time: start_time,
                pid: leader_pid,
                session_leader_pid: leader_pid,
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
        self.record.seat_or_tty.clone()
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
    pub records: Vec<SystemdLoginRecord>,
    pub current_index: usize,
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

        // Return SystemdUtmpxCompat
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
