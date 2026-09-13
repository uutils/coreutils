// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore nusers loadavg INFOW

//! Windows implementation of the platform side of `uucore::uptime`:
//! `GetTickCount64` for the uptime, WTS session enumeration for the user
//! count, and no load average.

use super::UptimeError;
use crate::error::UResult;
use libc::time_t;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::ptr;
use windows_sys::Win32::System::RemoteDesktop::{
    WTS_CURRENT_SERVER_HANDLE, WTS_SESSION_INFOW, WTSEnumerateSessionsW, WTSFreeMemory,
    WTSQuerySessionInformationW, WTSUserName,
};

/// Get the system uptime
///
/// # Arguments
///
/// boot_time will be ignored, pass None.
///
/// # Returns
///
/// Returns a UResult with the uptime in seconds if successful, otherwise an UptimeError.
#[allow(clippy::unnecessary_wraps, reason = "needed on some platforms")]
pub fn get_uptime(_boot_time: Option<time_t>) -> UResult<i64> {
    // GetTickCount64 (unlike GetTickCount) does not wrap after 49.7 days.
    use windows_sys::Win32::System::SystemInformation::GetTickCount64;
    // SAFETY: no preconditions; always returns milliseconds since boot
    let uptime = unsafe { GetTickCount64() };
    Ok((uptime / 1000) as i64)
}

/// Owns a WTS-allocated buffer, freeing it on every exit path.
struct WtsBuffer<T>(*mut T);

impl<T> Drop for WtsBuffer<T> {
    fn drop(&mut self) {
        // SAFETY: the pointer came from a successful WTS allocation and is
        // freed exactly once, here.
        unsafe { WTSFreeMemory(self.0.cast()) };
    }
}

/// The sessions of the local server, enumerated once and freed on drop.
///
/// Shaped for extraction into a shared module should another consumer appear
/// (e.g. a WTS-backed `users`).
struct Sessions {
    buffer: WtsBuffer<WTS_SESSION_INFOW>,
    count: usize,
}

impl Sessions {
    /// Enumerate the sessions of the local server, or `None` if the WTS API
    /// reports failure.
    fn enumerate() -> Option<Self> {
        let mut ptr = ptr::null_mut();
        let mut count = 0;
        // SAFETY: WTS_CURRENT_SERVER_HANDLE is always valid and the two
        // out-pointers are valid writable locations.
        let result = unsafe {
            WTSEnumerateSessionsW(
                WTS_CURRENT_SERVER_HANDLE,
                0,
                1,
                &raw mut ptr,
                &raw mut count,
            )
        };
        // A null buffer is not documented as impossible when the call reports
        // success with no sessions, and `ids()` may not build a slice from it.
        if result == 0 || ptr.is_null() {
            return None;
        }
        Some(Self {
            buffer: WtsBuffer(ptr),
            count: count as usize,
        })
    }

    /// The session identifiers, in enumeration order.
    fn ids(&self) -> impl Iterator<Item = u32> {
        // SAFETY: on success WTSEnumerateSessionsW produced an array of
        // `count` entries, owned by `self.buffer` and freed only on drop.
        let infos = unsafe { std::slice::from_raw_parts(self.buffer.0, self.count) };
        infos.iter().map(|info| info.SessionId)
    }
}

/// The user name a session is logged on as: `None` when the query fails,
/// `Some("")` when nobody is logged on to the session (services, listeners).
fn session_user_name(session_id: u32) -> Option<OsString> {
    let mut buffer: *mut u16 = ptr::null_mut();
    let mut byte_len = 0;
    // SAFETY: WTS_CURRENT_SERVER_HANDLE is always valid and the two
    // out-pointers are valid writable locations.
    let result = unsafe {
        WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            session_id,
            WTSUserName,
            &raw mut buffer,
            &raw mut byte_len,
        )
    };
    if result == 0 || buffer.is_null() {
        return None;
    }
    let buffer = WtsBuffer(buffer);
    // SAFETY: on success the buffer holds byte_len / 2 u16 units (the UTF-16
    // name including its terminating NUL), owned by `buffer` until drop.
    let units = unsafe { std::slice::from_raw_parts(buffer.0, byte_len as usize / 2) };
    let name = units.split(|&unit| unit == 0).next().unwrap_or(&[]);
    Some(OsString::from_wide(name))
}

/// Get the number of users currently logged in
///
/// # Returns
///
/// Returns the number of users currently logged in if successful, otherwise 0
pub fn get_nusers() -> usize {
    let Some(sessions) = Sessions::enumerate() else {
        return 0;
    };
    sessions
        .ids()
        .filter(|&id| session_user_name(id).is_some_and(|name| !name.is_empty()))
        .count()
}

/// Get the number of users from the default system source, for
/// [`super::get_formatted_nusers`].
pub(crate) fn default_nusers() -> usize {
    get_nusers()
}

/// Get the system load average
/// Windows does not have an equivalent to the load average on Unix-like systems.
///
/// # Returns
///
/// Returns a UResult with an UptimeError.
pub fn get_loadavg() -> UResult<(f64, f64, f64)> {
    Err(UptimeError::WindowsLoadavg)?
}
