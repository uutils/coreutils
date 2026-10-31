// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) getlogin userlogin runas

use clap::Command;
use std::io::{Write, stdout};
use uucore::translate;
use uucore::{error::UResult, show_error};

#[cfg(unix)]
fn get_userlogin() -> Option<String> {
    use core::ffi::CStr;
    let login_ptr = unsafe { libc::getlogin() };
    if login_ptr.is_null() {
        None
    } else {
        Some(String::from_utf8_lossy(unsafe { CStr::from_ptr(login_ptr) }.to_bytes()).to_string())
    }
}

#[cfg(windows)]
fn get_userlogin() -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::RemoteDesktop::{
        WTS_CURRENT_SERVER_HANDLE, WTS_CURRENT_SESSION, WTSFreeMemory, WTSQuerySessionInformationW,
        WTSUserName,
    };
    // Ask for the user logged on to this session rather than the process token's user
    // (`GetUserNameW`, as `whoami` does), so `runas` or a service account still reports
    // the person who logged in, like `getlogin` does on Unix.
    let mut buffer = std::ptr::null_mut();
    let mut bytes = 0_u32;
    // SAFETY: on success the buffer holds `bytes` bytes, a NUL-terminated UTF-16 string.
    let ok = unsafe {
        WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            WTS_CURRENT_SESSION,
            WTSUserName,
            &raw mut buffer,
            &raw mut bytes,
        )
    };
    if ok == 0 || buffer.is_null() {
        return None;
    }
    // `bytes` includes the terminating NUL
    let len = (bytes as usize / 2).saturating_sub(1);
    // SAFETY: `buffer` is valid for `len` u16s and is freed right after copying.
    let name = OsString::from_wide(unsafe { std::slice::from_raw_parts(buffer, len) });
    // SAFETY: `buffer` was allocated by `WTSQuerySessionInformationW`, is freed exactly once
    // and is not used afterwards; `name` owns its own copy.
    unsafe { WTSFreeMemory(buffer.cast()) };
    // Session 0 (services) has no logged-on user and returns an empty name.
    (!name.is_empty()).then(|| name.to_string_lossy().into_owned())
}

#[uucore::main(no_signals)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let _ = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    if let Some(userlogin) = get_userlogin() {
        writeln!(stdout(), "{userlogin}")?;
        Ok(())
    } else {
        show_error!("{}", translate!("logname-error-no-login-name"));
        Err(1.into())
    }
}

pub fn uu_app() -> Command {
    Command::new("logname")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("logname"))
        .override_usage(translate!("logname-usage"))
        .about(translate!("logname-about"))
        .infer_long_args(true)
}
