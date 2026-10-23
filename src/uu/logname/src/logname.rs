// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) getlogin userlogin

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
    use windows_sys::Win32::System::WindowsProgramming::GetUserNameW;
    // UNLEN (256) + terminating NUL
    let mut buffer = [0_u16; 257];
    let mut len = buffer.len() as u32;
    if unsafe { GetUserNameW(buffer.as_mut_ptr(), &raw mut len) } == 0 || len < 2 {
        return None;
    }
    // `len` includes the terminating NUL
    Some(
        OsString::from_wide(&buffer[..len as usize - 1])
            .to_string_lossy()
            .into_owned(),
    )
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
