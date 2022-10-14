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
use std::io::BufRead;
use std::io::{self, stdin, stdout, Write};

mod factor;
use clap::{crate_version, Arg, ArgAction, Command};
pub use factor::*;
use uucore::display::Quotable;
use uucore::error::UResult;

mod miller_rabin;
pub mod numeric;
mod rho;
pub mod table;

static ABOUT: &str = r#"Print the prime factors of the given NUMBER(s).
If none are specified, read from standard input."#;

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
    let matches = uu_app().try_get_matches_from(args)?;
    let stdout = stdout();
    // We use a smaller buffer here to pass a gnu test. 4KiB appears to be the default pipe size for bash.
    let mut w = io::BufWriter::with_capacity(4 * 1024, stdout.lock());
    let mut factors_buffer = String::new();

    if let Some(values) = matches.get_many::<String>(options::NUMBER) {
        for number in values {
            if let Err(e) = print_factors_str(number, &mut w, &mut factors_buffer) {
                show_warning!("{}: {}", number.maybe_quote(), e);
            }
        }
    } else {
        let stdin = stdin();
        let lines = stdin.lock().lines();
        for line in lines {
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

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .infer_long_args(true)
        .arg(Arg::new(options::NUMBER).action(ArgAction::Append))
}
