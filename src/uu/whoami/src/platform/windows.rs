// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use std::io;
use std::os::windows::ffi::OsStringExt;
use windows::Win32::NetworkManagement::NetManagement::UNLEN;
use windows::Win32::System::WindowsProgramming::GetUserNameW;
use windows::core::PWSTR;

pub fn get_username() -> io::Result<OsString> {
    const BUF_LEN: u32 = UNLEN + 1;
    let mut buffer = [0_u16; BUF_LEN as usize];
    let mut len = BUF_LEN;
    // SAFETY: buffer.len() == len
    let result = unsafe { GetUserNameW(PWSTR::from_raw(buffer.as_mut_ptr()), &mut len) };
    
    match result {
        Ok(_) => Ok(OsString::from_wide(&buffer[..len as usize - 1])),
        Err(_) =>  Err(io::Error::last_os_error()),
    }
}
