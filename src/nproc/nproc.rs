#![crate_name = "uu_nproc"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate num_cpus;
extern crate libc;

#[macro_use]
extern crate uucore;

use std::io::Write;
use std::env;

static NAME: &'static str = "nproc";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("", "all", "print the number of cores available to the system");
    opts.optopt("", "ignore", "ignore up to N cores", "N");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(err) => {
            show_error!("{}", err);
            return 1;
        }
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTIONS]...

Print the number of cores available to the current process.", NAME, VERSION);

        print!("{}", opts.usage(&msg));
        return 0;
    }

    let mut ignore = match matches.opt_str("ignore") {
        Some(numstr) => match numstr.parse() {
            Ok(num) => num,
            Err(e) => {
                show_error!("\"{}\" is not a valid number: {}", numstr, e);
                return 1;
            }
        },
        None => 0
    };

    if !matches.opt_present("all") {
        ignore += match env::var("OMP_NUM_THREADS") {
            Ok(threadstr) => match threadstr.parse() {
                Ok(num) => num,
                Err(_)=> 0
            },
            Err(_) => 0
        };
    }

    let mut cores = get_num_cpus();
    if cores <= ignore {
        cores = 1;
    } else {
        cores -= ignore;
    }
    println!("{}", cores);
    0
}

#[cfg(target_os = "linux")]
fn popcnt(n: u64) -> u64 {
    let mut c: u64 = (n & 0x5555555555555555) + ((n >> 1) & 0x5555555555555555);
    c = (c & 0x3333333333333333) + ((c >> 2) & 0x3333333333333333);
    c = (c & 0x0f0f0f0f0f0f0f0f) + ((c >> 4) & 0x0f0f0f0f0f0f0f0f);
    c = (c & 0x00ff00ff00ff00ff) + ((c >> 8) & 0x00ff00ff00ff00ff);
    c = (c & 0x0000ffff0000ffff) + ((c >> 16) & 0x0000ffff0000ffff);
    c = (c & 0x00000000ffffffff) + ((c >> 32) & 0x00000000ffffffff);
    c
}

#[cfg(target_os = "linux")]
fn get_num_cpus() -> usize {
    let mut set:  libc::cpu_set_t = unsafe { std::mem::zeroed() };
    if unsafe { libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut set) } == 0 {
        let ptr = unsafe { std::mem::transmute::<&libc::cpu_set_t, &u64>(&set) };
        popcnt(*ptr) as usize
    } else {
        num_cpus::get()
    }
}

#[cfg(not(target_os = "linux"))]
fn get_num_cpus() -> usize {
    num_cpus::get()
}
