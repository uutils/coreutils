// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use std::io;

use uucore::entries::uid2usr;
use uucore::process::geteuid;

pub fn get_username() -> io::Result<OsString> {
    // uid2usr should arguably return an OsString but currently doesn't
    uid2usr(geteuid()).map(Into::into)
}
