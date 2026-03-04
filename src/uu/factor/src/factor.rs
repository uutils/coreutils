// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs

use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::BufRead;
use std::io::{self, Write, stdin, stdout};
use std::str::FromStr;

use clap::{Arg, ArgAction, Command};
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, set_exit_code};
use uucore::translate;
use uucore::{format_usage, show_error, show_if_err};

mod options {
    pub static EXPONENTS: &str = "exponents";
    pub static HELP: &str = "help";
    pub static NUMBER: &str = "NUMBER";
}

const LF: u8 = b'\n';
const DELIM_SPACE: u8 = b' ';

fn write_factors_str(
    num_str: &[u8],
    w: &mut io::BufWriter<impl Write>,
    print_exponents: bool,
) -> UResult<()> {
    let parsed = parse_num::<BigUint>(num_str);
    show_if_err!(&parsed);
    let Ok(x) = parsed else {
        return Ok(());
    };

    if x > BigUint::from_u32(1).unwrap() {
        // use num_prime's factorize64 algorithm for u64 integers
        if x <= BigUint::from_u64(u64::MAX).unwrap() {
            let prime_factors = num_prime::nt_funcs::factorize64(x.clone().to_u64_digits()[0]);
            write_result_u64(w, &x, prime_factors, print_exponents)
                .map_err_context(|| translate!("factor-error-write-error"))?;
        }
        // use num_prime's factorize128 algorithm for u128 integers
        else if x <= BigUint::from_u128(u128::MAX).unwrap() {
            let parsed = parse_num::<u128>(num_str);
            show_if_err!(&parsed);
            let Ok(x) = parsed else {
                return Ok(());
            };
            let prime_factors = num_prime::nt_funcs::factorize128(x);
            write_result_u128(w, &x, prime_factors, print_exponents)
                .map_err_context(|| translate!("factor-error-write-error"))?;
        }
        // use num_prime's fallible factorization for anything greater than u128::MAX
        else {
            let (prime_factors, remaining) = num_prime::nt_funcs::factors(x.clone(), None);
            if let Some(_remaining) = remaining {
                return Err(USimpleError::new(
                    1,
                    translate!("factor-error-factorization-incomplete"),
                ));
            }
            write_result_big_uint(w, &x, prime_factors, print_exponents)
                .map_err_context(|| translate!("factor-error-write-error"))?;
        }
    } else {
        let empty_primes: BTreeMap<BigUint, usize> = BTreeMap::new();
        write_result_big_uint(w, &x, empty_primes, print_exponents)
            .map_err_context(|| translate!("factor-error-write-error"))?;
    }

    Ok(())
}

fn parse_num<T: FromStr>(slice: &[u8]) -> UResult<T> {
    str::from_utf8(slice)
        .ok()
        .and_then(|str| str.trim_matches(DELIM_SPACE as char).parse().ok())
        .ok_or_else(|| {
            let num = NumError(slice).to_string();
            let num = if num.len() > slice.len() {
                num.quote() // Force quoting if there are invalid characters.
            } else {
                num.maybe_quote()
            };
            USimpleError::new(
                1,
                format!("warning: {num}: {}", translate!("factor-error-invalid-int")),
            )
        })
}

struct NumError<'a>(&'a [u8]);

impl Display for NumError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match str::from_utf8(self.0) {
            Ok(s) => write!(f, "{s}"),
            Err(e) => {
                let valid = e.valid_up_to();
                let cont = valid + e.error_len().unwrap_or(1);
                write!(f, "{}", unsafe {
                    str::from_utf8_unchecked(&self.0[..valid])
                })?;
                for b in &self.0[valid..cont] {
                    write!(f, "\\{b:03o}")?;
                }
                <Self as Display>::fmt(&Self(&self.0[cont..]), f)
            }
        }
    }
}

/// Writing out the prime factors for u64 integers
fn write_result_u64(
    w: &mut io::BufWriter<impl Write>,
    x: &BigUint,
    factorization: BTreeMap<u64, usize>,
    print_exponents: bool,
) -> io::Result<()> {
    write!(w, "{x}:")?;
    for (factor, n) in factorization {
        if print_exponents {
            if n > 1 {
                write!(w, " {factor}^{n}")?;
            } else {
                write!(w, " {factor}")?;
            }
        } else {
            w.write_all(format!(" {factor}").repeat(n).as_bytes())?;
        }
    }
    writeln!(w)?;
    w.flush()
}

/// Writing out the prime factors for u128 integers
fn write_result_u128(
    w: &mut io::BufWriter<impl Write>,
    x: &u128,
    factorization: BTreeMap<u128, usize>,
    print_exponents: bool,
) -> io::Result<()> {
    write!(w, "{x}:")?;
    for (factor, n) in factorization {
        if print_exponents {
            if n > 1 {
                write!(w, " {factor}^{n}")?;
            } else {
                write!(w, " {factor}")?;
            }
        } else {
            w.write_all(format!(" {factor}").repeat(n).as_bytes())?;
        }
    }
    writeln!(w)?;
    w.flush()
}

/// Writing out the prime factors for BigUint integers
fn write_result_big_uint(
    w: &mut io::BufWriter<impl Write>,
    x: &BigUint,
    factorization: BTreeMap<BigUint, usize>,
    print_exponents: bool,
) -> io::Result<()> {
    write!(w, "{x}:")?;
    for (factor, n) in factorization {
        if print_exponents {
            if n > 1 {
                write!(w, " {factor}^{n}")?;
            } else {
                write!(w, " {factor}")?;
            }
        } else {
            w.write_all(format!(" {factor}").repeat(n).as_bytes())?;
        }
    }
    writeln!(w)?;
    w.flush()
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    // If matches find --exponents flag than variable print_exponents is true and p^e output format will be used.
    let print_exponents = matches.get_flag(options::EXPONENTS);

    let stdout = stdout();
    // We use a smaller buffer here to pass a gnu test. 4KiB appears to be the default pipe size for bash.
    let mut w = io::BufWriter::with_capacity(4 * 1024, stdout.lock());

    if let Some(values) = matches.get_many::<String>(options::NUMBER) {
        for number in values {
            write_factors_str(number.as_bytes(), &mut w, print_exponents)?;
        }
    } else {
        let stdin = stdin();
        let lines = stdin.lock().split(LF);
        for line in lines {
            match line {
                Ok(line) => {
                    for number in line.split(|c| (*c as char).is_whitespace()) {
                        write_factors_str(number, &mut w, print_exponents)?;
                    }
                }
                Err(e) => {
                    set_exit_code(1);
                    show_error!("{}", translate!("factor-error-reading-input", "error" => e));
                    return Ok(());
                }
            }
        }
    }

    if let Err(e) = w.flush() {
        show_error!("{e}");
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("factor-about"))
        .override_usage(format_usage(&translate!("factor-usage")))
        .infer_long_args(true)
        .disable_help_flag(true)
        .args_override_self(true)
        .arg(Arg::new(options::NUMBER).action(ArgAction::Append))
        .arg(
            Arg::new(options::EXPONENTS)
                .short('h')
                .long(options::EXPONENTS)
                .help(translate!("factor-help-exponents"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("factor-help-help"))
                .action(ArgAction::Help),
        )
}
