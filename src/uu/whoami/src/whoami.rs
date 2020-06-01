//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: whoami (GNU coreutils) 8.21 */

// spell-checker:ignore (ToDO) getusername

#[macro_use]
extern crate clap;
#[macro_use]
extern crate uucore;

mod platform;

pub fn uumain(args: Vec<String>) -> i32 {
    let app = app_from_crate!();

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
        match platform::getusername() {
            Ok(username) => println!("{}", username),
            Err(err) => match err.raw_os_error() {
                Some(0) | None => crash!(1, "failed to get username"),
                Some(_) => crash!(1, "failed to get username: {}", err),
            },
        }
    }
}
