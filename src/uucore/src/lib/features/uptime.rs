// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore gettime BOOTTIME clockid boottime nusers loadavg getloadavg

//! Provides functions to get system uptime, number of users and load average.

// The code was originally written in uu_uptime
// (https://github.com/uutils/coreutils/blob/main/src/uu/uptime/src/uptime.rs)
// but was eventually moved here.
// See https://github.com/uutils/coreutils/pull/7289 for discussion.

use crate::error::{UError, UResult};
use crate::translate;
use chrono::Local;
use libc::time_t;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UptimeError {
    #[error("{}", translate!("uptime-lib-error-system-uptime"))]
    SystemUptime,
    #[error("{}", translate!("uptime-lib-error-system-loadavg"))]
    SystemLoadavg,
    #[error("{}", translate!("uptime-lib-error-windows-loadavg"))]
    WindowsLoadavg,
    #[error("{}", translate!("uptime-lib-error-boot-time"))]
    BootTime,
}

impl UError for UptimeError {
    fn code(&self) -> i32 {
        1
    }
}

/// Returns the formatted time string, e.g. "12:34:56"
pub fn get_formatted_time() -> String {
    Local::now().time().format("%H:%M:%S").to_string()
}

/// Get the system uptime
///
/// # Arguments
///
/// boot_time: Option<time_t> - Manually specify the boot time, or None to try to get it from the system.
///
/// # Returns
///
/// Returns a UResult with the uptime in seconds if successful, otherwise an UptimeError.
#[cfg(target_os = "openbsd")]
pub fn get_uptime(_boot_time: Option<time_t>) -> UResult<i64> {
    use libc::CLOCK_BOOTTIME;
    use libc::clock_gettime;

    use libc::c_int;
    use libc::timespec;

    let mut tp: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let raw_tp = &mut tp as *mut timespec;

    // OpenBSD prototype: clock_gettime(clk_id: ::clockid_t, tp: *mut ::timespec) -> ::c_int;
    let ret: c_int = unsafe { clock_gettime(CLOCK_BOOTTIME, raw_tp) };

    if ret == 0 {
        #[cfg(target_pointer_width = "64")]
        let uptime: i64 = tp.tv_sec;
        #[cfg(not(target_pointer_width = "64"))]
        let uptime: i64 = tp.tv_sec.into();

        Ok(uptime)
    } else {
        Err(UptimeError::SystemUptime)?
    }
}

/// Get the system uptime
///
/// # Arguments
///
/// boot_time: Option<time_t> - Manually specify the boot time, or None to try to get it from the system.
///
/// # Returns
///
/// Returns a UResult with the uptime in seconds if successful, otherwise an UptimeError.
#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
pub fn get_uptime(boot_time: Option<time_t>) -> UResult<i64> {
    use crate::utmpx::Utmpx;
    use libc::BOOT_TIME;
    use std::fs::File;
    use std::io::Read;

    let mut proc_uptime_s = String::new();

    let proc_uptime = File::open("/proc/uptime")
        .ok()
        .and_then(|mut f| f.read_to_string(&mut proc_uptime_s).ok())
        .and_then(|_| proc_uptime_s.split_whitespace().next())
        .and_then(|s| s.split('.').next().unwrap_or("0").parse::<i64>().ok());

    if let Some(uptime) = proc_uptime {
        return Ok(uptime);
    }

    let boot_time = boot_time.or_else(|| {
        let records = Utmpx::iter_all_records();
        for line in records {
            match line.record_type() {
                BOOT_TIME => {
                    let dt = line.login_time();
                    if dt.unix_timestamp() > 0 {
                        return Some(dt.unix_timestamp() as time_t);
                    }
                }
                _ => continue,
            }
        }
        None
    });

    if let Some(t) = boot_time {
        let now = Local::now().timestamp();
        #[cfg(target_pointer_width = "64")]
        let boottime: i64 = t;
        #[cfg(not(target_pointer_width = "64"))]
        let boottime: i64 = t.into();
        if now < boottime {
            Err(UptimeError::BootTime)?;
        }
        return Ok(now - boottime);
    }

    Err(UptimeError::SystemUptime)?
}

/// Get the system uptime
///
/// # Arguments
///
/// boot_time will be ignored, pass None.
///
/// # Returns
///
/// Returns a UResult with the uptime in seconds if successful, otherwise an UptimeError.
#[cfg(windows)]
pub fn get_uptime(_boot_time: Option<time_t>) -> UResult<i64> {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    // SAFETY: always return u32
    let uptime = unsafe { GetTickCount() };
    Ok(uptime as i64 / 1000)
}

/// Get the system uptime in a human-readable format
///
/// # Arguments
///
/// boot_time: Option<time_t> - Manually specify the boot time, or None to try to get it from the system.
///
/// # Returns
///
/// Returns a UResult with the uptime in a human-readable format(e.g. "1 day, 3:45") if successful, otherwise an UptimeError.
#[inline]
pub fn get_formatted_uptime(boot_time: Option<time_t>) -> UResult<String> {
    let up_secs = get_uptime(boot_time)?;

    if up_secs < 0 {
        Err(UptimeError::SystemUptime)?;
    }
    let up_days = up_secs / 86400;
    let up_hours = (up_secs - (up_days * 86400)) / 3600;
    let up_mins = (up_secs - (up_days * 86400) - (up_hours * 3600)) / 60;

    Ok(translate!(
        "uptime-format",
        "days" => up_days,
        "time" => format!("{up_hours:02}:{up_mins:02}")
    ))
}

/// Get the number of users currently logged in
///
/// # Returns
///
/// Returns the number of users currently logged in if successful, otherwise 0.
#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
// see: https://gitlab.com/procps-ng/procps/-/blob/4740a0efa79cade867cfc7b32955fe0f75bf5173/library/uptime.c#L63-L115
pub fn get_nusers() -> usize {
    use crate::utmpx::Utmpx;
    use libc::USER_PROCESS;

    let mut num_user = 0;
    Utmpx::iter_all_records().for_each(|ut| {
        if ut.record_type() == USER_PROCESS {
            num_user += 1;
        }
    });
    num_user
}

/// Get the number of users currently logged in
///
/// # Returns
///
/// Returns the number of users currently logged in if successful, otherwise 0
#[cfg(target_os = "openbsd")]
pub fn get_nusers(file: &str) -> usize {
    use utmp_classic::{UtmpEntry, parse_from_path};

    let entries = match parse_from_path(file) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    if entries.is_empty() {
        return 0;
    }

    // Count entries that have a non-empty user field
    entries
        .iter()
        .filter_map(|entry| match entry {
            UtmpEntry::UTMP { user, .. } if !user.is_empty() => Some(()),
            _ => None,
        })
        .count()
}

/// Get the number of users currently logged in
///
/// # Returns
///
/// Returns the number of users currently logged in if successful, otherwise 0
#[cfg(target_os = "windows")]
pub fn get_nusers() -> usize {
    use std::ptr;
    use windows_sys::Win32::System::RemoteDesktop::*;

    let mut num_user = 0;

    // SAFETY: WTS_CURRENT_SERVER_HANDLE is a valid handle
    unsafe {
        let mut session_info_ptr = ptr::null_mut();
        let mut session_count = 0;

        let result = WTSEnumerateSessionsW(
            WTS_CURRENT_SERVER_HANDLE,
            0,
            1,
            &mut session_info_ptr,
            &mut session_count,
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
                &mut buffer,
                &mut bytes_returned,
            );
            if result == 0 || buffer.is_null() {
                continue;
            }

            let username = if !buffer.is_null() {
                let cstr = std::ffi::CStr::from_ptr(buffer as *const i8);
                cstr.to_string_lossy().to_string()
            } else {
                String::new()
            };
            if !username.is_empty() {
                num_user += 1;
            }

            WTSFreeMemory(buffer as _);
        }

        WTSFreeMemory(session_info_ptr as _);
    }

    num_user
}

/// Format the number of users to a human-readable string
///
/// # Returns
///
/// e.g. "0 users", "1 user", "2 users"
#[inline]
pub fn format_nusers(n: usize) -> String {
    translate!(
        "uptime-user-count",
        "count" => n
    )
}

/// Get the number of users currently logged in in a human-readable format
///
/// # Returns
///
/// e.g. "0 user", "1 user", "2 users"
#[inline]
pub fn get_formatted_nusers() -> String {
    #[cfg(not(target_os = "openbsd"))]
    return format_nusers(get_nusers());

    #[cfg(target_os = "openbsd")]
    format_nusers(get_nusers("/var/run/utmp"))
}

/// Get the system load average
///
/// # Returns
///
/// Returns a UResult with the load average if successful, otherwise an UptimeError.
/// The load average is a tuple of three floating point numbers representing the 1-minute, 5-minute, and 15-minute load averages.
#[cfg(unix)]
pub fn get_loadavg() -> UResult<(f64, f64, f64)> {
    use crate::libc::c_double;
    use libc::getloadavg;

    let mut avg: [c_double; 3] = [0.0; 3];
    // SAFETY: checked whether it returns -1
    let loads: i32 = unsafe { getloadavg(avg.as_mut_ptr(), 3) };

    if loads == -1 {
        Err(UptimeError::SystemLoadavg)?
    } else {
        Ok((avg[0], avg[1], avg[2]))
    }
}

/// Get the system load average
/// Windows does not have an equivalent to the load average on Unix-like systems.
///
/// # Returns
///
/// Returns a UResult with an UptimeError.
#[cfg(windows)]
pub fn get_loadavg() -> UResult<(f64, f64, f64)> {
    Err(UptimeError::WindowsLoadavg)?
}

/// Get the system load average in a human-readable format
///
/// # Returns
///
/// Returns a UResult with the load average in a human-readable format if successful, otherwise an UptimeError.
/// e.g. "load average: 0.00, 0.00, 0.00"
#[inline]
pub fn get_formatted_loadavg() -> UResult<String> {
    let loadavg = get_loadavg()?;
    Ok(translate!(
        "uptime-lib-format-loadavg",
        "avg1" => format!("{:.2}", loadavg.0),
        "avg5" => format!("{:.2}", loadavg.1),
        "avg15" => format!("{:.2}", loadavg.2),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale;

    #[test]
    fn test_format_nusers() {
        unsafe {
            std::env::set_var("LANG", "en_US.UTF-8");
        }
        let _ = locale::setup_localization("uptime");
        assert_eq!("0 users", format_nusers(0));
        assert_eq!("1 user", format_nusers(1));
        assert_eq!("2 users", format_nusers(2));
    }
}
