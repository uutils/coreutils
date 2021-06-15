// * This file is part of the uutils coreutils package.
// *
// * (c) 2014 T. Jameson Little <t.jameson.little@gmail.com>
// * (c) 2020 nicoo <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

#[macro_use]
extern crate uucore;

use std::error::Error;
use std::io::{self, stdin, stdout, BufRead, Write};

mod factor;
use clap::{crate_version, App, Arg};
pub use factor::*;

mod miller_rabin;
pub mod numeric;
mod rho;
pub mod table;

static SUMMARY: &str = "Print the prime factors of the given NUMBER(s).
If none are specified, read from standard input.";

mod options {
    pub static NUMBER: &str = "NUMBER";
}

fn print_factors_str(num_str: &str, w: &mut impl io::Write) -> Result<(), Box<dyn Error>> {
    num_str
        .parse::<u64>()
        .map_err(|e| e.into())
        .and_then(|x| writeln!(w, "{}:{}", x, factor(x)).map_err(|e| e.into()))
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let matches = uu_app().get_matches_from(args);
    let stdout = stdout();
    let mut w = io::BufWriter::new(stdout.lock());

    if let Some(values) = matches.values_of(options::NUMBER) {
        for number in values {
            if let Err(e) = print_factors_str(number, &mut w) {
                show_warning!("{}: {}", number, e);
            }
        }
    } else {
        let stdin = stdin();

        for line in stdin.lock().lines() {
            for number in line.unwrap().split_whitespace() {
                if let Err(e) = print_factors_str(number, &mut w) {
                    show_warning!("{}: {}", number, e);
                }
            }
        }
    }

    if let Err(e) = w.flush() {
        show_error!("{}", e);
    }

    0
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .version(crate_version!())
        .about(SUMMARY)
        .arg(Arg::with_name(options::NUMBER).multiple(true))
}
