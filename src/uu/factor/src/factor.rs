// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs

use std::collections::BTreeMap;
use std::io::BufRead;
use std::io::{self, stdin, stdout, Write};

use num_bigint::BigUint;
use num_traits::FromPrimitive;
use uucore::display::Quotable;
use uucore::error::{set_exit_code, FromIo, UResult, USimpleError};
use uucore::{show_error, show_warning};

fn print_factors_str(
    num_str: &str,
    w: &mut io::BufWriter<impl io::Write>,
    print_exponents: bool,
) -> UResult<()> {
    let rx = num_str.trim().parse::<num_bigint::BigUint>();
    let Ok(x) = rx else {
        // return Ok(). it's non-fatal and we should try the next number.
        show_warning!("{}: {}", num_str.maybe_quote(), rx.unwrap_err());
        set_exit_code(1);
        return Ok(());
    };

    let (factorization, remaining) = if x > BigUint::from_u32(1).unwrap() {
        num_prime::nt_funcs::factors(x.clone(), None)
    } else {
        (BTreeMap::new(), None)
    };

    if let Some(_remaining) = remaining {
        return Err(USimpleError::new(
            1,
            "Factorization incomplete. Remainders exists.",
        ));
    }

    write_result(w, &x, factorization, print_exponents).map_err_context(|| "write error".into())?;

    Ok(())
}

fn write_result(
    w: &mut io::BufWriter<impl Write>,
    x: &BigUint,
    factorization: BTreeMap<BigUint, usize>,
    print_exponents: bool,
) -> io::Result<()> {
    write!(w, "{x}:")?;
    for (factor, n) in factorization {
        if print_exponents {
            if n > 1 {
                write!(w, " {}^{}", factor, n)?;
            } else {
                write!(w, " {}", factor)?;
            }
        } else {
            w.write_all(format!(" {}", factor).repeat(n).as_bytes())?;
        }
    }
    writeln!(w)?;
    w.flush()
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().try_get_matches_from(args)?;

    // If matches find --exponents flag than variable print_exponents is true and p^e output format will be used.
    let print_exponents = matches.get_flag(crate::options::EXPONENTS);

    let stdout = stdout();
    // We use a smaller buffer here to pass a gnu test. 4KiB appears to be the default pipe size for bash.
    let mut w = io::BufWriter::with_capacity(4 * 1024, stdout.lock());

    if let Some(values) = matches.get_many::<String>(crate::options::NUMBER) {
        for number in values {
            print_factors_str(number, &mut w, print_exponents)?;
        }
    } else {
        let stdin = stdin();
        let lines = stdin.lock().lines();
        for line in lines {
            match line {
                Ok(line) => {
                    for number in line.split_whitespace() {
                        print_factors_str(number, &mut w, print_exponents)?;
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
