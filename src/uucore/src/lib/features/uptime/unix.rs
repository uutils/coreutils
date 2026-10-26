// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore gettime BOOTTIME clockid boottime nusers loadavg getloadavg cfgs

//! Unix implementation of the platform side of `uucore::uptime`: system
//! uptime, user count and load average. The macOS/NetBSD/Cygwin and OpenBSD
//! variants live here as target cfgs.

use super::UptimeError;
use crate::error::UResult;
use std::time::Duration;

/// Safely get macOS boot time using sysctl command
///
/// This function uses the sysctl command-line tool to retrieve the kernel
/// boot time on macOS, avoiding any unsafe code. It parses the output
/// of the sysctl command to extract the boot time.
///
/// # Returns
///
/// Returns Some(i64) if successful, None if the call fails.
#[cfg(target_vendor = "apple")]
fn get_macos_boot_time_sysctl() -> Option<i64> {
    use std::process::Command;

    // Execute sysctl command to get boot time
    let output = Command::new("sysctl")
        .arg("-n")
        .arg("kern.boottime")
        .output();

    if let Ok(output) = output
        && output.status.success()
    {
        // Parse output format: { sec = 1729338352, usec = 0 } Wed Oct 19 08:25:52 2025
        // We need to extract the seconds value from the structured output
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract the seconds from the output
        // Look for "sec = " pattern
        if let Some(sec_start) = stdout.find("sec = ") {
            let sec_part = &stdout[sec_start + 6..];
            if let Some(sec_end) = sec_part.find(',') {
                let sec_str = &sec_part[..sec_end];
                if let Ok(boot_time) = sec_str.trim().parse::<i64>() {
                    return Some(boot_time);
                }
            }
        }
    }

    None
}

/// Get the system uptime
///
/// # Arguments
///
/// `boot_time`: `Option<i64>` - Manually specify the boot time, or None to try to get it from the system.
///
/// # Returns
///
/// Returns a `UResult` with the elapsed duration since boot if successful, otherwise an `UptimeError`.
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "fuchsia",
    target_os = "openbsd",
))]
#[allow(clippy::unnecessary_wraps, reason = "needed on some platforms")]
pub fn get_uptime(_boot_time: Option<i64>) -> UResult<Duration> {
    use rustix::time::{ClockId, clock_gettime};

    let tp = clock_gettime(ClockId::Boottime);

    let seconds = u64::try_from(tp.tv_sec).map_err(|_| UptimeError::SystemUptime)?;
    Ok(Duration::new(seconds, tp.tv_nsec as u32))
}

/// Get the system uptime
///
/// # Arguments
///
/// `boot_time`: `Option<i64>` - Manually specify the boot time, or None to try to get it from the system.
///
/// # Returns
///
/// Returns a `UResult` with the elapsed duration since boot if successful, otherwise an [`UptimeError`].
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "fuchsia",
    target_os = "openbsd",
)))]
pub fn get_uptime(boot_time: Option<i64>) -> UResult<Duration> {
    use crate::utmpx::BOOT_TIME;
    use crate::utmpx::Utmpx;
    use jiff::Timestamp;
    use std::fs::File;
    use std::io::Read;

    let mut proc_uptime_s = String::new();

    let proc_uptime = File::open("/proc/uptime")
        .ok()
        .and_then(|mut f| f.read_to_string(&mut proc_uptime_s).ok())
        .and_then(|_| proc_uptime_s.split_whitespace().next())
        .and_then(|s| s.parse::<f64>().ok())
        .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok());

    if let Some(uptime) = proc_uptime {
        return Ok(uptime);
    }

    // Try provided boot_time or derive from utmpx
    let derived_boot_time = boot_time.or_else(|| {
        Utmpx::iter_all_records()
            .filter(|r| r.record_type() == BOOT_TIME)
            .map(|r| r.login_time().unix_timestamp())
            .find(|&ts| ts > 0)
    });

    // macOS-specific fallback: use sysctl kern.boottime when utmpx did not provide BOOT_TIME
    //
    // On macOS, the utmpx BOOT_TIME record can be unreliable or absent, causing intermittent
    // test failures (see issue #3621: https://github.com/uutils/coreutils/issues/3621).
    // The sysctl(CTL_KERN, KERN_BOOTTIME) approach is the canonical way to retrieve boot time
    // on macOS and is always available, making uptime more reliable on this platform.
    //
    // This fallback only runs if utmpx failed to provide a boot time.
    #[cfg(target_vendor = "apple")]
    let derived_boot_time = {
        let mut t = derived_boot_time;
        if t.is_none() {
            // Use a safe wrapper function to get boot time via sysctl
            if let Some(boot_time) = get_macos_boot_time_sysctl() {
                t = Some(boot_time);
            }
        }
        t
    };

    if let Some(boottime) = derived_boot_time {
        let now = Timestamp::now();
        if now.as_second() < boottime {
            Err(UptimeError::BootTime)?;
        }
        return Ok(Duration::new(
            (now.as_second() - boottime) as u64,
            now.subsec_nanosecond() as u32,
        ));
    }

    Err(UptimeError::SystemUptime)?
}

/// Get the number of users currently logged in
///
/// # Returns
///
/// Returns the number of users currently logged in if successful, otherwise 0.
#[cfg(not(any(target_os = "openbsd", target_os = "android")))]
// see: https://gitlab.com/procps-ng/procps/-/blob/4740a0efa79cade867cfc7b32955fe0f75bf5173/library/uptime.c#L63-L115
pub fn get_nusers() -> usize {
    use crate::utmpx::USER_PROCESS;
    use crate::utmpx::Utmpx;

    let mut num_user = 0;
    Utmpx::iter_all_records().for_each(|ut| {
        if ut.record_type() == USER_PROCESS {
            num_user += 1;
        }
    });
    num_user
}

/// Get the number of users currently logged in
///
/// # Returns
///
/// Returns the number of users currently logged in if successful, otherwise 0
#[cfg(target_os = "openbsd")]
pub fn get_nusers(file: &str) -> usize {
    use utmp_classic::{UtmpEntry, parse_from_path};

    let Ok(entries) = parse_from_path(file) else {
        return 0;
    };

    if entries.is_empty() {
        return 0;
    }

    // Count entries that have a non-empty user field
    entries
        .iter()
        .filter_map(|entry| match entry {
            UtmpEntry::UTMP { user, .. } if !user.is_empty() => Some(()),
            _ => None,
        })
        .count()
}

/// Get the number of users from the default system source, for
/// [`super::get_formatted_nusers`]. On OpenBSD the default source is
/// `/var/run/utmp`.
pub(crate) fn default_nusers() -> usize {
    #[cfg(not(any(target_os = "openbsd", target_os = "android")))]
    return get_nusers();
    #[cfg(target_os = "openbsd")]
    return get_nusers("/var/run/utmp");
    #[cfg(target_os = "android")]
    return 0;
}

/// Get the system load average
///
/// # Returns
///
/// Returns a `UResult` with the load average if successful, otherwise an [`UptimeError`].
/// The load average is a tuple of three floating point numbers representing the 1-minute, 5-minute, and 15-minute load averages.
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub fn get_loadavg() -> UResult<(f64, f64, f64)> {
    use core::ffi::c_double;
    use libc::getloadavg;

    let mut avg: [c_double; 3] = [0.0; 3];
    // SAFETY: checked whether it returns -1
    let loads: i32 = unsafe { getloadavg(avg.as_mut_ptr(), 3) };

    if loads == -1 {
        Err(UptimeError::SystemLoadavg)?
    } else {
        Ok((avg[0], avg[1], avg[2]))
    }
}

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
pub fn get_loadavg() -> UResult<(f64, f64, f64)> {
    Err(UptimeError::SystemLoadavg)?
}

#[cfg(all(test, target_vendor = "apple"))]
mod tests {
    use super::*;
    use jiff::Timestamp;

    /// Test that sysctl kern.boottime is accessible on macOS and returns valid boot time.
    /// This ensures the fallback mechanism added for issue #3621 works correctly.
    #[test]
    fn test_macos_sysctl_boottime_available() {
        // Test the safe wrapper function
        let boot_time = get_macos_boot_time_sysctl();

        // Verify the safe wrapper succeeded
        assert!(
            boot_time.is_some(),
            "get_macos_boot_time_sysctl should succeed on macOS"
        );

        let boot_time = boot_time.unwrap();

        // Verify boot time is valid (positive, reasonable value)
        assert!(boot_time > 0, "Boot time should be positive");

        // Boot time should be after 2000-01-01 (946684800 seconds since epoch)
        assert!(
            boot_time > 946_684_800,
            "Boot time should be after year 2000"
        );

        // Boot time should be before current time
        let boot_time = Timestamp::from_second(boot_time).unwrap();
        let now = Timestamp::now();
        assert!(boot_time < now, "Boot time should be before current time");
    }

    /// Test that `get_uptime` always succeeds on macOS due to sysctl fallback.
    /// This addresses the intermittent failures reported in issue #3621.
    #[test]
    fn test_get_uptime_always_succeeds_on_macos() {
        // Call get_uptime without providing boot_time, forcing the system
        // to use utmpx or fall back to sysctl
        let result = get_uptime(None);

        assert!(
            result.is_ok(),
            "get_uptime should always succeed on macOS with sysctl fallback"
        );

        let uptime = result.unwrap();
        assert!(!uptime.is_zero(), "Uptime should be positive");

        // Reasonable upper bound: system hasn't been up for more than 365 days
        // (This is just a sanity check)
        assert!(
            uptime < Duration::from_secs(365 * 86400),
            "Uptime seems unreasonably high: {uptime:?}"
        );
    }

    /// Test `get_uptime` consistency by calling it multiple times.
    /// Verifies the sysctl fallback produces stable results.
    #[test]
    fn test_get_uptime_macos_consistency() {
        let uptime1 = get_uptime(None).expect("First call should succeed");
        let uptime2 = get_uptime(None).expect("Second call should succeed");

        // Uptimes should be very close (within 1 second)
        let diff = uptime1.abs_diff(uptime2);
        assert!(
            diff <= Duration::from_secs(1),
            "Consecutive uptime calls should be consistent, got {uptime1:?} and {uptime2:?}"
        );
    }
}

#[cfg(all(
    test,
    any(
        target_os = "linux",
        target_os = "android",
        target_os = "fuchsia",
        target_os = "openbsd"
    )
))]
mod clock_tests {
    use super::*;
    use rustix::time::{ClockId, clock_gettime};

    #[test]
    fn uptime_retains_fractional_clock_precision() {
        let before = clock_gettime(ClockId::Boottime);
        let uptime = get_uptime(None).unwrap();
        let after = clock_gettime(ClockId::Boottime);
        let before = Duration::new(before.tv_sec as u64, before.tv_nsec as u32);
        let after = Duration::new(after.tv_sec as u64, after.tv_nsec as u32);
        assert!(
            before <= uptime && uptime <= after,
            "{before:?} <= {uptime:?} <= {after:?}"
        );
    }
}
