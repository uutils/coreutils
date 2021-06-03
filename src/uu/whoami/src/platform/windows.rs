/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate winapi;

use self::winapi::shared::lmcons;
use self::winapi::shared::minwindef;
use self::winapi::um::winnt;
use std::io::{Error, Result};
use std::mem;
use uucore::wide::FromWide;

pub unsafe fn get_username() -> Result<String> {
    #[allow(deprecated)]
    let mut buffer: [winnt::WCHAR; lmcons::UNLEN as usize + 1] = mem::uninitialized();
    let mut len = buffer.len() as minwindef::DWORD;
    if advapi32::GetUserNameW(buffer.as_mut_ptr(), &mut len) == 0 {
        return Err(Error::last_os_error());
    }
    let username = String::from_wide(&buffer[..len as usize - 1]);
    Ok(username)
}
