// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs biguint modpow unfactored
// NOTE:
//   For BigUint > u128, this implementation attempts factorization using Miller-Rabin,
//   an improved Pollard-rho, and p-1.
//   However, compared to GNU factor, there may still be differences in performance
//   and success rate.
//   To further approach GNU factor behavior, additional algorithms (e.g. ECM)
//   and parameter tuning may be required.

use std::collections::BTreeMap;
use std::io::BufRead;
use std::io::{self, Write, stdin, stdout};

use clap::{Arg, ArgAction, Command};
use num_bigint::BigUint;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, set_exit_code};
use uucore::translate;
use uucore::{format_usage, show_error, show_warning};

mod options {
    pub static EXPONENTS: &str = "exponents";
    pub static HELP: &str = "help";
    pub static NUMBER: &str = "NUMBER";
}

fn write_factors_str(
    num_str: &str,
    w: &mut io::BufWriter<impl Write>,
    print_exponents: bool,
) -> UResult<()> {
    let s = num_str.trim();

    // First, interpret as BigUint.
    let rx_big = s.parse::<BigUint>();
    let Ok(x_big) = rx_big else {
        // Non-fatal error. Proceed to the next input number.
        show_warning!("{}: {}", num_str.maybe_quote(), rx_big.unwrap_err());
        set_exit_code(1);
        return Ok(());
    };

    if x_big <= BigUint::from_u32(1).unwrap() {
        // For values <= 1: as in GNU factor, print the input with no prime factors.
        let empty_primes: BTreeMap<BigUint, usize> = BTreeMap::new();
        write_result_big_uint(w, &x_big, empty_primes, print_exponents)
            .map_err_context(|| translate!("factor-error-write-error"))?;
        return Ok(());
    }

    // Try parsing directly into u64 / u128 and delegate to num_prime if successful.
    // This avoids unnecessary BigUint conversions and speeds up the common cases.
    if let Ok(v) = s.parse::<u64>() {
        let prime_factors = num_prime::nt_funcs::factorize64(v);
        write_result_u64(w, &x_big, prime_factors, print_exponents)
            .map_err_context(|| translate!("factor-error-write-error"))?;
        return Ok(());
    }

    if let Ok(v) = s.parse::<u128>() {
        let prime_factors = num_prime::nt_funcs::factorize128(v);
        write_result_u128(w, &v, prime_factors, print_exponents)
            .map_err_context(|| translate!("factor-error-write-error"))?;
        return Ok(());
    }

    // For BigUint > u128: use our own recursive factorization based on
    // Miller-Rabin + Pollard-rho + p-1.
    let mut factors: Vec<BigUint> = Vec::new();
    let success = factor_biguint_recursive(&x_big, &mut factors);

    if !success {
        // Only set exit code=1 when complete factorization could not be achieved
        set_exit_code(1);
    }

    let factorization = collect_biguint_factors(&factors);
    write_result_big_uint(w, &x_big, factorization, print_exponents)
        .map_err_context(|| translate!("factor-error-write-error"))?;

    Ok(())
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
            let pf = num_prime::nt_funcs::factorize128(x128);
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
            write_factors_str(number, &mut w, print_exponents)?;
        }
    } else {
        let stdin = stdin();
        let lines = stdin.lock().lines();
        for line in lines {
            match line {
                Ok(line) => {
                    for number in line.split_whitespace() {
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
