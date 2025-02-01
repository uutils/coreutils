// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore getloadavg behaviour loadavg uptime upsecs updays upmins uphours boottime nusers utmpxname gettime clockid

use chrono::{Local, TimeZone, Utc};
use clap::ArgMatches;
use std::io;
use thiserror::Error;
use uucore::error::UError;
use uucore::libc::time_t;

use uucore::error::{UResult, USimpleError};

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command, ValueHint};

use uucore::{format_usage, help_about, help_usage};

#[cfg(target_os = "openbsd")]
use utmp_classic::{parse_from_path, UtmpEntry};
#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
use uucore::utmpx::*;

const ABOUT: &str = help_about!("uptime.md");
const USAGE: &str = help_usage!("uptime.md");
pub mod options {
    pub static SINCE: &str = "since";
    pub static PATH: &str = "path";
}

#[cfg(unix)]
use uucore::libc::getloadavg;

#[cfg(windows)]
extern "C" {
    fn GetTickCount() -> u32;
}

#[derive(Debug, Error)]
pub enum UptimeError {
    // io::Error wrapper
    #[error("couldn't get boot time: {0}")]
    IoErr(#[from] io::Error),

    #[error("couldn't get boot time: Is a directory")]
    TargetIsDir,

    #[error("couldn't get boot time: Illegal seek")]
    TargetIsFifo,
    #[error("extra operand '{0}'")]
    ExtraOperandError(String),
}
impl UError for UptimeError {
    fn code(&self) -> i32 {
        1
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    #[cfg(windows)]
    return default_uptime(&matches);

    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use uucore::error::set_exit_code;
        use uucore::show_error;

        let argument = matches.get_many::<OsString>(options::PATH);

        // Switches to default uptime behaviour if there is no argument
        if argument.is_none() {
            return default_uptime(&matches);
        }
        let mut arg_iter = argument.unwrap();

        let file_path = arg_iter.next().unwrap();
        if let Some(path) = arg_iter.next() {
            // Uptime doesn't attempt to calculate boot time if there is extra arguments.
            // Its a fatal error
            show_error!(
                "{}",
                UptimeError::ExtraOperandError(path.to_owned().into_string().unwrap())
            );
            set_exit_code(1);
            return Ok(());
        }

        uptime_with_file(file_path)
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SINCE)
                .short('s')
                .long(options::SINCE)
                .help("system up since")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PATH)
                .help("file to search boot time from")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(ValueHint::AnyPath),
        )
}

#[cfg(unix)]
fn uptime_with_file(file_path: &std::ffi::OsString) -> UResult<()> {
    use std::fs;
    use std::os::unix::fs::FileTypeExt;
    use uucore::error::set_exit_code;
    use uucore::show_error;

    // Uptime will print loadavg and time to stderr unless we encounter an extra operand.
    let mut non_fatal_error = false;

    // process_utmpx_from_file() doesn't detect or report failures, we check if the path is valid
    // before proceeding with more operations.
    let md_res = fs::metadata(file_path);
    if let Ok(md) = md_res {
        if md.is_dir() {
            show_error!("{}", UptimeError::TargetIsDir);
            non_fatal_error = true;
            set_exit_code(1);
        }
        if md.file_type().is_fifo() {
            show_error!("{}", UptimeError::TargetIsFifo);
            non_fatal_error = true;
            set_exit_code(1);
        }
    } else if let Err(e) = md_res {
        non_fatal_error = true;
        set_exit_code(1);
        show_error!("{}", UptimeError::IoErr(e));
    }
    // utmpxname() returns an -1 , when filename doesn't end with 'x' or its too long.
    // Reference: `<https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/utmpxname.3.html>`

    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = file_path.as_os_str().as_bytes();

        if bytes[bytes.len() - 1] != b'x' {
            show_error!("couldn't get boot time");
            print_time();
            print!("up ???? days ??:??,");
            print_nusers(Some(0))?;
            print_loadavg();
            set_exit_code(1);
            return Ok(());
        }
    }

    if non_fatal_error {
        print_time();
        print!("up ???? days ??:??,");
        print_nusers(Some(0))?;
        print_loadavg();
        return Ok(());
    }

    print_time();
    let user_count;

    #[cfg(not(target_os = "openbsd"))]
    {
        let (boot_time, count) = process_utmpx(Some(file_path));
        if let Some(time) = boot_time {
            print_uptime(Some(time))?;
        } else {
            show_error!("couldn't get boot time");
            set_exit_code(1);

            print!("up ???? days ??:??,");
        }
        user_count = count;
    }

    #[cfg(target_os = "openbsd")]
    {
        user_count = process_utmp_from_file(file_path.to_str().expect("invalid utmp path file"));

        let upsecs = get_uptime(None);
        if upsecs < 0 {
            show_error!("couldn't get boot time");
            set_exit_code(1);

            print!("up ???? days ??:??,");
        } else {
            print_uptime(Some(upsecs))?;
        }
    }

    print_nusers(Some(user_count))?;
    print_loadavg();

    Ok(())
}

/// Default uptime behaviour i.e. when no file argument is given.
fn default_uptime(matches: &ArgMatches) -> UResult<()> {
    #[cfg(unix)]
    #[cfg(not(target_os = "openbsd"))]
    let (boot_time, _) = process_utmpx(None);

    #[cfg(target_os = "openbsd")]
    let uptime = get_uptime(None);
    #[cfg(unix)]
    #[cfg(not(target_os = "openbsd"))]
    let uptime = get_uptime(boot_time);
    #[cfg(target_os = "windows")]
    let uptime = get_uptime(None);

    if matches.get_flag(options::SINCE) {
        let initial_date = Local
            .timestamp_opt(Utc::now().timestamp() - uptime, 0)
            .unwrap();
        println!("{}", initial_date.format("%Y-%m-%d %H:%M:%S"));
        return Ok(());
    }

    print_time();
    print_uptime(None)?;
    print_nusers(None)?;
    print_loadavg();

    Ok(())
}

#[cfg(unix)]
fn get_loadavg() -> (f64, f64, f64) {
    use uucore::libc::c_double;

    let mut avg: [c_double; 3] = [0.0; 3];
    let loads: i32 = unsafe { getloadavg(avg.as_mut_ptr(), 3) };

    if loads == -1 {
        (-1.0, -1.0, -1.0)
    } else {
        (avg[0], avg[1], avg[2])
    }
}

/// Windows does not seem to have anything similar.
#[cfg(windows)]
fn get_loadavg() -> (f64, f64, f64) {
    (-1.0, -1.0, -1.0)
}

#[inline]
fn get_formatted_loadavg() -> UResult<String> {
    let loadavg = get_loadavg();
    if loadavg.0 < 0.0 || loadavg.1 < 0.0 || loadavg.2 < 0.0 {
        Err(USimpleError::new(1, "could not retrieve uptime"))
    } else {
        Ok(format!(
            "load average: {:.2}, {:.2}, {:.2}",
            loadavg.0, loadavg.1, loadavg.2
        ))
    }
}

#[inline]
fn print_loadavg() {
    match get_formatted_loadavg() {
        Err(_) => {}
        Ok(s) => println!("{}", s),
    }
}

#[cfg(target_os = "openbsd")]
fn process_utmp_from_file(file: &str) -> usize {
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

#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
fn process_utmpx(file: Option<&std::ffi::OsString>) -> (Option<time_t>, usize) {
    let mut nusers = 0;
    let mut boot_time = None;

    let records = match file {
        Some(f) => Utmpx::iter_all_records_from(f),
        None => Utmpx::iter_all_records(),
    };

    for line in records {
        match line.record_type() {
            USER_PROCESS => nusers += 1,
            BOOT_TIME => {
                let dt = line.login_time();
                if dt.unix_timestamp() > 0 {
                    boot_time = Some(dt.unix_timestamp() as time_t);
                }
            }
            _ => continue,
        }
    }
    (boot_time, nusers)
}

#[cfg(windows)]
fn process_utmpx() -> (Option<time_t>, usize) {
    (None, 0) // TODO: change 0 to number of users
}

fn print_nusers(nusers: Option<usize>) -> UResult<()> {
    print!(
        "{},  ",
        match nusers {
            None => {
                get_formatted_nusers()
            }
            Some(nusers) => {
                format_nusers(nusers)
            }
        }
    );
    Ok(())
}

fn print_time() {
    print!(" {}  ", get_formatted_time());
}

fn print_uptime(boot_time: Option<time_t>) -> UResult<()> {
    print!("up  {},  ", get_formated_uptime(boot_time)?);
    Ok(())
}

fn get_formatted_time() -> String {
    Local::now().time().format("%H:%M:%S").to_string()
}

#[cfg(target_os = "openbsd")]
pub fn get_uptime(_boot_time: Option<time_t>) -> i64 {
    use uucore::libc::clock_gettime;
    use uucore::libc::CLOCK_BOOTTIME;

    use uucore::libc::c_int;
    use uucore::libc::timespec;

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

        uptime
    } else {
        -1
    }
}

#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
pub fn get_uptime(boot_time: Option<time_t>) -> i64 {
    use std::fs::File;
    use std::io::Read;

    let mut proc_uptime_s = String::new();

    let proc_uptime = File::open("/proc/uptime")
        .ok()
        .and_then(|mut f| f.read_to_string(&mut proc_uptime_s).ok())
        .and_then(|_| proc_uptime_s.split_whitespace().next())
        .and_then(|s| s.split('.').next().unwrap_or("0").parse().ok());

    proc_uptime.unwrap_or_else(|| {
        let boot_time = boot_time.or_else(|| {
            let (boot_time, _) = process_utmpx(None);
            boot_time
        });
        match boot_time {
            Some(t) => {
                let now = Local::now().timestamp();
                #[cfg(target_pointer_width = "64")]
                let boottime: i64 = t;
                #[cfg(not(target_pointer_width = "64"))]
                let boottime: i64 = t.into();
                now - boottime
            }
            None => -1,
        }
    })
}

#[cfg(windows)]
pub fn get_uptime(_boot_time: Option<time_t>) -> i64 {
    unsafe { GetTickCount() as i64 }
}

/// Returns the formatted uptime string, e.g. "1 day, 3:45"
#[inline]
pub fn get_formated_uptime(boot_time: Option<time_t>) -> UResult<String> {
    let up_secs = get_uptime(boot_time);

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

#[inline]
fn format_nusers(nusers: usize) -> String {
    match nusers {
        0 => "0 user".to_string(),
        1 => "1 user".to_string(),
        _ => format!("{} users", nusers),
    }
}

#[inline]
fn get_formatted_nusers() -> String {
    format_nusers(get_nusers())
}

#[cfg(target_os = "windows")]
fn get_nusers() -> usize {
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

#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
// see: https://gitlab.com/procps-ng/procps/-/blob/4740a0efa79cade867cfc7b32955fe0f75bf5173/library/uptime.c#L63-L115
fn get_nusers() -> usize {
    use uucore::utmpx::Utmpx;

    #[cfg(target_os = "linux")]
    unsafe {
        use libsystemd_sys::daemon::sd_booted;
        use libsystemd_sys::login::{sd_get_sessions, sd_session_get_class};
        use std::ffi::{c_char, c_void, CStr};
        use std::ptr;
        use uucore::libc::free;
        // systemd
        if sd_booted() > 0 {
            let mut sessions_list: *mut *mut c_char = ptr::null_mut();
            let mut num_user = 0;
            let sessions = sd_get_sessions(&mut sessions_list); // rust-systemd does not implement this

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

            return num_user;
        }
    }

    // utmpx
    let mut num_user = 0;
    Utmpx::iter_all_records().for_each(|ut| {
        if ut.record_type() == 7 && !ut.user().is_empty() {
            num_user += 1;
        }
    });
    num_user
}

#[cfg(target_os = "openbsd")]
fn get_nusers() -> usize {
    process_utmp_from_file("/var/run/utmp")
}
