// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use std::io;
use std::os::windows::ffi::OsStringExt;
use windows_sys::Win32::NetworkManagement::NetManagement::UNLEN;
use windows_sys::Win32::System::WindowsProgramming::GetUserNameW;

pub fn get_username() -> io::Result<OsString> {
    const BUF_LEN: u32 = UNLEN + 1;
    let mut buffer = [0_u16; BUF_LEN as usize];
    let mut len = BUF_LEN;
    // SAFETY: buffer.len() == len
    if unsafe { GetUserNameW(buffer.as_mut_ptr(), &mut len) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(OsString::from_wide(&buffer[..len as usize - 1]))
}
