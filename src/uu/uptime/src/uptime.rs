// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore behaviour loadavg nusers

mod platform;

use clap::{Arg, ArgAction, Command};
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use std::io::{self, Write, stdout};
use thiserror::Error;
use uucore::error::{UError, UResult};
use uucore::format_usage;
use uucore::libc::time_t;
use uucore::translate;
use uucore::uptime::{
    OutputFormat, format_nusers, get_formatted_loadavg, get_formatted_nusers, get_formatted_time,
    get_formatted_uptime,
};

pub mod options {
    pub static SINCE: &str = "since";
    pub static PATH: &str = "path";
    pub static PRETTY: &str = "pretty";
}

#[derive(Debug, Error)]
pub enum UptimeError {
    // io::Error wrapper
    #[error("{}", translate!("uptime-error-io", "error" => format!("{}", .0)))]
    IoErr(#[from] io::Error),
    #[error("{}", translate!("uptime-error-target-is-dir"))]
    TargetIsDir,
    #[error("{}", translate!("uptime-error-target-is-fifo"))]
    TargetIsFifo,
}

impl UError for UptimeError {
    fn code(&self) -> i32 {
        1
    }
}

#[uucore::main(no_signals)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    if matches.get_flag(options::SINCE) {
        return uptime_since();
    }
    if matches.get_flag(options::PRETTY) {
        return pretty_print_uptime();
    }
    if let Some(result) = platform::maybe_uptime_from_file(&matches) {
        return result;
    }
    default_uptime()
}

pub fn uu_app() -> Command {
    #[cfg(not(target_env = "musl"))]
    let about = translate!("uptime-about");
    #[cfg(target_env = "musl")]
    let about = translate!("uptime-about") + &translate!("uptime-about-musl-warning");

    let cmd = Command::new("uptime")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("uptime"))
        .about(about)
        .override_usage(format_usage(&translate!("uptime-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SINCE)
                .short('s')
                .long(options::SINCE)
                .help(translate!("uptime-help-since"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRETTY)
                .short('p')
                .long(options::PRETTY)
                .help(translate!("uptime-help-pretty"))
                .action(ArgAction::SetTrue),
        );
    platform::add_platform_args(cmd)
}

fn uptime_since() -> UResult<()> {
    let uptime = platform::system_uptime_seconds()?;

    let since_date = (Timestamp::now() - uptime.seconds()).to_zoned(TimeZone::system());
    writeln!(stdout(), "{}", since_date.strftime("%Y-%m-%d %H:%M:%S"))?;

    Ok(())
}

/// Default uptime behaviour i.e. when no file argument is given.
fn default_uptime() -> UResult<()> {
    print_time()?;
    print_uptime(None)?;
    print_nusers(None)?;
    print_loadavg()?;

    Ok(())
}

/// Prints the load average with its leading separator, or just the line ending
/// where load averages are unavailable (e.g. Windows), as GNU does.
#[inline]
fn print_loadavg() -> UResult<()> {
    if let Ok(s) = get_formatted_loadavg() {
        write!(stdout(), ",  {s}")?;
    }
    writeln!(stdout())?;
    Ok(())
}

fn print_nusers(nusers: Option<usize>) -> UResult<()> {
    write!(
        stdout(),
        "{}",
        match nusers {
            None => {
                get_formatted_nusers()
            }
            Some(nusers) => {
                format_nusers(nusers)
            }
        }
    )?;

    Ok(())
}

fn print_time() -> UResult<()> {
    write!(stdout(), " {} ", get_formatted_time())?;
    Ok(())
}

fn print_uptime(boot_time: Option<time_t>) -> UResult<()> {
    let localized_text = translate!("uptime-output-up-text");
    let uptime_message = get_formatted_uptime(boot_time, OutputFormat::HumanReadable)?;

    write!(stdout(), "{localized_text} {uptime_message},  ")?;
    Ok(())
}

fn pretty_print_uptime() -> UResult<()> {
    let localized_text = translate!("uptime-output-up-text");
    let uptime_message = get_formatted_uptime(None, OutputFormat::PrettyPrint)?;

    writeln!(stdout(), "{localized_text} {uptime_message}")?;
    Ok(())
}
