// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore nusers loadavg

//! Provides functions to get system uptime, number of users and load average.
//!
//! The platform-specific sources live in the cfg-gated `unix`/`windows`
//! submodules, which both provide `get_uptime`, `get_nusers`, `get_loadavg`
//! and `default_nusers` with identical signatures (one exception: OpenBSD's
//! `get_nusers` takes the utmp file path as an argument); the shared
//! formatting helpers here only talk to those re-exported functions.

// The code was originally written in uu_uptime
// (https://github.com/uutils/coreutils/blob/main/src/uu/uptime/src/uptime.rs)
// but was eventually moved here.
// See https://github.com/uutils/coreutils/pull/7289 for discussion.

use crate::error::{UError, UResult};
use crate::translate;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use libc::time_t;
use thiserror::Error;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[derive(Debug, Error)]
pub enum UptimeError {
    #[error("{}", translate!("uptime-lib-error-system-uptime"))]
    SystemUptime,
    #[error("{}", translate!("uptime-lib-error-system-loadavg"))]
    SystemLoadavg,
    #[error("{}", translate!("uptime-lib-error-windows-loadavg"))]
    WindowsLoadavg,
    #[error("{}", translate!("uptime-lib-error-boot-time"))]
    BootTime,
}

impl UError for UptimeError {
    fn code(&self) -> i32 {
        1
    }
}

/// Returns the formatted time string, e.g. "12:34:56"
pub fn get_formatted_time() -> String {
    Timestamp::now()
        .to_zoned(TimeZone::system())
        .strftime("%H:%M:%S")
        .to_string()
}

/// The format used to display a FormattedUptime.
pub enum OutputFormat {
    /// Typical `uptime` output (e.g. 2 days, 3:04).
    HumanReadable,

    /// Pretty printed output (e.g. 2 days, 3 hours, 04 minutes).
    PrettyPrint,
}

struct FormattedUptime {
    days: i64,
    hours: i64,
    mins: i64,
}

impl FormattedUptime {
    fn new(seconds: i64) -> Self {
        let days = seconds / 86400;
        let hours = (seconds - (days * 86400)) / 3600;
        let mins = (seconds - (days * 86400) - (hours * 3600)) / 60;

        Self { days, hours, mins }
    }

    fn get_human_readable_uptime(&self) -> String {
        // Hours are not zero-padded (issue #13027); minutes always are.
        translate!(
        "uptime-format",
        "days" => self.days,
        "time" => format!("{}:{:02}", self.hours, self.mins))
    }

    fn get_pretty_print_uptime(&self) -> String {
        let mut parts = Vec::new();
        if self.days > 0 {
            parts.push(translate!("uptime-format-pretty-day", "day" => self.days));
        }
        if self.hours > 0 {
            parts.push(translate!("uptime-format-pretty-hour", "hour" => self.hours));
        }
        if self.mins > 0 || parts.is_empty() {
            parts.push(translate!("uptime-format-pretty-min", "min" => self.mins));
        }
        parts.join(", ")
    }
}

/// Get the system uptime in a human-readable format
///
/// # Arguments
///
/// boot_time: Option<time_t> - Manually specify the boot time, or None to try to get it from the system.
/// output_format: OutputFormat - Selects the format of the output string.
///
/// # Returns
///
/// Returns a UResult with the uptime in a human-readable format(e.g. "1 day, 3:45") if successful, otherwise an UptimeError.
#[inline]
pub fn get_formatted_uptime(
    boot_time: Option<time_t>,
    output_format: OutputFormat,
) -> UResult<String> {
    let uptime = get_uptime(boot_time)?;

    if uptime < 0 {
        Err(UptimeError::SystemUptime)?;
    }

    let formatted_uptime = FormattedUptime::new(uptime);

    match output_format {
        OutputFormat::HumanReadable => Ok(formatted_uptime.get_human_readable_uptime()),
        OutputFormat::PrettyPrint => Ok(formatted_uptime.get_pretty_print_uptime()),
    }
}

/// Format the number of users to a human-readable string
///
/// # Returns
///
/// e.g. "0 users", "1 user", "2 users"
#[inline]
pub fn format_nusers(n: usize) -> String {
    translate!(
        "uptime-user-count",
        "count" => n
    )
}

/// Get the number of users currently logged in, in a human-readable format
///
/// # Returns
///
/// e.g. "0 user", "1 user", "2 users"
#[inline]
pub fn get_formatted_nusers() -> String {
    format_nusers(default_nusers())
}

/// Get the system load average in a human-readable format
///
/// # Returns
///
/// Returns a UResult with the load average in a human-readable format if successful, otherwise an UptimeError.
/// e.g. "load average: 0.00, 0.00, 0.00"
#[inline]
pub fn get_formatted_loadavg() -> UResult<String> {
    let loadavg = get_loadavg()?;
    let mut args = fluent::FluentArgs::new();
    args.set("avg1", format!("{:.2}", loadavg.0));
    args.set("avg5", format!("{:.2}", loadavg.1));
    args.set("avg15", format!("{:.2}", loadavg.2));
    Ok(crate::locale::get_message_with_args(
        "uptime-lib-format-loadavg",
        args,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale;

    #[test]
    fn test_format_nusers() {
        unsafe {
            std::env::set_var("LANG", "en_US.UTF-8");
        }
        let _ = locale::setup_localization("uptime");
        assert_eq!("0 users", format_nusers(0));
        assert_eq!("1 user", format_nusers(1));
        assert_eq!("2 users", format_nusers(2));
    }

    #[test]
    fn test_human_readable_uptime_hours_not_zero_padded() {
        unsafe {
            std::env::set_var("LANG", "en_US.UTF-8");
        }
        let _ = locale::setup_localization("uptime");
        // Hours below 10 are not zero-padded (issue #13027).
        assert_eq!(
            "1:27",
            FormattedUptime::new(3600 + 27 * 60).get_human_readable_uptime()
        );
        assert_eq!(
            "9:05",
            FormattedUptime::new(9 * 3600 + 5 * 60).get_human_readable_uptime()
        );
        // Two-digit hours are unchanged.
        assert_eq!(
            "10:05",
            FormattedUptime::new(10 * 3600 + 5 * 60).get_human_readable_uptime()
        );
    }
}
