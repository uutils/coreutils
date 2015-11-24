/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::io::{Result, Error};
use ::libc;
use uucore::c_types::{c_passwd, getpwuid};

extern {
    pub fn geteuid() -> libc::uid_t;
}

pub unsafe fn getusername() -> Result<String> {
    // Get effective user id
    let uid = geteuid();

    // Try to find username for uid
    let passwd: *const c_passwd = getpwuid(uid);
    if passwd.is_null() {
        return Err(Error::last_os_error())
    }

    // Extract username from passwd struct
    let pw_name: *const libc::c_char = (*passwd).pw_name;
    let username = String::from_utf8_lossy(::std::ffi::CStr::from_ptr(pw_name).to_bytes()).to_string();
    Ok(username)
}
