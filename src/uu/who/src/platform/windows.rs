// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) INFOW StationName WinStation

use crate::{Row, Who, format_idle, format_timestamp};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::translate;

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::ptr;
use windows_sys::Win32::System::RemoteDesktop::{
    ProcessIdToSessionId, WTS_CURRENT_SERVER_HANDLE, WTS_INFO_CLASS, WTS_SESSION_INFOW,
    WTSClientName, WTSEnumerateSessionsW, WTSFreeMemory, WTSINFOW, WTSListen,
    WTSQuerySessionInformationW, WTSSessionInfo,
};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;

/// Owns a buffer allocated by the Windows Terminal Services API.
struct WtsBuffer<T>(*mut T);

impl<T> Drop for WtsBuffer<T> {
    fn drop(&mut self) {
        // SAFETY: the pointer was returned by WTS and is freed exactly once.
        unsafe { WTSFreeMemory(self.0.cast()) };
    }
}

struct Session {
    id: u32,
    line: String,
    user: String,
    host: String,
    logon_time: i64,
    last_input_time: i64,
}

impl Who {
    pub(crate) fn exec(&mut self) -> UResult<()> {
        if self.names_only {
            let users = sessions(!self.all)
                .iter()
                .filter(|session| !session.user.is_empty())
                .map(|session| session.user.clone())
                .collect::<Vec<_>>();
            return self.emit_names(&users);
        }

        if self.layout.header {
            self.emit_header()?;
        }
        let current_session = if self.own_terminal_only {
            current_session_id()
        } else {
            None
        };

        for session in sessions(!self.all) {
            if self.own_terminal_only && current_session != Some(session.id) {
                continue;
            }
            if self.select.sessions && !session.user.is_empty() {
                self.emit_session(&session)?;
            }
        }
        Ok(())
    }
}

/// Enumerate WTS sessions. Without `all`, omit sessions that cannot contain a
/// login (listeners) so the default report only lists reachable login sessions.
fn sessions(all: bool) -> Vec<Session> {
    let Some(raw_sessions) = RawSessions::enumerate() else {
        return Vec::new();
    };

    raw_sessions
        .items()
        .iter()
        .filter(|raw| all || raw.State != WTSListen)
        .filter_map(|raw| {
            let line = if raw.pWinStationName.is_null() {
                String::new()
            } else {
                // SAFETY: WTS provides a NUL-terminated station name; its
                // documented maximum is 32 UTF-16 units.
                wide_string(unsafe { std::slice::from_raw_parts(raw.pWinStationName, 32) })
            };
            let info = query_session_info(raw.SessionId)?;
            let user = wide_string(&info.UserName);
            if !all && user.is_empty() {
                return None;
            }
            let host = query_string(raw.SessionId, WTSClientName).unwrap_or_default();

            Some(Session {
                id: raw.SessionId,
                line,
                user,
                host,
                logon_time: info.LogonTime,
                last_input_time: info.LastInputTime,
            })
        })
        .collect()
}

/// Owns a WTS-allocated session array.
struct RawSessions {
    buffer: WtsBuffer<WTS_SESSION_INFOW>,
    count: usize,
}

impl RawSessions {
    fn enumerate() -> Option<Self> {
        let mut buffer = ptr::null_mut();
        let mut count = 0;
        // SAFETY: the out-pointers are writable and the server handle is the
        // documented sentinel for the local machine.
        let result = unsafe {
            WTSEnumerateSessionsW(
                WTS_CURRENT_SERVER_HANDLE,
                0,
                1,
                &raw mut buffer,
                &raw mut count,
            )
        };
        if result == 0 || buffer.is_null() {
            return None;
        }

        Some(Self {
            buffer: WtsBuffer(buffer),
            count: count as usize,
        })
    }

    fn items(&self) -> &[WTS_SESSION_INFOW] {
        // SAFETY: WTS returned `count` consecutive records in this buffer.
        unsafe { std::slice::from_raw_parts(self.buffer.0, self.count) }
    }
}

/// Copy a fixed-size WTS session record out of its WTS-owned buffer.
fn query_session_info(session_id: u32) -> Option<WTSINFOW> {
    let mut buffer = ptr::null_mut();
    let mut byte_len = 0;
    // SAFETY: the out-pointers are writable and the server handle is the
    // documented sentinel for the local machine.
    let result = unsafe {
        WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            session_id,
            WTSSessionInfo,
            &raw mut buffer,
            &raw mut byte_len,
        )
    };
    if result == 0 || byte_len < size_of::<WTSINFOW>() as u32 {
        return None;
    }
    // WTS allocates WTSINFO on a suitable alignment for its fields.
    #[allow(clippy::cast_ptr_alignment)]
    let buffer = WtsBuffer(buffer.cast::<WTSINFOW>());
    unsafe { buffer.0.as_ref().copied() }
}

fn query_string(session_id: u32, info_class: WTS_INFO_CLASS) -> Option<String> {
    let mut buffer = ptr::null_mut();
    let mut byte_len = 0;
    // SAFETY: the out-pointers are writable and the server handle is the
    // documented sentinel for the local machine.
    let result = unsafe {
        WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            session_id,
            info_class,
            &raw mut buffer,
            &raw mut byte_len,
        )
    };
    if result == 0 || buffer.is_null() {
        return None;
    }
    let buffer = WtsBuffer(buffer.cast::<u16>());
    // SAFETY: on success `byte_len` is the size of the returned UTF-16 buffer.
    let units = unsafe { std::slice::from_raw_parts(buffer.0, byte_len as usize / 2) };
    Some(wide_string(units.split(|&unit| unit == 0).next()?))
}

fn wide_string(units: &[u16]) -> String {
    let length = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len());
    OsString::from_wide(&units[..length])
        .to_string_lossy()
        .into_owned()
}

fn current_session_id() -> Option<u32> {
    let process_id = unsafe { GetCurrentProcessId() };
    let mut session_id = 0;
    // SAFETY: the output pointer is writable and the process ID is read-only.
    let result = unsafe { ProcessIdToSessionId(process_id, &raw mut session_id) };
    (result != 0).then_some(session_id)
}

fn windows_timestamp_to_unix(timestamp: i64) -> i64 {
    ((timestamp as i128 - 116_444_736_000_000_000) / 10_000_000) as i64
}

fn optional_timestamp(timestamp: i64) -> Option<i64> {
    (timestamp != 0).then(|| windows_timestamp_to_unix(timestamp))
}

impl Who {
    fn emit_session(&self, session: &Session) -> UResult<()> {
        let host = if self.resolve_hosts {
            canonicalize_host(&session.host)?
        } else {
            session.host.clone()
        };
        let note = if host.is_empty() {
            host
        } else {
            format!("({host})")
        };
        let idle = match optional_timestamp(session.last_input_time) {
            Some(last_touched) => format_idle(last_touched, 0),
            None => "  ?".into(),
        };
        let time = optional_timestamp(session.logon_time)
            .and_then(|timestamp| time::OffsetDateTime::from_unix_timestamp(timestamp).ok())
            .map(format_timestamp)
            .unwrap_or_default();

        self.emit_row(&Row {
            user: &session.user,
            write_state: '?',
            line: &session.line,
            time: &time,
            idle: &idle,
            pid: &format!("{}", session.id),
            note: &note,
            exit: "",
        })
    }
}

fn canonicalize_host(host: &str) -> UResult<String> {
    if host.is_empty() {
        return Ok(String::new());
    }
    let Ok(address) = host.parse() else {
        return Ok(host.to_owned());
    };

    dns_lookup::lookup_addr(&address)
        .map_err(|_| {
            translate!(
                "who-canonicalize-error",
                "host" => host.split(':').next().unwrap_or(host).quote()
            )
        })
        .map_err(|message| USimpleError::new(1, message))
}
