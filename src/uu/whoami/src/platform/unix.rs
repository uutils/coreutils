/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 * (c) Jian Zeng <anonymousknight96 AT gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

// spell-checker:ignore (ToDO) getusername

use std::io::Result;
use uucore::entries::uid2usr;
use uucore::libc::geteuid;

pub unsafe fn get_username() -> Result<String> {
    // Get effective user id
    let uid = geteuid();
    uid2usr(uid)
}
