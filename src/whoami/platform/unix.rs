/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 * (c) Jian Zeng <anonymousknight96 AT gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::io::Result;
use std::ffi::OsString;
use uucore::libc::geteuid;
use uucore::entries::uid2usr;

pub unsafe fn getusername() -> Result<OsString> {
    // Get effective user id
    let uid = geteuid();
    // XXX: uid2usr returns a String, which may or may not be fine given that I am not sure if
    //      there are systems that allow non-utf8 characters in /etc/passwd
    uid2usr(uid).map(|name| OsString::from(name))
}
