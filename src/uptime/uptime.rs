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

extern crate getopts;

#[macro_use]
extern crate uucore;
// import crate time from utmpx
use uucore::utmpx::*;
use uucore::libc::{time_t, c_double};
pub use uucore::libc;

use getopts::Options;
use std::fs::File;
use std::io::{Read, Write};
use std::mem::transmute;

static NAME: &'static str = "uptime";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(unix)]
use libc::getloadavg;

#[cfg(windows)]
extern {
    fn GetTickCount() -> libc::uint32_t;
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("v", "version", "output version information and exit");
    opts.optflag("h", "help", "display this help and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };
    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }
    if matches.opt_present("help") || !matches.free.is_empty() {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION]", NAME);
        println!("");
        println!("{}", opts.usage("Print the current time, the length of time the system has been up,\n\
                              the number of users on the system, and the average number of jobs\n\
                              in the run queue over the last 1, 5 and 15 minutes."));
        return 0;
    }

    print_time();
    let (boot_time, user_count) = process_utmpx();
    let upsecs = get_uptime(boot_time) / 100;
    print_uptime(upsecs);
    print_nusers(user_count);
    print_loadavg();

    0
}

fn print_loadavg() {
    let mut avg: [c_double; 3] = [0.0; 3];
    let loads: i32 = unsafe { transmute(getloadavg(avg.as_mut_ptr(), 3)) };

    if loads == -1 {
        print!("\n");
    }
    else {
        print!("load average: ");
        for n in 0..loads {
            print!("{:.2}{}", avg[n as usize], if n == loads - 1 { "\n" }
                                   else { ", " } );
        }
    }
}

#[cfg(unix)]
fn process_utmpx() -> (Option<time_t>, usize) {
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
            },
            _ => continue
        }
    }
    (boot_time, nusers)
}

#[cfg(windows)]
fn process_utmpx() -> (Option<time_t>, usize) {
    (None, 0) // TODO: change 0 to number of users
}

fn print_nusers(nusers: usize) {
    if nusers == 1 {
        print!("1 user, ");
    } else if nusers > 1 {
        print!("{} users, ", nusers);
    }
}

fn print_time() {
    let local_time = time::now();

    print!(" {:02}:{:02}:{:02} ", local_time.tm_hour,
           local_time.tm_min, local_time.tm_sec);
}

#[cfg(unix)]
fn get_uptime(boot_time: Option<time_t>) -> i64 {
    let mut proc_uptime = String::new();

    if let Some(n) =
        File::open("/proc/uptime").ok()
            .and_then(|mut f| f.read_to_string(&mut proc_uptime).ok())
            .and_then(|_| proc_uptime.split_whitespace().next())
            .and_then(|s| s.replace(".", "").parse().ok()) {
        n
    } else {
        match boot_time {
            Some(t) => {
                let now = time::get_time().sec;
                let boottime = t as i64;
                ((now - boottime) * 100)
            },
            _ => -1,
        }
    }
}

#[cfg(windows)]
fn get_uptime(boot_time: Option<time_t>) -> i64 {
    unsafe { GetTickCount() as i64 }
}

fn print_uptime(upsecs: i64) {
    let updays = upsecs / 86400;
    let uphours = (upsecs - (updays * 86400)) / 3600;
    let upmins = (upsecs - (updays * 86400) - (uphours * 3600)) / 60;
    if updays == 1 {
        print!("up {:1} day, {:2}:{:02}, ", updays, uphours, upmins);
    }
    else if updays > 1 {
        print!("up {:1} days, {:2}:{:02}, ", updays, uphours, upmins);
    }
    else {
        print!("up  {:2}:{:02}, ", uphours, upmins);
    }
}
