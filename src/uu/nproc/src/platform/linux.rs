// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore NPROCESSORS SCHED ONLN getaffinity getscheduler sched sysconf

use std::fs::read_to_string;

pub(crate) fn available_parallelism() -> usize {
    // return all online cores if sched_getaffinity syscall failed as same as GNU
    let affinity = rustix::thread::sched_getaffinity(None).map_or_else(
        |_| unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) } as usize,
        |s| s.count() as usize,
    );
    // ignore quota under some schedulers
    match unsafe { libc::sched_getscheduler(0) } {
        libc::SCHED_FIFO | libc::SCHED_RR | libc::SCHED_DEADLINE => affinity,
        // GNU has no quota if /proc is masked
        _ => affinity.min(cgroups2_quota().unwrap_or(usize::MAX)),
    }
}

// We cannot use std::thread::available_parallelism to mimic GNU's rounding...
fn cgroups2_quota() -> Option<usize> {
    let cgroups = read_to_string("/proc/self/cgroup").ok()?;
    // the path is everything after `0::`, so it may contain `:`
    let path = cgroups.lines().find_map(|line| line.strip_prefix("0::"))?;
    min_cgroup_quota("/sys/fs/cgroup", path)
}

/// Return the smallest CPU quota of the cgroup at `path` and its ancestors.
///
/// A quota set on any ancestor also limits the process, and a cgroup
/// whose `cpu.max` is missing or `max` sets no limit of its own.
fn min_cgroup_quota(root: &str, path: &str) -> Option<usize> {
    let mut dir = format!("{root}{}", path.trim_end_matches('/'));
    let mut min: Option<usize> = None;
    loop {
        if let Some(quota) = read_to_string(format!("{dir}/cpu.max"))
            .ok()
            .as_deref()
            .and_then(parse_cpu_max)
        {
            min = Some(min.map_or(quota, |m| m.min(quota)));
        }
        if dir.len() <= root.len() {
            return min;
        }
        dir.truncate(dir.rfind('/').unwrap_or(0).max(root.len()));
    }
}

/// Parse a `cpu.max` file (`$QUOTA $PERIOD`) into a number of CPUs.
///
/// Returns `None` for an unlimited (`max`) quota.
fn parse_cpu_max(cpu_max: &str) -> Option<usize> {
    let mut pair = cpu_max.split_whitespace();
    let quota = pair.next()?.parse::<usize>().ok()?;
    // kernel does not provide 0 period. But it seems GNU cares about it
    let period = pair.next()?.parse::<usize>().ok().filter(|&p| p > 0)?;
    // mimic GNU's rounding
    Some(quota.saturating_add(period / 2) / period)
}
