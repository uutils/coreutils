// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs biguint modpow unfactored newtype
// NOTE:
//   For BigUint > u128, this implementation attempts factorization using Miller-Rabin,
//   an improved Pollard-rho, and p-1.
//   However, compared to GNU factor, there may still be differences in performance
//   and success rate.
//   To further approach GNU factor behavior, additional algorithms (e.g. ECM)
//   and parameter tuning may be required.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::BufRead;
use std::io::{self, Write, stdin, stdout};
use std::iter::once;
use std::num::IntErrorKind;

use clap::{Arg, ArgAction, Command};
use memchr::memchr3_iter;
use num_bigint::BigUint;
use num_prime::nt_funcs::{factorize64, factorize128};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
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
        // For BigUint > u128: use our own recursive factorization based on
        // Miller-Rabin + Pollard-rho + p-1.
        Number::BigUint(x) => {
            if x <= BigUint::from_u32(1).unwrap() {
                // For values <= 1: as in GNU factor, print the input with no prime factors.
                let empty_primes: BTreeMap<BigUint, usize> = BTreeMap::new();
                write_result(w, &x, empty_primes, print_exponents)
            } else {
                let mut factors: Vec<BigUint> = Vec::new();
                let success = factor_biguint_recursive(&x, &mut factors);

                if !success {
                    // Only set exit code=1 when complete factorization could not be achieved
                    set_exit_code(1);
                }

                let factorization = collect_biguint_factors(&factors);
                write_result(w, &x, factorization, print_exponents)
            }
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

fn collect_biguint_factors(factors: &[BigUint]) -> BTreeMap<BigUint, usize> {
    let mut map = BTreeMap::<BigUint, usize>::new();
    for f in factors {
        *map.entry(f.clone()).or_insert(0) += 1;
    }
    map
}

fn is_even(value: &BigUint) -> bool {
    (value & BigUint::one()).is_zero()
}

fn is_probable_prime(candidate: &BigUint) -> bool {
    if *candidate < BigUint::from_u32(2).unwrap() {
        return false;
    }
    if *candidate == BigUint::from_u32(2).unwrap() || *candidate == BigUint::from_u32(3).unwrap() {
        return true;
    }
    // even check: candidate % 2 == 0
    if is_even(candidate) {
        return false;
    }

    let one = BigUint::one();
    let two = BigUint::from_u32(2).unwrap();

    // candidate - 1 = odd_component * 2^power_of_two
    let mut odd_component = candidate - &one;
    let mut power_of_two = 0u32;
    // while odd_component is even
    while is_even(&odd_component) {
        odd_component >>= 1;
        power_of_two += 1;
    }

    let bases_32: [u64; 3] = [2, 7, 61];
    let bases_64: [u64; 12] = [
        2,
        325,
        9375,
        28178,
        450_775,
        9_780_504,
        1_795_265_022,
        3,
        5,
        7,
        11,
        13,
    ];

    let bases: Vec<u64> = if candidate.bits() <= 32 {
        bases_32.to_vec()
    } else if candidate.bits() <= 64 {
        bases_64.to_vec()
    } else {
        vec![2, 3, 5, 7, 11, 13, 17, 19, 23]
    };

    'outer: for &base_value in &bases {
        if BigUint::from(base_value) >= *candidate {
            continue;
        }
        let base = BigUint::from(base_value);
        let mut witness = base.modpow(&odd_component, candidate);
        if witness == one || witness == candidate - &one {
            continue 'outer;
        }

        for _ in 1..power_of_two {
            witness = witness.modpow(&two, candidate);
            if witness == candidate - &one {
                continue 'outer;
            }
            if witness == one {
                return false;
            }
        }
        return false;
    }

    true
}

fn small_trial_division(n: &BigUint) -> Option<BigUint> {
    // Quickly strip very small prime factors before applying expensive algorithms.
    // This is intentionally lightweight while still covering a reasonably wide range.
    // GNU factor maintains a large trial table; here we mimic only a small portion of it.
    //
    // NOTE: By removing many small prime factors here, we significantly reduce the
    //       search space and failure count of subsequent Pollard-rho / p-1 steps.
    const SMALL_PRIMES: [u16; 54] = [
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
        97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181,
        191, 193, 197, 199, 211, 223, 227, 229, 233, 239, 241, 251,
    ];

    for &p in &SMALL_PRIMES {
        let p_big = BigUint::from_u32(p as u32).unwrap();
        if n == &p_big {
            return None;
        }
        if (n % &p_big).is_zero() {
            return Some(p_big);
        }
    }
    None
}

/// Simplified Pollard p-1 method (Stage 1 only).
/// Effective when p-1 (for a prime divisor p of n) is smooth with small prime factors.
fn pollard_p_minus_1(n: &BigUint) -> Option<BigUint> {
    // Stage 1 only (simplified).
    // Best-effort: we do not spend too long here; give up quickly if it does not help.
    let one = BigUint::one();
    let two = BigUint::from_u32(2).unwrap();

    if n.is_zero() || n.is_one() {
        return None;
    }

    if is_even(n) {
        return Some(two);
    }

    let bits = n.bits();

    // Keep B1 relatively small to avoid excessive cost.
    // (GNU factor adjusts based on input and retries; we use a fixed approximation.)
    let b1: u64 = if bits <= 256 {
        10_000
    } else if bits <= 512 {
        20_000
    } else {
        50_000
    };

    // Only try a few small prime bases.
    const BASES: [u64; 3] = [2, 3, 5];

    for &base in &BASES {
        let mut a = BigUint::from(base);
        if &a >= n {
            continue;
        }

        let mut g = gcd_biguint(&a, n);
        if g > one && &g < n {
            return Some(g);
        }

        // Construct a^(M) step by step (approximating by extending the exponent with 2^k)
        let mut e = 2u64;
        while e <= b1 {
            a = a.modpow(&BigUint::from_u64(e).unwrap(), n);
            if a.is_one() {
                break;
            }
            let am1 = if a > one { &a - &one } else { continue };
            g = gcd_biguint(&am1, n);
            if g > one && &g < n {
                return Some(g);
            }
            e <<= 1;
        }
    }

    None
}

/// Improved Pollard-rho (Brent variant with batched gcd).
/// Not equivalent to GNU factor, but aims for better convergence and performance
/// than a naive implementation.
fn pollard_rho(composite: &BigUint) -> Option<BigUint> {
    // NOTE:
    //  - This implementation is inspired by the approach in GNU factor but simplified.
    //  - For large inputs we avoid running too long; we cap the iterations so that
    //    we do not spend many seconds on hopeless cases.
    //  - If factorization fails, we return "Factorization incomplete"-style results.
    let one = BigUint::one();
    let two = BigUint::from_u32(2).unwrap();

    // For small n we expect earlier code paths to have handled the input.
    if *composite <= BigUint::from_u32(3).unwrap() {
        return None;
    }

    // If composite is even, return 2 immediately.
    if is_even(composite) {
        return Some(two);
    }

    // Use a deterministic LCG to generate parameter sequences.
    const LCG_MULTIPLIER: u128 = 6_364_136_223_846_793_005;
    const LCG_INCREMENT: u128 = 1_442_695_040_888_963_407;

    fn lcg_next(x: &mut u128) {
        *x = x.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_INCREMENT);
    }

    let bits = composite.bits();

    // Search parameters: choose bounds based on bit length.
    // Avoid overly large limits; when exhausted, treat as failure to find a factor.
    let max_tries: u64 = 16;
    let max_iter: u64 = (bits * bits).clamp(10_000, 200_000);

    const LCG_DEFAULT_SEED: u128 = 0x9e37_79b9_7f4a_7c15;
    let mut seed: u128 = LCG_DEFAULT_SEED;

    for _try in 0..max_tries {
        lcg_next(&mut seed);
        let mut x_state = BigUint::from(seed % (u128::MAX / 2 + 1));
        lcg_next(&mut seed);
        let mut constant = BigUint::from(seed % (u128::MAX / 2 + 1));
        if constant.is_zero() {
            constant = BigUint::from(1u32);
        }
        x_state %= composite;
        constant %= composite;

        let mut y_state = x_state.clone();
        let mut current_gcd = one.clone();
        let mut product = one.clone();

        let mut iter: u64 = 0;
        let batch_size: u64 = 128;

        while current_gcd == one && iter < max_iter {
            // Brent variant: use batched gcd.
            let mut batch_iter = 0;
            let x_saved = x_state.clone();
            while batch_iter < batch_size && iter < max_iter {
                // f(z) = z^2 + c mod composite.
                y_state = (&y_state * &y_state + &constant) % composite;
                let diff = if x_state > y_state {
                    &x_state - &y_state
                } else {
                    &y_state - &x_state
                };
                if !diff.is_zero() {
                    product = (product * diff) % composite;
                }
                batch_iter += 1;
                iter += 1;
            }
            current_gcd = gcd_biguint(&product, composite);
            x_state = x_saved;

            if current_gcd == one {
                // Update x_state to advance the sequence.
                x_state.clone_from(&y_state);
            }
        }

        if current_gcd == one {
            continue;
        }
        if &current_gcd == composite {
            // Fallback: step-by-step gcd.
            let mut z_state = x_state.clone();
            loop {
                z_state = (&z_state * &z_state + &constant) % composite;
                let diff = if z_state > y_state {
                    &z_state - &y_state
                } else {
                    &y_state - &z_state
                };
                current_gcd = gcd_biguint(&diff, composite);
                if current_gcd > one || z_state == y_state {
                    break;
                }
            }
        }

        if current_gcd > one && &current_gcd < composite {
            return Some(current_gcd);
        }
    }

    None
}

/// Recursively factor n and append factors (primes or unfactored composites) to `factors`.
/// Returns true if full factorization succeeded, false otherwise.
fn gcd_biguint(lhs: &BigUint, rhs: &BigUint) -> BigUint {
    // Standard Euclidean algorithm using owned BigUint values to avoid lifetime issues.
    let mut dividend = lhs.clone();
    let mut divisor = rhs.clone();
    while !divisor.is_zero() {
        let remainder = &dividend % &divisor;
        dividend = divisor;
        divisor = remainder;
    }
    dividend
}

fn factor_biguint_recursive(n: &BigUint, factors: &mut Vec<BigUint>) -> bool {
    let one = BigUint::one();
    if *n <= one {
        return true;
    }

    // First remove small prime factors, then apply more expensive methods.
    if let Some(p) = small_trial_division(n) {
        let mut q = n.clone();
        let mut ok = true;
        while (&q % &p).is_zero() {
            q /= &p;
            factors.push(p.clone());
        }
        if !q.is_one() {
            ok &= factor_biguint_recursive(&q, factors);
        }
        return ok;
    }

    // If n is small enough, use num_prime's factorize128 for speed.
    if n.bits() <= 128 {
        if let Some(x128) = n.to_u128() {
            let pf = factorize128(x128);
            if !pf.is_empty() {
                for (p, e) in pf {
                    for _ in 0..e {
                        factors.push(BigUint::from(p));
                    }
                }
                return true;
            }
        }
    }

    if is_probable_prime(n) {
        factors.push(n.clone());
        return true;
    }

    // Try Pollard p-1 first (simplified Stage 1).
    if let Some(f) = pollard_p_minus_1(n) {
        if f.is_one() || &f == n {
            // Treat as failure.
        } else {
            let q = n / &f;
            let left_ok = factor_biguint_recursive(&f, factors);
            let right_ok = factor_biguint_recursive(&q, factors);
            return left_ok && right_ok;
        }
    }

    // Then try improved Pollard-rho (Brent variant).
    if let Some(f) = pollard_rho(n) {
        if f.is_one() || &f == n {
            factors.push(n.clone());
            return false;
        }
        let q = n / &f;
        let left_ok = factor_biguint_recursive(&f, factors);
        let right_ok = factor_biguint_recursive(&q, factors);
        return left_ok && right_ok;
    }

    // If no factor was found, include n itself as part of the (incomplete) factorization.
    factors.push(n.clone());
    false
}

/// Writing out the prime factors for integers
fn write_result<T: Display>(
    w: &mut io::BufWriter<impl Write>,
    x: &impl Display,
    factorization: BTreeMap<T, usize>,
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
