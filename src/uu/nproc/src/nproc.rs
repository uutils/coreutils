//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) NPROCESSORS nprocs numstr threadstr sysconf

use clap::{crate_version, Arg, Command};
use std::env;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 83;
#[cfg(target_vendor = "apple")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = libc::_SC_NPROCESSORS_CONF;
#[cfg(target_os = "freebsd")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 57;
#[cfg(target_os = "netbsd")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 1001;

static OPT_ALL: &str = "all";
static OPT_IGNORE: &str = "ignore";

static ABOUT: &str = r#"Print the number of cores available to the current process.
If the OMP_NUM_THREADS or OMP_THREAD_LIMIT environment variables are set, then
they will determine the minimum and maximum returned value respectively."#;
const USAGE: &str = "{} [OPTIONS]...";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let ignore = match matches.value_of(OPT_IGNORE) {
        Some(numstr) => match numstr.trim().parse() {
            Ok(num) => num,
            Err(e) => {
                return Err(USimpleError::new(
                    1,
                    format!("{} is not a valid number: {}", numstr.quote(), e),
                ));
            }
        },
        None => 0,
    };

    let limit = match env::var("OMP_THREAD_LIMIT") {
        // Uses the OpenMP variable to limit the number of threads
        // If the parsing fails, returns the max size (so, no impact)
        // If OMP_THREAD_LIMIT=0, rejects the value
        Ok(threadstr) => match threadstr.parse() {
            Ok(0) | Err(_) => usize::MAX,
            Ok(n) => n,
        },
        // the variable 'OMP_THREAD_LIMIT' doesn't exist
        // fallback to the max
        Err(_) => usize::MAX,
    };

    let mut cores = if matches.is_present(OPT_ALL) {
        num_cpus_all()
    } else {
        // OMP_NUM_THREADS doesn't have an impact on --all
        match env::var("OMP_NUM_THREADS") {
            // Uses the OpenMP variable to force the number of threads
            // If the parsing fails, returns the number of CPU
            Ok(threadstr) => {
                // In some cases, OMP_NUM_THREADS can be "x,y,z"
                // In this case, only take the first one (like GNU)
                // If OMP_NUM_THREADS=0, rejects the value
                let thread: Vec<&str> = threadstr.split_terminator(',').collect();
                match &thread[..] {
                    [] => num_cpus::get(),
                    [s, ..] => match s.parse() {
                        Ok(0) | Err(_) => num_cpus::get(),
                        Ok(n) => n,
                    },
                }
            }
            // the variable 'OMP_NUM_THREADS' doesn't exist
            // fallback to the regular CPU detection
            Err(_) => num_cpus::get(),
        }
    };

    cores = std::cmp::min(limit, cores);
    if cores <= ignore {
        cores = 1;
    } else {
        cores -= ignore;
    }
    println!("{}", cores);
    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_ALL)
                .long(OPT_ALL)
                .help("print the number of cores available to the system"),
        )
        .arg(
            Arg::new(OPT_IGNORE)
                .long(OPT_IGNORE)
                .takes_value(true)
                .help("ignore up to N cores"),
        )
}

#[cfg(any(
    target_os = "linux",
    target_vendor = "apple",
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
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd"
)))]
fn num_cpus_all() -> usize {
    num_cpus::get()
}
