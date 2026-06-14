// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) NPROCESSORS SCHED getscheduler nprocs sched sysconf

use clap::{Arg, ArgAction, Command};
use std::io::{Write, stdout};
use std::{env, thread};
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
                    s.trim().parse::<usize>().map_err(|e| e.to_string())
                })
                .help(translate!("nproc-help-ignore")),
        )
}

#[cfg(unix)]
fn num_cpus_all() -> usize {
    // In some situation, /proc and /sys are not mounted, and sysconf returns 1.
    // However, we want to guarantee that `nproc --all` >= `nproc`.
    unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) }
        .try_into()
        .ok()
        .filter(|&n: &isize| n > 1)
        .map_or_else(available_parallelism, |n| n as usize)
}

// Other platforms (e.g., windows), available_parallelism() directly.
#[cfg(not(unix))]
fn num_cpus_all() -> usize {
    available_parallelism()
}

/// In some cases, [`thread::available_parallelism`]() may return an Err
/// In this case, we will return 1 (like GNU)
fn available_parallelism() -> usize {
    // ignore quota under some schedulers
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let scheduler = unsafe { libc::sched_getscheduler(0) };

        if matches!(
            scheduler,
            libc::SCHED_FIFO | libc::SCHED_RR | libc::SCHED_DEADLINE
        ) {
            return num_cpus_all();
        }
    }
    thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}
