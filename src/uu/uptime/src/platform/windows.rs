// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Windows implementation of `uptime`'s platform facade. There are no utmp
//! files on Windows, so the file-operand facades degenerate, and `--since`
//! reads the uptime directly from [`uucore::uptime::get_uptime`]
//! (`GetTickCount64`).

use clap::{ArgMatches, Command};
use std::time::Duration;
use uucore::error::UResult;
use uucore::uptime::get_uptime;

/// No platform-only CLI arguments on Windows (no utmp files).
pub(crate) fn add_platform_args(cmd: Command) -> Command {
    cmd
}

/// The utmp file operand does not exist on Windows; never handled here.
pub(crate) fn maybe_uptime_from_file(_matches: &ArgMatches) -> Option<UResult<()>> {
    None
}

/// The elapsed duration since boot, for `--since`.
pub(crate) fn system_uptime() -> UResult<Duration> {
    get_uptime(None)
}
