// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs

use std::collections::BTreeMap;
use std::io::{self, BufRead, Write, stdin, stdout};

use clap::{Arg, ArgAction, Command};
use num_bigint::BigUint;
use num_traits::cast::{FromPrimitive, ToPrimitive};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, set_exit_code};
use uucore::translate;
use uucore::{format_usage, show_error, show_warning};

/// Result type for factorization operations
type FactorResult = BTreeMap<BigUint, usize>;

mod options {
    pub static EXPONENTS: &str = "exponents";
    pub static HELP: &str = "help";
    pub static NUMBER: &str = "NUMBER";
}

/// Factorization for numbers > u128 using num_prime algorithms
/// (trial division, Pollard's rho, and other methods as configured)
fn factor_large_number_recursive(number: &BigUint) -> FactorResult {
    // Use num_prime's factors function with strict configuration for better factorization
    use num_prime::FactorizationConfig;
    let config = FactorizationConfig::strict();
    let (mut factors, remainder) = num_prime::nt_funcs::factors(number.clone(), Some(config));

    // If there's a remainder, recursively factorize it
    if let Some(remainders) = remainder {
        for rem in remainders {
            if rem > BigUint::from_u32(1).unwrap() {
                let sub_factors = factor_large_number_recursive(&rem);
                for (factor, count) in sub_factors {
                    *factors.entry(factor).or_insert(0) += count;
                }
            }
        }
    }

    factors
}

fn print_factors_str(
    num_str: &str,
    w: &mut io::BufWriter<impl Write>,
    print_exponents: bool,
) -> UResult<()> {
    let x = match num_str.trim().parse::<BigUint>() {
        Ok(x) => x,
        Err(e) => {
            // return Ok(). it's non-fatal and we should try the next number.
            show_warning!("{}: {}", num_str.maybe_quote(), e);
            set_exit_code(1);
            return Ok(());
        }
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
            let x_u128 = x.to_u128().unwrap();
            let prime_factors = num_prime::nt_funcs::factorize128(x_u128);
            write_result_u128(w, &x_u128, prime_factors, print_exponents)
                .map_err_context(|| translate!("factor-error-write-error"))?;
        }
        // For numbers greater than u128::MAX, use recursive factorization
        else {
            show_warning!(
                "Number {num_str} exceeds u128 limits. Using recursive factorization (may be slow)."
            );

            let factors = factor_large_number_recursive(&x);
            write_result_big_uint(w, &x, factors, print_exponents)
                .map_err_context(|| translate!("factor-error-write-error"))?;
        }
    } else {
        let empty_primes: BTreeMap<BigUint, usize> = BTreeMap::new();
        write_result_big_uint(w, &x, empty_primes, print_exponents)
            .map_err_context(|| translate!("factor-error-write-error"))?;
    }

    Ok(())
}

/// Macro to generate write_result functions for different numeric types
macro_rules! write_result {
    ($name:ident, $x_type:ty, $factor_type:ty) => {
        #[doc = concat!("Writing out the prime factors for ", stringify!($factor_type), " integers")]
        fn $name(
            w: &mut io::BufWriter<impl Write>,
            x: &$x_type,
            factorization: BTreeMap<$factor_type, usize>,
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
    };
}

write_result!(write_result_u64, BigUint, u64);
write_result!(write_result_u128, u128, u128);
write_result!(write_result_big_uint, BigUint, BigUint);

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
                    show_error!(
                        "{}",
                        translate!("factor-error-reading-input ", "error" => e)
                    );
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
        .about(translate!("factor-about "))
        .override_usage(format_usage(&translate!("factor-usage ")))
        .infer_long_args(true)
        .disable_help_flag(true)
        .args_override_self(true)
        .arg(Arg::new(options::NUMBER).action(ArgAction::Append))
        .arg(
            Arg::new(options::EXPONENTS)
                .short('h')
                .long(options::EXPONENTS)
                .help(translate!("factor-help-exponents "))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("factor-help-help "))
                .action(ArgAction::Help),
        )
}
