#[crate_id(name="uptime", vers="1.0.0", author="José Neder")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: cat (GNU coreutils) 8.13 */

#[allow(non_camel_case_types)];
#[feature(macro_rules, globs)];

extern crate getopts;

use std::os;
use std::cast::transmute;
use std::io::{print,File};
use std::libc::{time_t,c_double,c_int,size_t,c_char};
use std::ptr::null;
use std::from_str::from_str;
use c_types::c_tm;
use utmpx::*;

#[path = "../common/util.rs"] mod util;

#[path = "../common/c_types.rs"] mod c_types;

#[path = "../common/utmpx.rs"] mod utmpx;

static NAME: &'static str = "uptime";

extern {
    fn time(timep: *time_t) -> time_t;
    fn localtime(timep: *time_t) -> *c_tm;

    fn getloadavg(loadavg: *c_double, nelem: c_int) -> c_int;

    fn getutxent() -> *c_utmp;
    fn setutxent();
    fn endutxent();

    fn utmpxname(file: *c_char) -> c_int;
}

fn main() {
    let args = os::args();
    let program = args[0].clone();
    let opts = ~[
        getopts::optflag("v", "version", "output version information and exit"),
        getopts::optflag("h", "help", "display this help and exit"),
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f.to_err_msg())
    };
    if matches.opt_present("version") {
        println!("uptime 1.0.0");
        return;
    }
    if matches.opt_present("help") || matches.free.len() > 0 {
        println!("Usage:");
        println!("  {0:s} [OPTION]", program);
        println!("");
        print(getopts::usage("Print the current time, the length of time the system has been up,\n\
                              the number of users on the system, and the average number of jobs\n\
                              in the run queue over the last 1, 5 and 15 minutes.", opts));
        return;
    }

    print_time();
    print_uptime();
    print_nusers();
    print_loadavg();
}

fn print_loadavg() {
    let avg: [c_double, ..3] = [0.0, ..3];
    let loads: i32 = unsafe { transmute(getloadavg(avg.as_ptr(), 3)) };

    if loads == -1 {
        print!("\n");
    }
    else {
        print!("load average: ")
        for n in range(0, loads) {
            print!("{:.2f}{}", avg[n], if n == loads - 1 { "\n" }
                                   else { ", " } );
        }
    }
}

fn print_nusers() {
    DEFAULT_FILE.with_c_str(|filename| {
        unsafe {
            utmpxname(filename);
        }
    });

    let mut nusers = 0;

    unsafe {
        setutxent();

        loop {
            let line = getutxent();

            if line == null() {
                break;
            }

            if (*line).ut_type == USER_PROCESS {
                nusers += 1;
            }
        }

        endutxent();
    }

    if nusers == 1 {
        print!("1 user, ");
    } else if nusers > 1 {
        print!("{} users, ", nusers);
    }
}

fn print_time() {
    let local_time = unsafe { *localtime(&time(null())) };

    if local_time.tm_hour >= 0 && local_time.tm_min >= 0 &&
       local_time.tm_sec >= 0 {
        print!(" {:02d}:{:02d}:{:02d} ", local_time.tm_hour,
               local_time.tm_min, local_time.tm_sec);
    }
}

fn get_uptime() -> int {
    let proc_uptime = File::open(&Path::new("/proc/uptime"))
                            .read_to_str();

    let uptime_text = match proc_uptime {
        Ok(s) => s,
        _ => return -1
    };

    match uptime_text.words().next() {
        Some(s) => match from_str(s.replace(".","")) {
                    Some(n) => n,
                    None => -1
                   },
        None => -1
    }
}

fn print_uptime() {
    let uptime = get_uptime() / 100;
    let updays = uptime / 86400;
    let uphours = (uptime - (updays * 86400)) / 3600;
    let upmins = (uptime - (updays * 86400) - (uphours * 3600)) / 60;
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
