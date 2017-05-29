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

#[cfg(unix)]
extern crate libc;

#[macro_use]
extern crate uucore;

use std::io::Write;
use std::env;

#[cfg(target_os = "linux")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 83;
#[cfg(target_os = "macos")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = libc::_SC_NPROCESSORS_CONF;
#[cfg(target_os = "freebsd")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 57;
#[cfg(target_os = "netbsd")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 1001;


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

    let mut cores = if matches.opt_present("all") {
        num_cpus_all()
    } else {
        num_cpus::get()
    };

    if cores <= ignore {
        cores = 1;
    } else {
        cores -= ignore;
    }
    println!("{}", cores);
    0
}

#[cfg(any(target_os = "linux",
          target_os = "macos",
          target_os = "freebsd",
          target_os = "netbsd"))]
fn num_cpus_all() -> usize {
    let nprocs = unsafe { libc::sysconf(_SC_NPROCESSORS_CONF) };
    if nprocs == 1 {
        // In some situation, /proc and /sys are not mounted, and sysconf returns 1.
        // However, we want to guarantee that `nproc --all` >= `nproc`.
        num_cpus::get()
    } else {
        if nprocs > 0 { nprocs as usize } else { 1 }
    }
}

// Other platform(e.g., windows), num_cpus::get() directly.
#[cfg(not(any(target_os = "linux",
              target_os = "macos",
              target_os = "freebsd",
              target_os = "netbsd")))]
fn num_cpus_all() -> usize {
    num_cpus::get()
}
