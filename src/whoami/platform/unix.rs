/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use ::libc;
use self::c_types::{c_passwd, getpwuid};

#[path = "../../common/c_types.rs"] mod c_types;

extern {
    pub fn geteuid() -> libc::uid_t;
}

pub unsafe fn getusername() -> String {
    let passwd: *const c_passwd = getpwuid(geteuid());

    let pw_name: *const libc::c_char = (*passwd).pw_name;
    String::from_utf8_lossy(::std::ffi::CStr::from_ptr(pw_name).to_bytes()).to_string()
}
