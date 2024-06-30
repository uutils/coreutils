// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore getloadavg behaviour loadavg uptime upsecs updays upmins uphours boottime nusers utmpxname gettime clockid

use chrono::{Local, TimeZone, Utc};
use clap::ArgMatches;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::fs::FileTypeExt;
use thiserror::Error;
use uucore::error::set_exit_code;
use uucore::error::UError;
use uucore::show_error;

#[cfg(not(target_os = "openbsd"))]
use uucore::libc::time_t;

use uucore::error::{UResult, USimpleError};

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command, ValueHint};

use uucore::{format_usage, help_about, help_usage};

#[cfg(target_os = "openbsd")]
use utmp_classic::{parse_from_path, UtmpEntry};
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
    fn GetTickCount() -> uucore::libc::uint32_t;
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
fn uptime_with_file(file_path: &OsString) -> UResult<()> {
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
            print_nusers(0);
            print_loadavg();
            set_exit_code(1);
            return Ok(());
        }
    }

    if non_fatal_error {
        print_time();
        print!("up ???? days ??:??,");
        print_nusers(0);
        print_loadavg();
        return Ok(());
    }

    print_time();
    let user_count;

    #[cfg(not(target_os = "openbsd"))]
    {
        let (boot_time, count) = process_utmpx_from_file(file_path);
        if let Some(time) = boot_time {
            let upsecs = get_uptime_from_boot_time(time);
            print_uptime(upsecs);
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

        let upsecs = get_uptime();
        if upsecs < 0 {
            show_error!("couldn't get boot time");
            set_exit_code(1);

            print!("up ???? days ??:??,");
        } else {
            print_uptime(upsecs);
        }
    }

    print_nusers(user_count);
    print_loadavg();

    Ok(())
}

/// Default uptime behaviour i.e. when no file argument is given.
fn default_uptime(matches: &ArgMatches) -> UResult<()> {
    #[cfg(target_os = "openbsd")]
    let user_count = process_utmp_from_file("/var/run/utmp");
    #[cfg(not(target_os = "openbsd"))]
    let (boot_time, user_count) = process_utmpx();

    #[cfg(target_os = "openbsd")]
    let uptime = get_uptime();
    #[cfg(not(target_os = "openbsd"))]
    let uptime = get_uptime(boot_time);

    if matches.get_flag(options::SINCE) {
        let initial_date = Local
            .timestamp_opt(Utc::now().timestamp() - uptime, 0)
            .unwrap();
        println!("{}", initial_date.format("%Y-%m-%d %H:%M:%S"));
        return Ok(());
    }

    if uptime < 0 {
        return Err(USimpleError::new(1, "could not retrieve system uptime"));
    }

    print_time();
    print_uptime(uptime);
    print_nusers(user_count);
    print_loadavg();

    Ok(())
}

#[cfg(unix)]
fn print_loadavg() {
    use uucore::libc::c_double;

    let mut avg: [c_double; 3] = [0.0; 3];
    let loads: i32 = unsafe { getloadavg(avg.as_mut_ptr(), 3) };

    if loads == -1 {
        println!();
    } else {
        print!("load average: ");
        for n in 0..loads {
            print!(
                "{:.2}{}",
                avg[n as usize],
                if n == loads - 1 { "\n" } else { ", " }
            );
        }
    }
}

#[cfg(windows)]
fn print_loadavg() {
    // XXX: currently this is a noop as Windows does not seem to have anything comparable to
    //      getloadavg()
}

#[cfg(unix)]
#[cfg(target_os = "openbsd")]
fn process_utmp_from_file(file: &str) -> usize {
    let mut nusers = 0;

    let entries = parse_from_path(file).unwrap_or_default();
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
fn process_utmpx() -> (Option<time_t>, usize) {
    let mut nusers = 0;
    let mut boot_time = None;

    for line in Utmpx::iter_all_records() {
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

#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
fn process_utmpx_from_file(file: &OsString) -> (Option<time_t>, usize) {
    let mut nusers = 0;
    let mut boot_time = None;

    for line in Utmpx::iter_all_records_from(file) {
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

fn print_nusers(nusers: usize) {
    match nusers.cmp(&1) {
        std::cmp::Ordering::Less => print!("  0 users,  "),
        std::cmp::Ordering::Equal => print!("1 user,  "),
        std::cmp::Ordering::Greater => print!("{nusers} users,  "),
    };
}

fn print_time() {
    let local_time = Local::now().time();

    print!(" {}  ", local_time.format("%H:%M:%S"));
}

#[cfg(not(target_os = "openbsd"))]
fn get_uptime_from_boot_time(boot_time: time_t) -> i64 {
    let now = Local::now().timestamp();
    #[cfg(target_pointer_width = "64")]
    let boottime: i64 = boot_time;
    #[cfg(not(target_pointer_width = "64"))]
    let boottime: i64 = boot_time.into();
    now - boottime
}

#[cfg(unix)]
#[cfg(target_os = "openbsd")]
fn get_uptime() -> i64 {
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
fn get_uptime(boot_time: Option<time_t>) -> i64 {
    use std::fs::File;
    use std::io::Read;

    let mut proc_uptime_s = String::new();

    let proc_uptime = File::open("/proc/uptime")
        .ok()
        .and_then(|mut f| f.read_to_string(&mut proc_uptime_s).ok())
        .and_then(|_| proc_uptime_s.split_whitespace().next())
        .and_then(|s| s.split('.').next().unwrap_or("0").parse().ok());

    proc_uptime.unwrap_or_else(|| match boot_time {
        Some(t) => {
            let now = Local::now().timestamp();
            #[cfg(target_pointer_width = "64")]
            let boottime: i64 = t;
            #[cfg(not(target_pointer_width = "64"))]
            let boottime: i64 = t.into();
            now - boottime
        }
        None => -1,
    })
}

#[cfg(windows)]
fn get_uptime(_boot_time: Option<time_t>) -> i64 {
    unsafe { GetTickCount() as i64 }
}

fn print_uptime(upsecs: i64) {
    let updays = upsecs / 86400;
    let uphours = (upsecs - (updays * 86400)) / 3600;
    let upmins = (upsecs - (updays * 86400) - (uphours * 3600)) / 60;
    match updays.cmp(&1) {
        std::cmp::Ordering::Equal => print!("up {updays:1} day, {uphours:2}:{upmins:02},  "),
        std::cmp::Ordering::Greater => {
            print!("up {updays:1} days {uphours:2}:{upmins:02},  ");
        }
        _ => print!("up  {uphours:2}:{upmins:02},  "),
    };
}
