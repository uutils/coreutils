#![crate_name = "uu_uptime"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 * (c) Jian Zeng <anonymousknight86@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: cat (GNU coreutils) 8.13 */

extern crate chrono;
extern crate clap;
extern crate time;

use chrono::{Local, TimeZone, Utc};
use clap::{App, Arg};

#[macro_use]
extern crate uucore;
// import crate time from utmpx
pub use uucore::libc;
use uucore::libc::time_t;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Display the current time, the length of time the system has been up,\n\
the number of users on the system, and the average number of jobs\n\
in the run queue over the last 1, 5 and 15 minutes.";
static OPT_SINCE: &str = "SINCE";

#[cfg(unix)]
use libc::getloadavg;

#[cfg(windows)]
extern "C" {
    fn GetTickCount() -> libc::uint32_t;
}

fn get_usage() -> String {
    format!("{0} [OPTION]...", executable!())
}

pub fn uumain(args: Vec<String>) -> i32 {
    let usage = get_usage();
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_SINCE)
                .short("s")
                .long("since")
                .help("system up since"),
        )
        .get_matches_from(&args);

    let (boot_time, user_count) = process_utmpx();
    let uptime = get_uptime(boot_time);
    if uptime < 0 {
        show_error!("could not retrieve system uptime");

        1
    } else {
        if matches.is_present(OPT_SINCE) {
            let initial_date = Local.timestamp(Utc::now().timestamp() - uptime, 0);
            println!("{}", initial_date.format("%Y-%m-%d %H:%M:%S"));
            return 0;
        }

        print_time();
        let upsecs = uptime;
        print_uptime(upsecs);
        print_nusers(user_count);
        print_loadavg();

        0
    }
}

#[cfg(unix)]
fn print_loadavg() {
    use libc::c_double;

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
    let local_time = time::now();

    print!(
        " {:02}:{:02}:{:02} ",
        local_time.tm_hour, local_time.tm_min, local_time.tm_sec
    );
}

#[cfg(unix)]
fn get_uptime(boot_time: Option<time_t>) -> i64 {
    use std::fs::File;
    use std::io::Read;

    let mut proc_uptime = String::new();

    if let Some(n) = File::open("/proc/uptime")
        .ok()
        .and_then(|mut f| f.read_to_string(&mut proc_uptime).ok())
        .and_then(|_| proc_uptime.split_whitespace().next())
        .and_then(|s| s.split('.').next().unwrap_or("0").parse().ok())
    {
        n
    } else {
        match boot_time {
            Some(t) => {
                let now = time::get_time().sec;
                let boottime = t as i64;
                now - boottime
            }
            _ => -1,
        }
    }
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
            print!("up {:1} days, {:2}:{:02},  ", updays, uphours, upmins)
        }
        _ => print!("up  {:2}:{:02}, ", uphours, upmins),
    };
}
