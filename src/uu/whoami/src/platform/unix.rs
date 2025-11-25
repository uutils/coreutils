use std::ffi::OsString;
use std::io;

use uucore::entries::uid2usr;
use uucore::process::geteuid;

pub fn get_username() -> io::Result<OsString> {
    // uid2usr should arguably return an OsString but currently doesn't
    uid2usr(geteuid()).map(Into::into)
}
