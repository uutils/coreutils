//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) NPROCESSORS nprocs numstr threadstr sysconf

extern crate clap;
extern crate getopts;
extern crate num_cpus;

#[cfg(unix)]
extern crate libc;

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::env;

#[cfg(target_os = "linux")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 83;
#[cfg(target_os = "macos")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = libc::_SC_NPROCESSORS_CONF;
#[cfg(target_os = "freebsd")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 57;
#[cfg(target_os = "netbsd")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 1001;

static OPT_ALL: &str = "all";
static OPT_IGNORE: &str = "ignore";

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Print the number of cores available to the current process.";

fn get_usage() -> String {
    format!("{0} [OPTIONS]...", executable!())
}

pub fn uumain(args: Vec<String>) -> i32 {
    let usage = get_usage();
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_ALL)
                .short("")
                .long("all")
                .help("print the number of cores available to the system"),
        )
        .arg(
            Arg::with_name(OPT_IGNORE)
                .short("")
                .long("ignore")
                .takes_value(true)
                .help("ignore up to N cores"),
        )
        .get_matches_from(&args);

    let mut ignore = match matches.value_of(OPT_IGNORE) {
        Some(numstr) => match numstr.parse() {
            Ok(num) => num,
            Err(e) => {
                show_error!("\"{}\" is not a valid number: {}", numstr, e);
                return 1;
            }
        },
        None => 0,
    };

    if !matches.is_present(OPT_ALL) {
        // OMP_NUM_THREADS doesn't have an impact on --all
        ignore += match env::var("OMP_NUM_THREADS") {
            Ok(threadstr) => match threadstr.parse() {
                Ok(num) => num,
                Err(_) => 0,
            },
            Err(_) => 0,
        };
    }

    let mut cores = if matches.is_present(OPT_ALL) {
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

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn num_cpus_all() -> usize {
    let nprocs = unsafe { libc::sysconf(_SC_NPROCESSORS_CONF) };
    if nprocs == 1 {
        // In some situation, /proc and /sys are not mounted, and sysconf returns 1.
        // However, we want to guarantee that `nproc --all` >= `nproc`.
        num_cpus::get()
    } else if nprocs > 0 {
        nprocs as usize
    } else {
        1
    }
}

// Other platforms (e.g., windows), num_cpus::get() directly.
#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd"
)))]
fn num_cpus_all() -> usize {
    num_cpus::get()
}
