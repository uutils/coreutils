//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: whoami (GNU coreutils) 8.21 */

#[macro_use]
extern crate uucore;

use crate::app::get_app;

mod app;
mod platform;

pub fn uumain(args: impl uucore::Args) -> i32 {
    let app = get_app(executable!());

    if let Err(err) = app.get_matches_from_safe(args) {
        if err.kind == clap::ErrorKind::HelpDisplayed
            || err.kind == clap::ErrorKind::VersionDisplayed
        {
            println!("{}", err);
            0
        } else {
            show_error!("{}", err);
            1
        }
    } else {
        exec();

        0
    }
}

pub fn exec() {
    unsafe {
        match platform::get_username() {
            Ok(username) => println!("{}", username),
            Err(err) => match err.raw_os_error() {
                Some(0) | None => crash!(1, "failed to get username"),
                Some(_) => crash!(1, "failed to get username: {}", err),
            },
        }
    }
}
