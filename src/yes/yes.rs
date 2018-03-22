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

// it's possible that using a smaller or larger buffer might provide better performance on some
// systems, but honestly this is good enough
const BUF_SIZE: usize = 16 * 1024;

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
        result.push('\n');
        Cow::from(result)
    } else {
        Cow::from("y\n")
    };

    let mut buffer = [0; BUF_SIZE];
    let bytes = if string.len() < BUF_SIZE / 2 {
        let mut size = 0;
        while size < BUF_SIZE - string.len() {
            let (_, right) = buffer.split_at_mut(size);
            right[..string.len()].copy_from_slice(string.as_bytes());
            size += string.len();
        }
        &buffer[..size]
    } else {
        string.as_bytes()
    };

    exec(bytes);

    0
}

pub fn exec(bytes: &[u8]) {
    let stdout_raw = io::stdout();
    let mut stdout = stdout_raw.lock();
    loop {
        stdout.write_all(bytes).unwrap();
    }
}
