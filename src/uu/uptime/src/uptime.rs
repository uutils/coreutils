// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) getloadavg upsecs updays nusers loadavg boottime uphours upmins

use chrono::{Local, TimeZone, Utc};
use clap::{crate_version, Arg, ArgAction, Command};

use uucore::libc::time_t;
use uucore::{format_usage, help_about, help_usage};

use uucore::error::{UResult, USimpleError};

const ABOUT: &str = help_about!("uptime.md");
const USAGE: &str = help_usage!("uptime.md");
pub mod options {
    pub static SINCE: &str = "since";
}

#[cfg(unix)]
use uucore::libc::getloadavg;

#[cfg(windows)]
extern "C" {
    fn GetTickCount() -> uucore::libc::uint32_t;
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let (boot_time, user_count) = process_utmpx();
    let uptime = get_uptime(boot_time);
    if uptime < 0 {
        Err(USimpleError::new(1, "could not retrieve system uptime"))
    } else {
        if matches.get_flag(options::SINCE) {
            let initial_date = Local
                .timestamp_opt(Utc::now().timestamp() - uptime, 0)
                .unwrap();
            println!("{}", initial_date.format("%Y-%m-%d %H:%M:%S"));
            return Ok(());
        }

        print_time();
        let upsecs = uptime;
        print_uptime(upsecs);
        print_nusers(user_count);
        print_loadavg();

        Ok(())
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
fn process_utmpx() -> (Option<time_t>, usize) {
    use uucore::utmpx::*;

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

#[cfg(windows)]
fn process_utmpx() -> (Option<time_t>, usize) {
    (None, 0) // TODO: change 0 to number of users
}

fn print_nusers(nusers: usize) {
    match nusers.cmp(&1) {
        std::cmp::Ordering::Equal => print!("1 user,  "),
        std::cmp::Ordering::Greater => print!("{nusers} users,  "),
        _ => {}
    };
}

fn print_time() {
    let local_time = Local::now().time();

    print!(" {} ", local_time.format("%H:%M:%S"));
}

#[cfg(unix)]
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
            print!("up {updays:1} days, {uphours:2}:{upmins:02},  ");
        }
        _ => print!("up  {uphours:2}:{upmins:02}, "),
    };
}
