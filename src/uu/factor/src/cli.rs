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
use std::fmt::Write as FmtWrite;
use std::io::{self, stdin, stdout, BufRead, Write};

mod factor;
use clap::{crate_version, Arg, Command};
pub use factor::*;
use uucore::display::Quotable;
use uucore::error::UResult;

mod miller_rabin;
pub mod numeric;
mod rho;
pub mod table;

static SUMMARY: &str = "Print the prime factors of the given NUMBER(s).
If none are specified, read from standard input.";

mod options {
    pub static NUMBER: &str = "NUMBER";
}

fn print_factors_str(
    num_str: &str,
    w: &mut io::BufWriter<impl io::Write>,
    factors_buffer: &mut String,
) -> Result<(), Box<dyn Error>> {
    num_str.parse::<u64>().map_err(|e| e.into()).and_then(|x| {
        factors_buffer.clear();
        writeln!(factors_buffer, "{}:{}", x, factor(x))?;
        w.write_all(factors_buffer.as_bytes())?;
        Ok(())
    })
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);
    let stdout = stdout();
    // We use a smaller buffer here to pass a gnu test. 4KiB appears to be the default pipe size for bash.
    let mut w = io::BufWriter::with_capacity(4 * 1024, stdout.lock());
    let mut factors_buffer = String::new();

    if let Some(values) = matches.values_of(options::NUMBER) {
        for number in values {
            if let Err(e) = print_factors_str(number, &mut w, &mut factors_buffer) {
                show_warning!("{}: {}", number.maybe_quote(), e);
            }
        }
    } else {
        let stdin = stdin();

        for line in stdin.lock().lines() {
            for number in line.unwrap().split_whitespace() {
                if let Err(e) = print_factors_str(number, &mut w, &mut factors_buffer) {
                    show_warning!("{}: {}", number.maybe_quote(), e);
                }
            }
        }
    }

    if let Err(e) = w.flush() {
        show_error!("{}", e);
    }

    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(SUMMARY)
        .infer_long_args(true)
        .arg(Arg::new(options::NUMBER).multiple_occurrences(true))
}
