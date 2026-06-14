// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore NPROCESSORS SCHED getaffinity getcpu getscheduler sched sysconf

use clap::{Arg, ArgAction, Command};
use std::env;
use std::io::{Write, stdout};
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;
use uucore::translate;

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
    } else if let Ok(threads) = env::var("OMP_NUM_THREADS") {
        // OMP_NUM_THREADS doesn't have an impact on --all
        // Uses the OpenMP variable to force the number of threads
        // If the parsing fails, returns the number of CPU
        // In some cases, OMP_NUM_THREADS can be "x,y,z"
        // In this case, only take the first one (like GNU)
        // If OMP_NUM_THREADS=0, rejects the value
        match threads.split_terminator(',').next() {
            None => available_parallelism(),
            Some(s) => match s.trim().parse::<usize>() {
                Ok(n @ 1..) => n,
                Err(e) if *e.kind() == std::num::IntErrorKind::PosOverflow => usize::MAX,
                _ => available_parallelism(),
            },
        }
    } else {
        // the variable 'OMP_NUM_THREADS' doesn't exist
        // fallback to the regular CPU detection
        available_parallelism()
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
    // sysconf returns 2 if /proc and /sys are masked, and sched_getaffinity syscall was blocked by strace
    // when SMT is enabled. So fallback to available_parallelism at here is not useful
    #[cfg(unix)]
    return unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) }.max(1) as usize;
    // not sure what we can do for non-unix...
    #[cfg(not(unix))]
    available_parallelism()
}

// We cannot use std::thread::available_parallelism to mimic GNU's rounding...
#[cfg(any(target_os = "linux", target_os = "android"))]
fn cgroups2_quota() -> Option<usize> {
    use std::fs::read_to_string;
    let cgroups = read_to_string("/proc/self/cgroup").ok()?;
    let path = cgroups.lines().next()?.split(':').nth(2)?;
    let pair = read_to_string(format!("/sys/fs/cgroup{path}/cpu.max")).ok()?;
    let mut pair = pair.split_whitespace();
    // map the string "max" to None as we unwrap_or(usize::MAX) later
    let quota = pair.next()?.parse::<usize>().ok()?;
    let period = pair.next()?.parse::<usize>().ok()?;
    debug_assert!(period > 0, "kernel should validate it");
    // mimic GNU's rounding
    Some(quota.saturating_add(period / 2) / period)
}

fn available_parallelism() -> usize {
    // return all cores if sched_getaffinity syscall failed as same as GNU
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let affinity = rustix::thread::sched_getaffinity(None)
        .map_or_else(|_| num_cpus_all(), |s| s.count() as usize);
    // ignore quota under some schedulers
    #[cfg(any(target_os = "linux", target_os = "android"))]
    match unsafe { libc::sched_getscheduler(0) } {
        libc::SCHED_FIFO | libc::SCHED_RR | libc::SCHED_DEADLINE => affinity,
        // GNU has no quota if /proc is masked
        _ => affinity.min(cgroups2_quota().unwrap_or(usize::MAX)),
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}
