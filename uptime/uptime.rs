#![crate_id(name="uptime", vers="1.0.0", author="José Neder")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: cat (GNU coreutils) 8.13 */

#![allow(non_camel_case_types)]
#![feature(macro_rules, globs)]

extern crate getopts;
extern crate libc;

use std::mem::transmute;
use std::io::{print, File};
use std::ptr::{mut_null, null};
use std::from_str::from_str;
use libc::{time_t, c_double, c_int, c_char};
use c_types::c_tm;
use utmpx::*;

#[path = "../common/util.rs"] mod util;

#[path = "../common/c_types.rs"] mod c_types;

#[path = "../common/utmpx.rs"] mod utmpx;

static NAME: &'static str = "uptime";

extern {
    fn time(timep: *mut time_t) -> time_t;
    fn localtime(timep: *const time_t) -> *const c_tm;

    fn getloadavg(loadavg: *mut c_double, nelem: c_int) -> c_int;

    fn getutxent() -> *const c_utmp;
    fn setutxent();
    fn endutxent();

    fn utmpxname(file: *const c_char) -> c_int;
}

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0).clone();
    let opts = [
        getopts::optflag("v", "version", "output version information and exit"),
        getopts::optflag("h", "help", "display this help and exit"),
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };
    if matches.opt_present("version") {
        println!("uptime 1.0.0");
        return 0;
    }
    if matches.opt_present("help") || matches.free.len() > 0 {
        println!("Usage:");
        println!("  {0:s} [OPTION]", program);
        println!("");
        print(getopts::usage("Print the current time, the length of time the system has been up,\n\
                              the number of users on the system, and the average number of jobs\n\
                              in the run queue over the last 1, 5 and 15 minutes.", opts).as_slice());
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
    let mut avg: [c_double, ..3] = [0.0, ..3];
    let loads: i32 = unsafe { transmute(getloadavg(avg.as_mut_ptr(), 3)) };

    if loads == -1 {
        print!("\n");
    }
    else {
        print!("load average: ")
        for n in range(0, loads) {
            print!("{:.2f}{}", avg[n as uint], if n == loads - 1 { "\n" }
                                   else { ", " } );
        }
    }
}

fn process_utmpx() -> (Option<time_t>, uint) {
    DEFAULT_FILE.with_c_str(|filename| {
        unsafe {
            utmpxname(filename);
        }
    });

    let mut nusers = 0;
    let mut boot_time = None;

    unsafe {
        setutxent();

        loop {
            let line = getutxent();

            if line == null() {
                break;
            }

            match (*line).ut_type {
                USER_PROCESS => nusers += 1,
                BOOT_TIME => {
                    let t = (*line).ut_tv;
                    if t.tv_sec > 0 {
                        boot_time = Some(t.tv_sec);
                    }
                },
                _ => continue
            }
        }

        endutxent();
    }

    (boot_time, nusers)
}

fn print_nusers(nusers: uint) {
    if nusers == 1 {
        print!("1 user, ");
    } else if nusers > 1 {
        print!("{} users, ", nusers);
    }
}

fn print_time() {
    let local_time = unsafe { *localtime(&time(mut_null())) };

    if local_time.tm_hour >= 0 && local_time.tm_min >= 0 &&
       local_time.tm_sec >= 0 {
        print!(" {:02d}:{:02d}:{:02d} ", local_time.tm_hour,
               local_time.tm_min, local_time.tm_sec);
    }
}

fn get_uptime(boot_time: Option<time_t>) -> i64 {
    let proc_uptime = File::open(&Path::new("/proc/uptime"))
                            .read_to_str();

    let uptime_text = match proc_uptime {
        Ok(s) => s,
        _ => return match boot_time {
                Some(t) => {
                    let now = unsafe { time(mut_null()) };
                    ((now - t) * 100) as i64 // Return in ms
                },
                _ => -1
             }
    };

    match uptime_text.as_slice().words().next() {
        Some(s) => match from_str(s.replace(".","").as_slice()) {
                    Some(n) => n,
                    None => -1
                   },
        None => -1
    }
}

fn print_uptime(upsecs: i64) {
    let updays = upsecs / 86400;
    let uphours = (upsecs - (updays * 86400)) / 3600;
    let upmins = (upsecs - (updays * 86400) - (uphours * 3600)) / 60;
    if updays == 1 { 
        print!("up {:1d} day, {:2d}:{:02d},  ", updays, uphours, upmins);
    }
    else if updays > 1 {
        print!("up {:1d} days, {:2d}:{:02d},  ", updays, uphours, upmins);
    }
    else {
        print!("up  {:2d}:{:02d},  ", uphours, upmins);
    }
}
