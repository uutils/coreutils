/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use ::libc;
use std::mem;
use std::io::Write;

extern "system" {
    pub fn GetUserNameA(out: *mut libc::c_char, len: *mut libc::uint32_t) -> libc::uint8_t;
}

#[allow(unused_unsafe)]
pub unsafe fn getusername() -> String {
    // usernames can only be up to 104 characters in windows
    let mut buffer: [libc::c_char; 105] = mem::uninitialized();

    if !GetUserNameA(buffer.as_mut_ptr(), &mut (buffer.len() as libc::uint32_t)) == 0 {
        crash!(1, "username is too long");
    }
    String::from_utf8_lossy(::std::ffi::CStr::from_ptr(buffer.as_ptr()).to_bytes()).to_string()
}
