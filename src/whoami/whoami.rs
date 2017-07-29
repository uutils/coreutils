#![crate_name = "uu_whoami"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: whoami (GNU coreutils) 8.21 */

#[macro_use]
extern crate clap;

#[macro_use]
extern crate uucore;

use clap::App;
use std::io::Write;
use std::ffi::OsString;

mod platform;

pub fn uumain(args: Vec<OsString>) -> i32 {
    let _ = App::new(executable!(args))
                    .version(crate_version!())
                    .author("uutils developers (https://github.com/uutils)")
                    .about("Print effective user ID.")
                    .get_matches_from(args);

    exec();

    0
}

pub fn exec() {
    let maybe_name = unsafe { platform::getusername() };
    match maybe_name {
        Ok(username) => byte_print!(&os_bytesln!(username)),
        Err(err) => match err.raw_os_error() {
            Some(0) | None => crash!(1, "failed to get username"),
            Some(_) => crash!(1, "failed to get username: {}", err),
        }
    }
}
