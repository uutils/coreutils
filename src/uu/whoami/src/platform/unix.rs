/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 * (c) Jian Zeng <anonymousknight96 AT gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::ffi::OsString;
use std::io;

use uucore::entries::uid2usr;

pub fn get_username() -> io::Result<OsString> {
    // SAFETY: getuid() does nothing with memory and is always successful.
    let uid = unsafe { libc::geteuid() };
    // uid2usr should arguably return an OsString but currently doesn't
    uid2usr(uid).map(Into::into)
}
