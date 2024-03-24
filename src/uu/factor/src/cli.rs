// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io::BufRead;
use std::io::{self, stdin, stdout, Write};

mod factor;
use clap::{crate_version, Arg, ArgAction, Command};
pub use factor::*;
use uucore::display::Quotable;
use uucore::error::{set_exit_code, FromIo, UResult};
use uucore::{format_usage, help_about, help_usage, show_error, show_warning};

mod miller_rabin;
pub mod numeric;
mod rho;
pub mod table;

const ABOUT: &str = help_about!("factor.md");
const USAGE: &str = help_usage!("factor.md");

mod options {
    pub static EXPONENTS: &str = "exponents";
    pub static HELP: &str = "help";
    pub static NUMBER: &str = "NUMBER";
}

fn print_factors_str(
    num_str: &str,
    w: &mut io::BufWriter<impl io::Write>,
    print_exponents: bool,
) -> io::Result<()> {
    let x = match num_str.trim().parse::<u64>() {
        Ok(x) => x,
        Err(e) => {
            // We return Ok() instead of Err(), because it's non-fatal and we should try the next
            // number.
            show_warning!("{}: {}", num_str.maybe_quote(), e);
            set_exit_code(1);
            return Ok(());
        }
    };

    // If print_exponents is true, use the alternate format specifier {:#} from fmt to print the factors
    // of x in the form of p^e.
    if print_exponents {
        writeln!(w, "{}:{:#}", x, factor(x))?;
    } else {
        writeln!(w, "{}:{}", x, factor(x))?;
    }
    w.flush()
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    // If matches find --exponents flag than variable print_exponents is true and p^e output format will be used.
    let print_exponents = matches.get_flag(options::EXPONENTS);

    let stdout = stdout();
    // We use a smaller buffer here to pass a gnu test. 4KiB appears to be the default pipe size for bash.
    let mut w = io::BufWriter::with_capacity(4 * 1024, stdout.lock());

    if let Some(values) = matches.get_many::<String>(options::NUMBER) {
        for number in values {
            print_factors_str(number, &mut w, print_exponents)
                .map_err_context(|| "write error".into())?;
        }
    } else {
        let stdin = stdin();
        let lines = stdin.lock().lines();
        for line in lines {
            match line {
                Ok(line) => {
                    for number in line.split_whitespace() {
                        print_factors_str(number, &mut w, print_exponents)
                            .map_err_context(|| "write error".into())?;
                    }
                }
                Err(e) => {
                    set_exit_code(1);
                    show_error!("error reading input: {}", e);
                    return Ok(());
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
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .disable_help_flag(true)
        .args_override_self(true)
        .arg(Arg::new(options::NUMBER).action(ArgAction::Append))
        .arg(
            Arg::new(options::EXPONENTS)
                .short('h')
                .long(options::EXPONENTS)
                .help("Print factors in the form p^e")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help),
        )
}
