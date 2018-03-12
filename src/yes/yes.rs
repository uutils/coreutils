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

use clap::Arg;
use std::borrow::Cow;
use std::io::{self, Write};

// force a re-build whenever Cargo.toml changes
const _CARGO_TOML: &'static str = include_str!("Cargo.toml");

const BUF_SIZE: usize = 8192;

pub fn uumain(args: Vec<String>) -> i32 {
    let app = app_from_crate!().arg(Arg::with_name("STRING").index(1).multiple(true));

    let matches = match app.get_matches_from_safe(args) {
        Ok(m) => m,
        Err(ref e)
            if e.kind == clap::ErrorKind::HelpDisplayed
                || e.kind == clap::ErrorKind::VersionDisplayed =>
        {
            println!("{}", e);
            return 0;
        }
        Err(f) => {
            show_error!("{}", f);
            return 1;
        }
    };

    let string = if let Some(values) = matches.values_of("STRING") {
        let mut result = values.fold(String::new(), |res, s| res + s + " ");
        result.pop();
        Cow::from(result)
    } else {
        Cow::from("y")
    };

    let mut multistring = String::with_capacity(BUF_SIZE);
    while multistring.len() < BUF_SIZE - string.len() - 1 {
        multistring.push_str(&string);
        multistring.push_str("\n");
    }

    exec(&multistring[..]);

    0
}

pub fn exec(string: &str) {
    let stdout_raw = io::stdout();
    let mut stdout = stdout_raw.lock();
    loop {
        writeln!(stdout, "{}", string).unwrap();
    }
}
