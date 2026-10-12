// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore loadavg nusers upsecs utmpxname couldnt

//! Unix implementation of `uptime`'s platform facade: the utmp-file operand
//! and the utmpx-derived boot time for `--since` (on OpenBSD, which has no
//! utmpx binding, boot time comes from [`get_uptime`] directly).

use clap::{Arg, ArgAction, ArgMatches, Command, ValueHint, builder::ValueParser};
use std::ffi::OsString;
use std::io::{Write, stdout};
use uucore::error::UResult;
#[cfg(not(any(target_os = "openbsd", target_os = "android")))]
use uucore::libc::time_t;
use uucore::translate;
use uucore::uptime::get_uptime;
#[cfg(not(any(target_os = "openbsd", target_os = "android")))]
use uucore::utmpx::{BOOT_TIME, USER_PROCESS, Utmpx};

use crate::{UptimeError, options, print_loadavg, print_nusers, print_time, print_uptime};

/// Register platform-only CLI arguments: the utmp file operand, which only
/// makes sense where utmp files exist.
pub(crate) fn add_platform_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new(options::PATH)
            .help(translate!("uptime-help-path"))
            .action(ArgAction::Set)
            .num_args(0..=1)
            .value_parser(ValueParser::os_string())
            .value_hint(ValueHint::AnyPath),
    )
}

/// Run `uptime` against a user-supplied utmp file if the file operand was
/// given, or return `None` to fall through to the default system sources.
pub(crate) fn maybe_uptime_from_file(matches: &ArgMatches) -> Option<UResult<()>> {
    matches
        .get_one::<OsString>(options::PATH)
        .map(uptime_with_file)
}

/// The system uptime in seconds, for `--since`: derived from the utmpx
/// `BOOT_TIME` record where utmpx is available (on OpenBSD, from
/// [`get_uptime`] directly).
pub(crate) fn system_uptime_seconds() -> UResult<i64> {
    #[cfg(not(any(target_os = "openbsd", target_os = "android")))]
    {
        let (boot_time, _) = process_utmpx(None);
        get_uptime(boot_time)
    }
    #[cfg(any(target_os = "openbsd", target_os = "android"))]
    get_uptime(None)
}

fn uptime_with_file(file_path: &OsString) -> UResult<()> {
    use std::fs;
    use std::os::unix::fs::FileTypeExt;
    use uucore::error::set_exit_code;
    use uucore::show_error;

    // Uptime will print loadavg and time to stderr unless we encounter an extra operand.
    let mut non_fatal_error = false;

    // process_utmpx_from_file() doesn't detect or report failures, we check if the path is valid
    // before proceeding with more operations.
    let md_res = fs::metadata(file_path);
    if let Ok(md) = md_res {
        if md.is_dir() {
            show_error!("{}", UptimeError::TargetIsDir);
            non_fatal_error = true;
            set_exit_code(1);
        }
        if md.file_type().is_fifo() {
            show_error!("{}", UptimeError::TargetIsFifo);
            non_fatal_error = true;
            set_exit_code(1);
        }
    } else if let Err(e) = md_res {
        non_fatal_error = true;
        set_exit_code(1);
        show_error!("{}", UptimeError::IoErr(e));
    }
    // utmpxname() returns an -1 , when filename doesn't end with 'x' or its too long.
    // Reference: `<https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/utmpxname.3.html>`

    #[cfg(target_vendor = "apple")]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = file_path.as_os_str().as_bytes();

        if bytes.last() != Some(&b'x') {
            show_error!("{}", translate!("uptime-error-couldnt-get-boot-time"));
            print_time()?;
            write!(stdout(), "{}", translate!("uptime-output-unknown-uptime"))?;
            print_nusers(Some(0))?;
            print_loadavg()?;
            set_exit_code(1);
            return Ok(());
        }
    }

    if non_fatal_error {
        print_time()?;
        write!(stdout(), "{}", translate!("uptime-output-unknown-uptime"))?;
        print_nusers(Some(0))?;
        print_loadavg()?;
        return Ok(());
    }

    print_time()?;
    let user_count;

    #[cfg(not(any(target_os = "openbsd", target_os = "android")))]
    {
        let (boot_time, count) = process_utmpx(Some(file_path));
        if let Some(time) = boot_time {
            print_uptime(Some(time))?;
        } else {
            show_error!("{}", translate!("uptime-error-couldnt-get-boot-time"));
            set_exit_code(1);

            write!(stdout(), "{}", translate!("uptime-output-unknown-uptime"))?;
        }
        user_count = count;
    }

    #[cfg(any(target_os = "openbsd", target_os = "android"))]
    {
        let upsecs = get_uptime(None)?;
        if upsecs >= 0 {
            print_uptime(Some(upsecs))?;
        } else {
            show_error!("{}", translate!("uptime-error-couldnt-get-boot-time"));
            set_exit_code(1);

            write!(stdout(), "{}", translate!("uptime-output-unknown-uptime"))?;
        }
        #[cfg(target_os = "openbsd")]
        {
            user_count =
                uucore::uptime::get_nusers(file_path.to_str().expect("invalid utmp path file"));
        }
        #[cfg(target_os = "android")]
        {
            user_count = 0;
        }
    }

    print_nusers(Some(user_count))?;
    print_loadavg()?;

    Ok(())
}

#[cfg(not(any(target_os = "openbsd", target_os = "android")))]
fn process_utmpx(file: Option<&OsString>) -> (Option<time_t>, usize) {
    let mut nusers = 0;
    let mut boot_time = None;

    let records = match file {
        Some(f) => Utmpx::iter_all_records_from(f),
        None => Utmpx::iter_all_records(),
    };

    for line in records {
        match line.record_type() {
            x if x == USER_PROCESS => nusers += 1,
            x if x == BOOT_TIME => {
                let dt = line.login_time();
                if dt.unix_timestamp() > 0 {
                    boot_time = Some(dt.unix_timestamp() as time_t);
                }
            }
            _ => (),
        }
    }
    (boot_time, nusers)
}
