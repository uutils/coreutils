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
pub(crate) use factor::*;

mod miller_rabin;
mod numeric;
mod rho;
mod table;

static SYNTAX: &str = "[OPTION] [NUMBER]...";
static SUMMARY: &str = "Print the prime factors of the given number(s).
 If none are specified, read from standard input.";
static LONG_HELP: &str = "";

fn print_factors_str(num_str: &str, w: &mut impl io::Write) -> Result<(), Box<dyn Error>> {
    num_str
        .parse::<u64>()
        .map_err(|e| e.into())
        .and_then(|x| writeln!(w, "{}:{}", x, factor(x)).map_err(|e| e.into()))
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let matches = app!(SYNTAX, SUMMARY, LONG_HELP).parse(args.collect_str());
    let stdout = stdout();
    let mut w = io::BufWriter::new(stdout.lock());

    if matches.free.is_empty() {
        let stdin = stdin();

        for line in stdin.lock().lines() {
            for number in line.unwrap().split_whitespace() {
                if let Err(e) = print_factors_str(number, &mut w) {
                    show_warning!("{}: {}", number, e);
                }
            }
        }
    } else {
        for number in &matches.free {
            if let Err(e) = print_factors_str(number, &mut w) {
                show_warning!("{}: {}", number, e);
            }
        }
    }

    if let Err(e) = w.flush() {
        show_error!("{}", e);
    }

    0
}
