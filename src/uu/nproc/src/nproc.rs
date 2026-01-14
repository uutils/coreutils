// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) NPROCESSORS nprocs numstr sysconf

use clap::{Arg, ArgAction, Command};
use std::io::{Write, stdout};
use std::{env, thread};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;
use uucore::translate;

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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let ignore = match matches.get_one::<String>(OPT_IGNORE) {
        Some(numstr) => match numstr.trim().parse::<usize>() {
            Ok(num) => num,
            Err(e) => {
                return Err(USimpleError::new(
                    1,
                    translate!("nproc-error-invalid-number", "value" => numstr.quote(), "error" => e),
                ));
            }
        },
        None => 0,
    };

    let limit = match env::var("OMP_THREAD_LIMIT") {
        // Uses the OpenMP variable to limit the number of threads
        // If the parsing fails, returns the max size (so, no impact)
        // If OMP_THREAD_LIMIT=0, rejects the value
        Ok(threads) => match threads.parse() {
            Ok(0) | Err(_) => usize::MAX,
            Ok(n) => n,
        },
        // the variable 'OMP_THREAD_LIMIT' doesn't exist
        // fallback to the max
        Err(_) => usize::MAX,
    };

    let mut cores = if matches.get_flag(OPT_ALL) {
        num_cpus_all()
    } else {
        // OMP_NUM_THREADS doesn't have an impact on --all
        match env::var("OMP_NUM_THREADS") {
            // Uses the OpenMP variable to force the number of threads
            // If the parsing fails, returns the number of CPU
            Ok(threads) => {
                // In some cases, OMP_NUM_THREADS can be "x,y,z"
                // In this case, only take the first one (like GNU)
                // If OMP_NUM_THREADS=0, rejects the value
                match threads.split_terminator(',').next() {
                    None => available_parallelism(),
                    Some(s) => match s.parse() {
                        Ok(0) | Err(_) => available_parallelism(),
                        Ok(n) => n,
                    },
                }
            }
            // the variable 'OMP_NUM_THREADS' doesn't exist
            // fallback to the regular CPU detection
            Err(_) => available_parallelism(),
        }
    };

    cores = std::cmp::min(limit, cores);
    if cores <= ignore {
        cores = 1;
    } else {
        cores -= ignore;
    }
    //discard error about stdout flush
    stdout()
        .lock()
        .write_all(format!("{cores}\n").as_bytes())
        .map_err(|e| USimpleError::new(1, e.to_string()))?;
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("nproc-about"))
        .override_usage(format_usage(&translate!("nproc-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_ALL)
                .long(OPT_ALL)
                .help(translate!("nproc-help-all"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_IGNORE)
                .long(OPT_IGNORE)
                .value_name("N")
                .help(translate!("nproc-help-ignore")),
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
        available_parallelism()
    } else if nprocs > 0 {
        nprocs as usize
    } else {
        1
    }
}

// Other platforms (e.g., windows), available_parallelism() directly.
#[cfg(not(any(
    target_os = "linux",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd"
)))]
fn num_cpus_all() -> usize {
    available_parallelism()
}

/// In some cases, [`thread::available_parallelism`]() may return an Err
/// In this case, we will return 1 (like GNU)
fn available_parallelism() -> usize {
    match thread::available_parallelism() {
        Ok(n) => n.get(),
        Err(_) => 1,
    }
}
