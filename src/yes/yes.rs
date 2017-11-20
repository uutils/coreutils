#![crate_name = "uu_yes"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: yes (GNU coreutils) 8.13 */

#[macro_use]
extern crate clap;

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::io::Write;
use std::ffi::{OsStr, OsString};

pub fn uumain(args: Vec<OsString>) -> i32 {
    let matches = App::new(executable!(args))
                          .version(crate_version!())
                          .author("uutils developers (https://github.com/uutils)")
                          .about("Repeatedly output a line with all specified STRING(s), or 'y'.")
                          .arg(Arg::with_name("STRING")
                               .help("Sets the strings that should be printed")
                               .index(1)
                               .multiple(true))
                          .get_matches_from(args);

    if !matches.is_present("STRING") {
        exec(OsStr::new("y"));
    } else {
        let mut values = matches.values_of_os("STRING").unwrap();
        let init = OsString::from(values.next().unwrap());
        let msg = values.fold(init, |mut acc, arg| {
            acc.push(" ");
            acc.push(arg);
            acc
        });
        exec(&msg[..]);
    };

    0
}

pub fn exec(string: &OsStr) {
    let output = os_bytesln!(string);
    loop {
        byte_print!(&output);
    }
}
