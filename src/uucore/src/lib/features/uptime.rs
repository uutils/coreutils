// spell-checker:ignore gettime BOOTTIME clockid boottime formated nusers loadavg getloadavg

use crate::error::{UResult, USimpleError};
use chrono::Local;
use libc::time_t;

#[cfg(target_os = "linux")]
extern "C" {
    pub fn sd_booted() -> libc::c_int;
    pub fn sd_get_sessions(sessions: *mut *mut *mut libc::c_char) -> libc::c_int;
    pub fn sd_session_get_class(
        session: *const libc::c_char,
        class: *mut *mut libc::c_char,
    ) -> libc::c_int;
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
        Err(USimpleError::new(1, "could not retrieve system uptime"))
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

    match proc_uptime {
        None => {
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
            match boot_time {
                Some(t) => {
                    let now = Local::now().timestamp();
                    #[cfg(target_pointer_width = "64")]
                    let boottime: i64 = t;
                    #[cfg(not(target_pointer_width = "64"))]
                    let boottime: i64 = t.into();
                    Ok(now - boottime)
                }
                None => Err(USimpleError::new(1, "could not retrieve system uptime"))?,
            }
        }
        Some(time) => Ok(time),
    }
}

#[cfg(windows)]
pub fn get_uptime(_boot_time: Option<time_t>) -> UResult<i64> {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    unsafe { Ok(GetTickCount() as i64) }
}

/// Returns the formatted uptime string, e.g. "1 day, 3:45"
#[inline]
pub fn get_formated_uptime(boot_time: Option<time_t>) -> UResult<String> {
    let up_secs = get_uptime(boot_time)?;

    if up_secs < 0 {
        return Err(USimpleError::new(1, "could not retrieve system uptime"));
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

#[cfg(target_os = "linux")]
pub fn get_nusers_systemd() -> UResult<usize> {
    use crate::libc::*;
    use std::ffi::CStr;
    use std::ptr;

    unsafe {
        // systemd
        if sd_booted() > 0 {
            let mut sessions_list: *mut *mut c_char = ptr::null_mut();
            let mut num_user = 0;
            let sessions = sd_get_sessions(&mut sessions_list);

            if sessions > 0 {
                for i in 0..sessions {
                    let mut class: *mut c_char = ptr::null_mut();

                    if sd_session_get_class(
                        *sessions_list.add(i as usize) as *const c_char,
                        &mut class,
                    ) < 0
                    {
                        continue;
                    }
                    if CStr::from_ptr(class).to_str().unwrap().starts_with("user") {
                        num_user += 1;
                    }
                    free(class as *mut c_void);
                }
            }

            for i in 0..sessions {
                free(*sessions_list.add(i as usize) as *mut c_void);
            }
            free(sessions_list as *mut c_void);

            return Ok(num_user);
        }
    }
    Err(USimpleError::new(
        1,
        "could not retrieve number of logged users",
    ))
}

#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
// see: https://gitlab.com/procps-ng/procps/-/blob/4740a0efa79cade867cfc7b32955fe0f75bf5173/library/uptime.c#L63-L115
pub fn get_nusers() -> usize {
    use crate::utmpx::Utmpx;
    use libc::USER_PROCESS;

    #[cfg(target_os = "linux")]
    // systemd
    if let Ok(systemd_users) = get_nusers_systemd() {
        return systemd_users;
    }

    // utmpx
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
        Err(USimpleError::new(1, "could not retrieve load average"))
    } else {
        Ok((avg[0], avg[1], avg[2]))
    }
}

/// Windows does not seem to have anything similar.
#[cfg(windows)]
pub fn get_loadavg() -> UResult<(f64, f64, f64)> {
    Err(USimpleError::new(1, "could not retrieve load average"))
}

#[inline]
pub fn get_formatted_loadavg() -> UResult<String> {
    let loadavg = get_loadavg()?;
    Ok(format!(
        "load average: {:.2}, {:.2}, {:.2}",
        loadavg.0, loadavg.1, loadavg.2
    ))
}
