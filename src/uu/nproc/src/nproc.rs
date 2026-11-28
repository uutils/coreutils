// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore NPROCESSORS SCHED ONLN getaffinity getcpu getscheduler sched sysconf

use clap::{Arg, ArgAction, Command};
use std::env;
use std::io::{Write, stdout};
use uucore::{
    error::{UResult, USimpleError, strip_errno},
    format_usage, translate,
};

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "platform/linux.rs"]
mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
use linux::available_parallelism;

static OPT_ALL: &str = "all";
static OPT_IGNORE: &str = "ignore";

#[uucore::main(no_signals)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    #[allow(clippy::unwrap_used, reason = "clap provides 0 by default")]
    let ignore = *matches.get_one::<usize>(OPT_IGNORE).unwrap();
    // Uses the OpenMP variable to limit the number of threads
    // Non OMP_THREAD_LIMIT>0 cases are rejected
    let limit = env::var("OMP_THREAD_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(usize::MAX);

    let mut cores = if matches.get_flag(OPT_ALL) {
        num_cpus_all()
    } else {
        // OMP_NUM_THREADS doesn't have an impact on --all
        // Uses the OpenMP variable to force the number of threads
        // If the parsing fails, returns the number of CPU
        // Non OMP_NUM_THREADS>0 cases are rejected
        omp_num_threads().unwrap_or_else(available_parallelism)
    };

    cores = cores.saturating_sub(ignore).clamp(1, limit);
    //discard error about stdout flush
    stdout()
        .write_all(format!("{cores}\n").as_bytes())
        .map_err(|e| USimpleError::new(1, strip_errno(&e)))
}

fn omp_num_threads() -> Option<usize> {
    let threads = env::var("OMP_NUM_THREADS").ok()?;
    let s = threads.split_terminator(',').next()?;
    // In some cases, OMP_NUM_THREADS can be "x,y,z"
    // In this case, only take the first one (like GNU)
    match s.trim().parse::<usize>() {
        Ok(n @ 1..) => Some(n),
        Err(e) if *e.kind() == std::num::IntErrorKind::PosOverflow => Some(usize::MAX),
        _ => None,
    }
}

pub fn uu_app() -> Command {
    Command::new("nproc")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("nproc"))
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
                .default_value("0")
                .value_parser(|s: &str| -> Result<usize, String> {
                    match s.trim().parse::<usize>() {
                        Ok(n) => Ok(n),
                        Err(e) if *e.kind() == std::num::IntErrorKind::PosOverflow => {
                            Ok(usize::MAX)
                        }
                        Err(e) => Err(e.to_string()),
                    }
                })
                .help(translate!("nproc-help-ignore")),
        )
}

fn num_cpus_all() -> usize {
    // sysconf returns (hardcoded?) 2 if /proc and /sys are masked, and sched_getaffinity syscall was blocked by strace.
    // So fallback to available_parallelism at here is not useful
    #[cfg(unix)]
    return unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize;
    // not sure what we can do for non-unix...
    #[cfg(not(unix))]
    available_parallelism()
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn available_parallelism() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}
