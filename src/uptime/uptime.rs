#![crate_name = "uptime"]
#![feature(collections, core, io, libc, path, rustc_private, std_misc)]

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

extern crate getopts;
extern crate libc;
extern crate "time" as rtime;

use std::ffi::CString;
use std::mem::transmute;
use std::old_io::{print, File};
use std::ptr::null;
use libc::{time_t, c_double, c_int, c_char};
use utmpx::*;

#[path = "../common/util.rs"] #[macro_use] mod util;

#[path = "../common/c_types.rs"] mod c_types;

#[path = "../common/utmpx.rs"] mod utmpx;

static NAME: &'static str = "uptime";

#[cfg(unix)]
extern {
    fn getloadavg(loadavg: *mut c_double, nelem: c_int) -> c_int;

    fn getutxent() -> *const c_utmp;
    fn setutxent();
    fn endutxent();

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn utmpxname(file: *const c_char) -> c_int;
}

#[cfg(windows)]
extern {
    fn GetTickCount() -> libc::uint32_t;
}

#[cfg(target_os = "freebsd")]
unsafe extern fn utmpxname(_file: *const c_char) -> c_int {
    0
}

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();
    let opts = [
        getopts::optflag("v", "version", "output version information and exit"),
        getopts::optflag("h", "help", "display this help and exit"),
    ];
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };
    if matches.opt_present("version") {
        println!("uptime 1.0.0");
        return 0;
    }
    if matches.opt_present("help") || matches.free.len() > 0 {
        println!("Usage:");
        println!("  {0} [OPTION]", program);
        println!("");
        print(getopts::usage("Print the current time, the length of time the system has been up,\n\
                              the number of users on the system, and the average number of jobs\n\
                              in the run queue over the last 1, 5 and 15 minutes.", &opts).as_slice());
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
        for n in range(0, loads) {
            print!("{:.2}{}", avg[n as usize], if n == loads - 1 { "\n" }
                                   else { ", " } );
        }
    }
}

#[cfg(unix)]
fn process_utmpx() -> (Option<time_t>, usize) {
    unsafe {
        utmpxname(CString::from_slice(DEFAULT_FILE.as_bytes()).as_ptr());
    }

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
    let local_time = rtime::now();

    print!(" {:02}:{:02}:{:02} ", local_time.tm_hour,
           local_time.tm_min, local_time.tm_sec);
}

#[cfg(unix)]
fn get_uptime(boot_time: Option<time_t>) -> i64 {
    let proc_uptime = File::open(&Path::new("/proc/uptime"))
                            .read_to_string();

    let uptime_text = match proc_uptime {
        Ok(s) => s,
        _ => return match boot_time {
                Some(t) => {
                    let now = rtime::get_time().sec;
                    let time = t as i64;
                    ((now - time) * 100) as i64 // Return in ms
                },
                _ => -1
             }
    };

    match uptime_text.as_slice().words().next() {
        Some(s) => match s.replace(".", "").as_slice().parse() {
                    Ok(n) => n,
                    Err(_) => -1
                   },
        None => -1
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
        print!("up {:1} day, {:2}:{:02},  ", updays, uphours, upmins);
    }
    else if updays > 1 {
        print!("up {:1} days, {:2}:{:02},  ", updays, uphours, upmins);
    }
    else {
        print!("up  {:2}:{:02},  ", uphours, upmins);
    }
}
