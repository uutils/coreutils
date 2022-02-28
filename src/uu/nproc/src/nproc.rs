//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) NPROCESSORS nprocs numstr threadstr sysconf

use clap::{crate_version, App, AppSettings, Arg};
use std::env;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;

#[cfg(target_os = "linux")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 83;
#[cfg(target_vendor = "apple")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = libc::_SC_NPROCESSORS_CONF;
#[cfg(target_os = "freebsd")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 57;
#[cfg(target_os = "netbsd")]
pub const _SC_NPROCESSORS_CONF: libc::c_int = 1001;

static OPT_ALL: &str = "all";
static OPT_IGNORE: &str = "ignore";

static ABOUT: &str = "Print the number of cores available to the current process.";
const USAGE: &str = "{} [OPTIONS]...";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let mut ignore = match matches.value_of(OPT_IGNORE) {
        Some(numstr) => match numstr.parse() {
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

    if !matches.is_present(OPT_ALL) {
        // OMP_NUM_THREADS doesn't have an impact on --all
        ignore += match env::var("OMP_NUM_THREADS") {
            Ok(threadstr) => threadstr.parse().unwrap_or(0),
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
    Ok(())
}

pub fn uu_app<'a>() -> App<'a> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .setting(AppSettings::InferLongArgs)
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
