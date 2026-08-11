// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore nusers loadavg

//! Windows implementation of the platform side of `uucore::uptime`:
//! `GetTickCount64` for the uptime, WTS session enumeration for the user
//! count, and no load average.

use super::UptimeError;
use crate::error::UResult;
use libc::time_t;

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

/// Get the number of users currently logged in
///
/// # Returns
///
/// Returns the number of users currently logged in if successful, otherwise 0
pub fn get_nusers() -> usize {
    use std::ptr;
    use windows_sys::Win32::System::RemoteDesktop::{
        WTS_CURRENT_SERVER_HANDLE, WTSEnumerateSessionsW, WTSFreeMemory,
        WTSQuerySessionInformationW,
    };

    let mut num_user = 0;

    // SAFETY: WTS_CURRENT_SERVER_HANDLE is a valid handle
    unsafe {
        let mut session_info_ptr = ptr::null_mut();
        let mut session_count = 0;

        let result = WTSEnumerateSessionsW(
            WTS_CURRENT_SERVER_HANDLE,
            0,
            1,
            &raw mut session_info_ptr,
            &raw mut session_count,
        );
        if result == 0 {
            return 0;
        }

        let sessions = std::slice::from_raw_parts(session_info_ptr, session_count as usize);

        for session in sessions {
            let mut buffer: *mut u16 = ptr::null_mut();
            let mut bytes_returned = 0;

            let result = WTSQuerySessionInformationW(
                WTS_CURRENT_SERVER_HANDLE,
                session.SessionId,
                5,
                &raw mut buffer,
                &raw mut bytes_returned,
            );
            if result == 0 || buffer.is_null() {
                continue;
            }

            // The buffer is UTF-16 (WTSUserNameW); checking it byte-wise as a
            // C string would misread names whose first code unit has a zero
            // low byte (e.g. U+AC00) as empty.
            if *buffer != 0 {
                num_user += 1;
            }

            WTSFreeMemory(buffer.cast());
        }

        WTSFreeMemory(session_info_ptr.cast());
    }

    num_user
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
