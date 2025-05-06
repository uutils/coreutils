// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore getloadavg behaviour loadavg uptime upsecs updays upmins uphours boottime nusers utmpxname gettime clockid

use chrono::{Local, TimeZone, Utc};
#[cfg(unix)]
use std::ffi::OsString;
use std::io;
use thiserror::Error;
use uucore::error::{UError, UResult};
use uucore::libc::time_t;
use uucore::uptime::*;

use clap::{Arg, ArgAction, Command, ValueHint, builder::ValueParser};

use uucore::{format_usage, help_about, help_usage};

#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
use uucore::utmpx::*;

#[cfg(target_env = "musl")]
const ABOUT: &str = concat!(
    help_about!("uptime.md"),
    "\n\nWarning: When built with musl libc, the `uptime` utility may show '0 users' \n",
    "due to musl's stub implementation of utmpx functions. Boot time and load averages \n",
    "are still calculated using alternative mechanisms."
);

#[cfg(not(target_env = "musl"))]
const ABOUT: &str = help_about!("uptime.md");

const USAGE: &str = help_usage!("uptime.md");

pub mod options {
    pub static SINCE: &str = "since";
    pub static PATH: &str = "path";
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
}

impl UError for UptimeError {
    fn code(&self) -> i32 {
        1
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    #[cfg(unix)]
    let file_path = matches.get_one::<OsString>(options::PATH);
    #[cfg(windows)]
    let file_path = None;

    if matches.get_flag(options::SINCE) {
        uptime_since()
    } else if let Some(path) = file_path {
        uptime_with_file(path)
    } else {
        default_uptime()
    }
}

pub fn uu_app() -> Command {
    let cmd = Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SINCE)
                .short('s')
                .long(options::SINCE)
                .help("system up since")
                .action(ArgAction::SetTrue),
        );
    #[cfg(unix)]
    cmd.arg(
        Arg::new(options::PATH)
            .help("file to search boot time from")
            .action(ArgAction::Set)
            .num_args(0..=1)
            .value_parser(ValueParser::os_string())
            .value_hint(ValueHint::AnyPath),
    )
}

#[cfg(unix)]
fn uptime_with_file(file_path: &OsString) -> UResult<()> {
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
            print_nusers(Some(0));
            print_loadavg();
            set_exit_code(1);
            return Ok(());
        }
    }

    if non_fatal_error {
        print_time();
        print!("up ???? days ??:??,");
        print_nusers(Some(0));
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
        let upsecs = get_uptime(None);
        if upsecs >= 0 {
            print_uptime(Some(upsecs))?;
        } else {
            show_error!("couldn't get boot time");
            set_exit_code(1);

            print!("up ???? days ??:??,");
        }
        user_count = get_nusers(file_path.to_str().expect("invalid utmp path file"));
    }

    print_nusers(Some(user_count));
    print_loadavg();

    Ok(())
}

fn uptime_since() -> UResult<()> {
    #[cfg(unix)]
    #[cfg(not(target_os = "openbsd"))]
    let (boot_time, _) = process_utmpx(None);

    #[cfg(target_os = "openbsd")]
    let uptime = get_uptime(None)?;
    #[cfg(unix)]
    #[cfg(not(target_os = "openbsd"))]
    let uptime = get_uptime(boot_time)?;
    #[cfg(target_os = "windows")]
    let uptime = get_uptime(None)?;

    let initial_date = Local
        .timestamp_opt(Utc::now().timestamp() - uptime, 0)
        .unwrap();
    println!("{}", initial_date.format("%Y-%m-%d %H:%M:%S"));

    Ok(())
}

/// Default uptime behaviour i.e. when no file argument is given.
fn default_uptime() -> UResult<()> {
    print_time();
    print_uptime(None)?;
    print_nusers(None);
    print_loadavg();

    Ok(())
}

#[inline]
fn print_loadavg() {
    match get_formatted_loadavg() {
        Err(_) => {}
        Ok(s) => println!("{s}"),
    }
}

#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
fn process_utmpx(file: Option<&OsString>) -> (Option<time_t>, usize) {
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

fn print_nusers(nusers: Option<usize>) {
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
}

fn print_time() {
    print!(" {}  ", get_formatted_time());
}

fn print_uptime(boot_time: Option<time_t>) -> UResult<()> {
    print!("up  {},  ", get_formatted_uptime(boot_time)?);
    Ok(())
}
