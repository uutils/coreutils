// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs newtype

use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::BufRead;
use std::io::{self, Write, stdin, stdout};
use std::iter::once;
use std::num::IntErrorKind;

use clap::{Arg, ArgAction, Command};
use memchr::memchr3_iter;
use num_bigint::BigUint;
use num_prime::nt_funcs::{factorize64, factorize128, factors};
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
const CR: u8 = b'\r';
const DELIM_SPACE: u8 = b' ';
const DELIM_TAB: u8 = b'\t';
const DELIM_NULL: u8 = b'\0';

#[derive(Debug, PartialEq, Eq)]
enum Number {
    U64(u64),
    U128(u128),
    BigUint(BigUint),
}

fn write_factors_str(
    num_str: &[u8],
    w: &mut io::BufWriter<impl Write>,
    print_exponents: bool,
) -> UResult<()> {
    let parsed = parse_num(num_str);
    show_if_err!(&parsed);
    let Ok(x) = parsed else {
        // return Ok(). it's non-fatal and we should try the next number.
        return Ok(());
    };

    match x {
        // use num_prime's factorize64 algorithm for u64 integers
        Number::U64(x) if x > 1 => write_result(w, &x, factorize64(x), print_exponents),
        Number::U64(x) => write_result(w, &x, BTreeMap::<u64, usize>::new(), print_exponents),
        // use num_prime's factorize128 algorithm for u128 integers
        Number::U128(x) => write_result(w, &x, factorize128(x), print_exponents),
        // use num_prime's fallible factorization for anything greater than u128::MAX
        Number::BigUint(x) => {
            let (prime_factors, remaining) = factors(x.clone(), None);
            if remaining.is_some() {
                return Err(USimpleError::new(
                    1,
                    translate!("factor-error-factorization-incomplete"),
                ));
            }
            write_result(w, &x, prime_factors, print_exponents)
        }
    }
    .map_err_context(|| translate!("factor-error-write-error"))
}

fn parse_num(slice: &[u8]) -> UResult<Number> {
    let err_invalid = |s: &str, force_quoting| {
        let num = if force_quoting {
            s.quote() // Force quoting if there are invalid characters.
        } else {
            s.maybe_quote()
        };
        USimpleError::new(
            1,
            format!("warning: {num}: {}", translate!("factor-error-invalid-int")),
        )
    };
    let num = str::from_utf8(slice).map_err(|_| err_invalid(&NumError(slice).to_string(), true))?;

    match num.parse::<u64>() {
        Ok(x) => return Ok(Number::U64(x)),
        // If overflown, attempt a greater width
        Err(e) if matches!(e.kind(), IntErrorKind::PosOverflow) => {}
        Err(_) => return Err(err_invalid(num, false)),
    }

    match num.parse::<u128>() {
        Ok(x) => return Ok(Number::U128(x)),
        // If overflown, attempt a greater width
        Err(e) if matches!(e.kind(), IntErrorKind::PosOverflow) => {}
        Err(_) => return Err(err_invalid(num, false)),
    }

    num.parse::<BigUint>()
        .map(Number::BigUint)
        .map_err(|_| err_invalid(num, false))
}

/// This is a newtype wrapper over a potentially malformed UTF-8
/// string of a number, which has an optimized [`Display`] impl
/// matching GNU's formatting. This can be removed in favor of
/// the C quoting in uucore once support for byte slices is added.
#[repr(transparent)]
struct NumError<'a>(&'a [u8]);

impl Display for NumError<'_> {
    /// This function formats the valid string segments and displays
    /// the invalid ones as escaped octal, like GNU. For example, the
    /// (escaped) string "\xFFabc\x1C" is formatted as "\377abc\034".
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match str::from_utf8(self.0) {
            Ok(s) => write!(f, "{s}"),
            Err(e) => {
                let valid = e.valid_up_to();
                let cont = valid + e.error_len().unwrap_or(1);
                // SAFETY: `self.0` has been checked to contain valid
                // UTF-8 sequences up to `valid`.
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

/// Writing out the prime factors for integers
fn write_result(
    w: &mut io::BufWriter<impl Write>,
    x: &impl Display,
    factorization: BTreeMap<impl Display, usize>,
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
            write_factors_str(number.trim().as_bytes(), &mut w, print_exponents)?;
        }
    } else {
        let stdin = stdin();
        let lines = stdin.lock().split(LF);
        for line in lines {
            match line {
                Ok(line) => {
                    // Ignore CR on Windows if present; disabled everywhere else for GNU compatibility.
                    let le = if cfg!(windows) && line.last() == Some(&CR) {
                        line.len() - 1
                    } else {
                        line.len()
                    };

                    // GNU factor treats numbers optionally as null-terminated due to its
                    // implementation details. Here we also split the line with nulls and
                    // ignore those chunks until another delimiter is found.
                    let (mut display, mut prev) = (true, 0);
                    for i in memchr3_iter(DELIM_SPACE, DELIM_TAB, DELIM_NULL, &line).chain(once(le))
                    {
                        let has_null = line.get(i) == Some(&DELIM_NULL);
                        if display && (prev != i || has_null) {
                            write_factors_str(&line[prev..i], &mut w, print_exponents)?;
                        }
                        (display, prev) = (!has_null, i + 1);
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
    Command::new("factor")
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

#[cfg(test)]
mod tests {
    use crate::Number;
    use crate::parse_num;

    #[test]
    fn test_correct_parsing() {
        const U128_MAX_ONE: &str = "340282366920938463463374607431768211456";
        let zero = parse_num(b"0").unwrap();
        let u64_max = parse_num(u64::MAX.to_string().as_bytes()).unwrap();
        let u128_one = parse_num((u64::MAX as u128 + 1).to_string().as_bytes()).unwrap();
        let u128_max = parse_num(u128::MAX.to_string().as_bytes()).unwrap();
        let bigint = parse_num(U128_MAX_ONE.as_bytes()).unwrap();
        assert_eq!(
            (
                Number::U64(0),
                Number::U64(u64::MAX),
                Number::U128(u64::MAX as u128 + 1),
                Number::U128(u128::MAX),
                Number::BigUint(U128_MAX_ONE.parse().unwrap())
            ),
            (zero, u64_max, u128_one, u128_max, bigint)
        );
    }

    #[test]
    #[should_panic]
    fn test_incorrect_parsing() {
        parse_num(b"abcd").unwrap();
        parse_num(b"12\x00\xFF\x1D").unwrap();
    }
}
