//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  * (c) Jian Zeng <anonymousknight86@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) getloadavg upsecs updays nusers loadavg boottime uphours upmins

use chrono::{Local, TimeZone, Utc};
use clap::{crate_version, Arg, Command};

use uucore::format_usage;
// import crate time from utmpx
pub use uucore::libc;
use uucore::libc::time_t;

use uucore::error::{UResult, USimpleError};

static ABOUT: &str = "Display the current time, the length of time the system has been up,\n\
                      the number of users on the system, and the average number of jobs\n\
                      in the run queue over the last 1, 5 and 15 minutes.";
const USAGE: &str = "{} [OPTION]...";
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
    let matches = uu_app().get_matches_from(args);

    let (boot_time, user_count) = process_utmpx();
    let uptime = get_uptime(boot_time);
    if uptime < 0 {
        Err(USimpleError::new(1, "could not retrieve system uptime"))
    } else {
        if matches.is_present(options::SINCE) {
            let initial_date = Local.timestamp(Utc::now().timestamp() - uptime, 0);
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

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SINCE)
                .short('s')
                .long(options::SINCE)
                .help("system up since"),
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
                let t = line.login_time().to_timespec();
                if t.sec > 0 {
                    boot_time = Some(t.sec as time_t);
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
        std::cmp::Ordering::Greater => print!("{} users,  ", nusers),
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
            let boottime = t as i64;
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
        std::cmp::Ordering::Equal => print!("up {:1} day, {:2}:{:02},  ", updays, uphours, upmins),
        std::cmp::Ordering::Greater => {
            print!("up {:1} days, {:2}:{:02},  ", updays, uphours, upmins);
        }
        _ => print!("up  {:2}:{:02}, ", uphours, upmins),
    };
}
