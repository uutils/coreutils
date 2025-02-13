// spell-checker:ignore gettime BOOTTIME clockid boottime formated nusers loadavg getloadavg

use crate::error::{UError, UResult};
use chrono::Local;
use libc::time_t;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UptimeError {
    #[error("could not retrieve system uptime")]
    SystemUptime,
    #[error("could not retrieve system load average")]
    SystemLoadavg,
    #[error("boot time larger than current time")]
    BootTime,
}

impl UError for UptimeError {
    fn code(&self) -> i32 {
        1
    }
}

pub fn get_formatted_time() -> String {
    Local::now().time().format("%H:%M:%S").to_string()
}

#[cfg(target_os = "openbsd")]
pub fn get_uptime(_boot_time: Option<time_t>) -> UResult<i64> {
    use libc::clock_gettime;
    use libc::CLOCK_BOOTTIME;

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
        Err(UptimeError::SystemUptime)
    }
}

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

#[cfg(windows)]
pub fn get_uptime(_boot_time: Option<time_t>) -> UResult<i64> {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    let uptime = unsafe { GetTickCount() };
    if uptime < 0 {
        Err(UptimeError::SystemUptime)?;
    }
    Ok(uptime as i64)
}

/// Returns the formatted uptime string, e.g. "1 day, 3:45"
#[inline]
pub fn get_formated_uptime(boot_time: Option<time_t>) -> UResult<String> {
    let up_secs = get_uptime(boot_time)?;

    if up_secs < 0 {
        Err(UptimeError::SystemUptime)?;
    }
    let up_days = up_secs / 86400;
    let up_hours = (up_secs - (up_days * 86400)) / 3600;
    let up_mins = (up_secs - (up_days * 86400) - (up_hours * 3600)) / 60;
    match up_days.cmp(&1) {
        std::cmp::Ordering::Equal => Ok(format!("{up_days:1} day, {up_hours:2}:{up_mins:02}")),
        std::cmp::Ordering::Greater => Ok(format!("{up_days:1} days {up_hours:2}:{up_mins:02}")),
        _ => Ok(format!("{up_hours:2}:{up_mins:02}")),
    }
}

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

#[cfg(target_os = "openbsd")]
pub fn get_nusers(file: &str) -> usize {
    use utmp_classic::{parse_from_path, UtmpEntry};

    let mut nusers = 0;

    let entries = match parse_from_path(file) {
        Some(e) => e,
        None => return 0,
    };

    for entry in entries {
        if let UtmpEntry::UTMP {
            line: _,
            user,
            host: _,
            time: _,
        } = entry
        {
            if !user.is_empty() {
                nusers += 1;
            }
        }
    }
    nusers
}

#[cfg(target_os = "windows")]
pub fn get_nusers() -> usize {
    use std::ptr;
    use windows_sys::Win32::System::RemoteDesktop::*;

    let mut num_user = 0;

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

#[inline]
pub fn format_nusers(nusers: usize) -> String {
    match nusers {
        0 => "0 user".to_string(),
        1 => "1 user".to_string(),
        _ => format!("{} users", nusers),
    }
}

#[inline]
pub fn get_formatted_nusers() -> String {
    #[cfg(not(target_os = "openbsd"))]
    return format_nusers(get_nusers());

    #[cfg(target_os = "openbsd")]
    format_nusers(get_nusers("/var/run/utmp"))
}

#[cfg(unix)]
pub fn get_loadavg() -> UResult<(f64, f64, f64)> {
    use crate::libc::c_double;
    use libc::getloadavg;

    let mut avg: [c_double; 3] = [0.0; 3];
    let loads: i32 = unsafe { getloadavg(avg.as_mut_ptr(), 3) };

    if loads == -1 {
        Err(UptimeError::SystemLoadavg)?
    } else {
        Ok((avg[0], avg[1], avg[2]))
    }
}

/// Windows does not seem to have anything similar.
#[cfg(windows)]
pub fn get_loadavg() -> UResult<(f64, f64, f64)> {
    Err(UptimeError::SystemLoadavg)?
}

#[inline]
pub fn get_formatted_loadavg() -> UResult<String> {
    let loadavg = get_loadavg()?;
    Ok(format!(
        "load average: {:.2}, {:.2}, {:.2}",
        loadavg.0, loadavg.1, loadavg.2
    ))
}
