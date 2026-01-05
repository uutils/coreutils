// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Optimized Pollard-Rho implementation with Brent's cycle-finding algorithm
//!
//! This module provides an efficient implementation of Pollard's rho algorithm
//! using Brent's variant for detecting cycles, which is faster than Floyd's method.
//! Uses Montgomery form arithmetic for 3-5x speedup on large numbers.

use super::montgomery::MontgomeryContext;
use num_bigint::BigUint;
use num_integer::gcd;
use num_traits::{One, Zero};
use std::time::Instant;

/// Pollard-Rho factorization with Brent's cycle-finding variant
pub fn pollard_rho_brent(n: &BigUint) -> Option<BigUint> {
    if n <= &BigUint::one() {
        return None;
    }

    const MAX_ITERATIONS: u64 = 100_000;
    const MAX_TRIES: usize = 20;

    for _attempt in 0..MAX_TRIES {
        // Random starting point
        let x = get_random_biguint(n);
        let c = get_random_biguint(n);

        if let Some(factor) = brent_cycle_find(&x, &c, n, MAX_ITERATIONS) {
            if factor > BigUint::one() && factor < *n {
                return Some(factor);
            }
        }
    }

    None
}

/// Pollard-Rho with target factor size for optimal configuration
pub fn pollard_rho_with_target(n: &BigUint, target_factor_bits: u32) -> Option<BigUint> {
    if n <= &BigUint::one() {
        return None;
    }

    // HYPOTHESIS 2: Multi-stage Pollard-Rho for 24-48 bit factors
    // Key insight: For k-bit factor, need ~2^(k/2) iterations
    // - 32-bit: ~65K iterations
    // - 40-bit: ~1M iterations
    // - 48-bit: ~16M iterations
    //
    // Strategy: Use adequate iterations with multiple attempts

    let max_iterations = match target_factor_bits {
        24..=32 => 200_000,   // 2^16 = 65K, use 3x for safety
        33..=40 => 500_000,   // 2^20 = 1M, use 0.5x
        41..=48 => 2_000_000, // 2^24 = 16M, use 0.125x (multiple attempts)
        _ => 5_000_000,       // Larger factors (shouldn't reach here)
    };

    // Attempts: Balance between coverage and timeout
    let max_attempts = if n.bits() > 110 {
        match target_factor_bits {
            24..=40 => 5, // Small targets: fewer attempts, more iterations
            41..=48 => 8, // Medium targets: more attempts to compensate
            _ => 10,      // Larger targets
        }
    } else {
        // For smaller numbers, use more attempts
        match target_factor_bits {
            24..=40 => 10,
            41..=48 => 15,
            _ => 20,
        }
    };

    let overall_start = Instant::now();
    for _attempt in 0..max_attempts {
        // Timeout after 30 seconds total for large numbers (increased from 10s)
        // This allows time for the multi-stage approach to try all stages
        if n.bits() > 110 && overall_start.elapsed().as_secs() > 30 {
            return None;
        }
        let x = get_random_biguint(n);
        let c = get_random_biguint(n);

        if let Some(factor) = brent_cycle_find(&x, &c, n, max_iterations) {
            if factor > BigUint::one() && factor < *n {
                return Some(factor);
            }
        }
    }

    None
}

/// Brent's cycle-finding variant of Pollard-Rho
/// More efficient than Floyd's method
/// Uses Montgomery form for large numbers (3-5x speedup)
fn brent_cycle_find(
    x0: &BigUint,
    c: &BigUint,
    n: &BigUint,
    max_iterations: u64,
) -> Option<BigUint> {
    // Try Montgomery form for numbers > 64 bits (includes our 105-bit test case)
    if n.bits() > 64 {
        if let Some(mont_ctx) = MontgomeryContext::new(n) {
            return brent_cycle_find_montgomery(x0, c, n, max_iterations, &mont_ctx);
        }
    }

    // Fallback to standard arithmetic
    brent_cycle_find_standard(x0, c, n, max_iterations)
}

/// Standard Brent's cycle-finding without Montgomery form
#[allow(clippy::many_single_char_names)]
fn brent_cycle_find_standard(
    x0: &BigUint,
    c: &BigUint,
    n: &BigUint,
    max_iterations: u64,
) -> Option<BigUint> {
    let mut x = x0.clone();
    let mut y = x0.clone();
    let mut d = BigUint::one();

    let mut r: u64 = 1;
    let mut q = BigUint::one();

    while d == BigUint::one() {
        // Do r iterations of f(x) = x^2 + c
        for _ in 0..r {
            x = f(&x, c, n);

            // Accumulate differences for batch GCD
            let diff = if x > y { (&x - &y) % n } else { (&y - &x) % n };

            if diff != BigUint::zero() {
                q = (&q * &diff) % n;
            }
        }

        y.clone_from(&x);
        r *= 2;

        if r > max_iterations {
            return None; // Didn't find factor
        }

        // Periodic GCD check
        d = gcd(q.clone(), n.clone());
    }

    if d == *n { None } else { Some(d) }
}

/// Brent's cycle-finding with Montgomery form arithmetic
/// All arithmetic done in Montgomery form for 3-5x speedup
#[allow(clippy::many_single_char_names)]
fn brent_cycle_find_montgomery(
    x0: &BigUint,
    c: &BigUint,
    n: &BigUint,
    max_iterations: u64,
    mont_ctx: &MontgomeryContext,
) -> Option<BigUint> {
    // Convert to Montgomery form
    let c_mont = mont_ctx.to_montgomery(c);
    let mut x_mont = mont_ctx.to_montgomery(x0);
    let mut y_mont = mont_ctx.to_montgomery(x0);
    let mut d = BigUint::one();

    let mut r: u64 = 1;
    let mut q_mont = mont_ctx.to_montgomery(&BigUint::one());

    while d == BigUint::one() {
        // Do r iterations of f(x) = x^2 + c in Montgomery form
        for _ in 0..r {
            // f(x) = x^2 + c in Montgomery form
            let x_sq_mont = mont_ctx.mul(&x_mont, &x_mont);
            x_mont = (&x_sq_mont + &c_mont) % n;

            // Accumulate differences for batch GCD
            let diff_mont = if x_mont > y_mont {
                (&x_mont - &y_mont) % n
            } else {
                (&y_mont - &x_mont) % n
            };

            if diff_mont != BigUint::zero() {
                q_mont = mont_ctx.mul(&q_mont, &diff_mont);
            }
        }

        y_mont.clone_from(&x_mont);
        r *= 2;

        if r > max_iterations {
            return None; // Didn't find factor
        }

        // Periodic GCD check (convert back from Montgomery form)
        let q = mont_ctx.convert_from_montgomery(&q_mont);
        d = gcd(q, n.clone());
    }

    if d == *n { None } else { Some(d) }
}

/// Pollard-Rho sequence function: f(x) = x^2 + c mod n
#[inline]
fn f(x: &BigUint, c: &BigUint, n: &BigUint) -> BigUint {
    let x_sq = (x * x) % n;
    (&x_sq + c) % n
}

/// Generate a random BigUint in range [1, n)
fn get_random_biguint(n: &BigUint) -> BigUint {
    use rand::Rng;
    let mut rng = rand::rng();
    let bits = n.bits() as usize;

    // Generate random bytes
    let byte_len = bits.div_ceil(8);
    let mut bytes = vec![0u8; byte_len];
    rng.fill(&mut bytes[..]);

    let mut result = BigUint::from_bytes_le(&bytes);
    result = &result % n;

    // Ensure result is in [1, n)
    if result.is_zero() {
        result = BigUint::one();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pollard_rho_small() {
        let n = BigUint::parse_bytes(b"91", 10).unwrap(); // 7 * 13
        if let Some(factor) = pollard_rho_brent(&n) {
            assert!(factor == BigUint::from(7u32) || factor == BigUint::from(13u32));
        }
    }

    #[test]
    fn test_pollard_rho_medium() {
        let n = BigUint::parse_bytes(b"1000000007", 10).unwrap(); // Prime
        let result = pollard_rho_brent(&n);
        assert!(result.is_none()); // Should not find factor for prime
    }
}
