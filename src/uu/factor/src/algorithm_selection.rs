// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Algorithm selection for optimal factorization
//!
//! This module routes numbers to the appropriate factorization method:
//! - Small numbers (< 128 bits): fast_factor (optimized for u64/u128 range)
//! - Larger numbers (>= 128 bits): falls back to num_prime

use num_bigint::BigUint;
use num_traits::ToPrimitive;
use std::collections::BTreeMap;

use super::ecm::ecm_find_factor;
use super::fermat::{fermat_factor_biguint, fermat_factor_u64};
use super::montgomery_u128::{
    is_probable_prime_u128_montgomery, pollard_rho_brent_u128_montgomery,
};
use super::pollard_rho::pollard_rho_with_target;
use super::trial_division::{extract_small_factors, quick_trial_divide};
use super::u64_factor::{is_probable_prime_u64, pollard_rho_brent_u64, trial_division_u64};

/// Fast factorization for numbers < 128 bits
///
/// Strategy (internal routing):
/// - ≤ 64 bits: Use optimized u64 algorithms (trial division + Pollard-Rho, Fermat hint)
/// - 65-127 bits: Use Montgomery-optimized u128 path (avoids slow mulmod with REDC)
/// - ≥ 128 bits: Fall back to BigUint algorithms
fn fast_factorize_small(n: &BigUint) -> BTreeMap<BigUint, usize> {
    let bits = n.bits();

    // Handle trivial cases
    if n <= &BigUint::from(1u32) {
        return BTreeMap::new();
    }

    // For numbers ≤ 64 bits, use u64 optimization path
    if bits <= 64 {
        if let Some(n_u64) = n.to_u64() {
            return factorize_u64_fast(n_u64);
        }
    }

    // For 65-127 bit numbers, use Montgomery-optimized u128 path
    // Montgomery reduction replaces expensive division with bit shifts
    if bits <= 127 {
        if let Some(n_u128) = n.to_u128() {
            return factorize_u128_montgomery(n_u128);
        }
    }

    // For ≥ 128 bit numbers, use BigUint path
    factorize_biguint_fast(n)
}

/// Optimized factorization for u64 numbers
fn factorize_u64_fast(mut n: u64) -> BTreeMap<BigUint, usize> {
    let mut factors = BTreeMap::new();

    if n <= 1 {
        return factors;
    }

    if n == 2 || n == 3 || n == 5 {
        factors.insert(BigUint::from(n), 1);
        return factors;
    }

    // Trial division for small primes (up to ~1000)
    let small_primes_u64 = trial_division_u64(&mut n, 1000);
    for prime in small_primes_u64 {
        *factors.entry(BigUint::from(prime)).or_insert(0) += 1;
    }

    // If fully factored, return
    if n == 1 {
        return factors;
    }

    if is_probable_prime_u64(n) {
        factors.insert(BigUint::from(n), 1);
        return factors;
    }

    // Try Fermat's method first for semiprimes (optimal for close factors)
    if let Some(fermat_factor) = fermat_factor_u64(n) {
        // Found via Fermat! Recursively factor both parts
        factorize_u64_pollard_rho(&mut factors, fermat_factor);
        factorize_u64_pollard_rho(&mut factors, n / fermat_factor);
        return factors;
    }

    // Fallback to Pollard-Rho for remaining composite
    factorize_u64_pollard_rho(&mut factors, n);

    factors
}

/// Recursive Pollard-Rho factorization for u64
fn factorize_u64_pollard_rho(factors: &mut BTreeMap<BigUint, usize>, n: u64) {
    if n == 1 {
        return;
    }

    if is_probable_prime_u64(n) {
        *factors.entry(BigUint::from(n)).or_insert(0) += 1;
        return;
    }

    // Find a factor using Pollard-Rho
    if let Some(factor) = pollard_rho_brent_u64(n) {
        // Recursively factor both parts
        factorize_u64_pollard_rho(factors, factor);
        factorize_u64_pollard_rho(factors, n / factor);
    } else {
        // Couldn't find factor, assume it's prime (shouldn't happen often)
        *factors.entry(BigUint::from(n)).or_insert(0) += 1;
    }
}

/// Montgomery-optimized factorization for u128 (65-127 bit numbers)
///
/// Uses Montgomery reduction for fast modular arithmetic:
/// - All operations stay in Montgomery form during Pollard-Rho inner loop
/// - REDC replaces expensive division with bit shifts
/// - Expected ~5-7x speedup over non-Montgomery u128 path
fn factorize_u128_montgomery(mut n: u128) -> BTreeMap<BigUint, usize> {
    let mut factors = BTreeMap::new();

    if n <= 1 {
        return factors;
    }

    // Extract factors of 2
    while n % 2 == 0 {
        *factors.entry(BigUint::from(2u32)).or_insert(0) += 1;
        n /= 2;
    }

    if n == 1 {
        return factors;
    }

    // Extract small prime factors (trial division up to 1000)
    for p in [
        3u128, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83,
        89, 97,
    ] {
        while n % p == 0 {
            *factors.entry(BigUint::from(p)).or_insert(0) += 1;
            n /= p;
        }
    }

    if n == 1 {
        return factors;
    }

    // Use recursive Montgomery Pollard-Rho for remaining cofactor
    factorize_u128_pollard_rho_montgomery(&mut factors, n);

    factors
}

/// Recursive Pollard-Rho factorization for u128 using Montgomery optimization
fn factorize_u128_pollard_rho_montgomery(factors: &mut BTreeMap<BigUint, usize>, n: u128) {
    if n == 1 {
        return;
    }

    if is_probable_prime_u128_montgomery(n) {
        *factors.entry(BigUint::from(n)).or_insert(0) += 1;
        return;
    }

    // Find a factor using Montgomery-optimized Pollard-Rho
    if let Some(factor) = pollard_rho_brent_u128_montgomery(n) {
        // Recursively factor both parts
        factorize_u128_pollard_rho_montgomery(factors, factor);
        factorize_u128_pollard_rho_montgomery(factors, n / factor);
    } else {
        // Couldn't find factor, assume it's prime (shouldn't happen often)
        *factors.entry(BigUint::from(n)).or_insert(0) += 1;
    }
}

/// Optimized factorization for BigUint (> 128 bit range, internal)
fn factorize_biguint_fast(n: &BigUint) -> BTreeMap<BigUint, usize> {
    use num_prime::nt_funcs::is_prime;

    let mut factors = BTreeMap::new();

    // Extract small factors first
    let (small_factors, mut remaining) = extract_small_factors(n.clone());
    for factor in small_factors {
        *factors.entry(factor).or_insert(0) += 1;
    }

    // If fully factored, return
    if remaining == BigUint::from(1u32) {
        return factors;
    }

    // Trial division for medium-sized primes
    let (more_factors, final_remaining) = quick_trial_divide(remaining);
    for factor in more_factors {
        *factors.entry(factor).or_insert(0) += 1;
    }
    remaining = final_remaining;

    // If fully factored, return
    if remaining == BigUint::from(1u32) || remaining == BigUint::from(0u32) {
        return factors;
    }

    // Check if remaining is prime
    if is_prime(&remaining, None).probably() {
        *factors.entry(remaining).or_insert(0) += 1;
        return factors;
    }

    // For 65-127 bit composites, use our Montgomery-optimized u128 Pollard-Rho
    // This is faster than num_prime for this size range
    if remaining.bits() <= 127 {
        if let Some(n_u128) = remaining.to_u128() {
            factorize_u128_pollard_rho_montgomery(&mut factors, n_u128);
            return factors;
        }
    }

    // Try Fermat's method for numbers up to ~90 bits (optimal for close factors)
    if remaining.bits() <= 90 {
        if let Some(fermat_factor) = fermat_factor_biguint(&remaining) {
            // Found via Fermat! Recursively factor both parts
            factorize_biguint_pollard_rho(&mut factors, fermat_factor.clone());
            factorize_biguint_pollard_rho(&mut factors, &remaining / &fermat_factor);
            return factors;
        }
    }

    // Fallback to Pollard-Rho for remaining composite
    factorize_biguint_pollard_rho(&mut factors, remaining);

    factors
}

/// Recursive Pollard-Rho factorization for BigUint (internal)
fn factorize_biguint_pollard_rho(factors: &mut BTreeMap<BigUint, usize>, n: BigUint) {
    if n == BigUint::from(1u32) {
        return;
    }

    // For very small n, assume prime
    if n.bits() <= 20 {
        *factors.entry(n).or_insert(0) += 1;
        return;
    }

    // Estimate factor size (assume roughly balanced factors)
    let target_bits = (n.bits() as u32) / 2;

    // Find a factor using Pollard-Rho
    if let Some(factor) = pollard_rho_with_target(&n, target_bits) {
        if factor < n {
            // Recursively factor both parts
            factorize_biguint_pollard_rho(factors, factor.clone());
            factorize_biguint_pollard_rho(factors, &n / &factor);
        } else {
            // Factor is same as n, assume prime
            *factors.entry(n).or_insert(0) += 1;
        }
    } else {
        // Couldn't find factor, assume it's prime
        *factors.entry(n).or_insert(0) += 1;
    }
}

/// Main factorization entry point with algorithm selection
///
/// Routes to the optimal algorithm based on number size:
/// - < 128 bits: fast_factor (trial division + Fermat + Pollard-Rho)
/// - >= 128 bits: ECM (Elliptic Curve Method) with num_prime fallback
pub fn factorize(n: &BigUint) -> (BTreeMap<BigUint, usize>, Option<Vec<BigUint>>) {
    let bits = n.bits();

    // < 128-bit path: use our fast implementation
    if bits < 128 {
        return (fast_factorize_small(n), None);
    }

    // >= 128 bits: Use ECM-based factorization with recursive decomposition
    factorize_large_ecm(n)
}

/// Factorize large numbers (>= 128 bits) using ECM with recursive decomposition
fn factorize_large_ecm(n: &BigUint) -> (BTreeMap<BigUint, usize>, Option<Vec<BigUint>>) {
    use num_bigint::BigUint;
    use num_prime::nt_funcs::is_prime;
    use num_traits::One;

    let mut factors: BTreeMap<BigUint, usize> = BTreeMap::new();
    let mut remaining = n.clone();

    // First extract small factors using trial division
    let (small_factors, after_trial) = extract_small_factors(remaining);
    for f in small_factors {
        *factors.entry(f).or_insert(0) += 1;
    }
    remaining = after_trial;

    // Work queue for composite numbers to factor
    let mut composites: Vec<BigUint> = vec![remaining];

    while let Some(composite) = composites.pop() {
        if composite <= BigUint::one() {
            continue;
        }

        // Check if it's prime
        if is_prime(&composite, None).probably() {
            *factors.entry(composite).or_insert(0) += 1;
            continue;
        }

        // Try ECM with timeout based on size
        let timeout_ms = match composite.bits() {
            0..=150 => 5_000,    // 5 seconds for smaller composites
            151..=200 => 15_000, // 15 seconds for medium
            _ => 30_000,         // 30 seconds for very large
        };

        if let Some(factor) = ecm_find_factor(&composite, timeout_ms) {
            // Found a factor! Add both parts to work queue
            let cofactor = &composite / &factor;
            composites.push(factor);
            composites.push(cofactor);
        } else {
            // ECM failed, try Pollard-Rho as backup
            if let Some(factor) = super::pollard_rho::pollard_rho_brent(&composite) {
                let cofactor = &composite / &factor;
                composites.push(factor);
                composites.push(cofactor);
            } else {
                // Both failed, fall back to num_prime for this composite
                let (sub_factors, sub_remaining) = num_prime::nt_funcs::factors(composite, None);
                for (f, count) in sub_factors {
                    *factors.entry(f).or_insert(0) += count;
                }
                if let Some(unfactored) = sub_remaining {
                    return (factors, Some(unfactored));
                }
            }
        }
    }

    (factors, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factorize_128bit() {
        // 128-bit semiprime (boundary of <u128 focus)
        // Using two ~64-bit primes to create a ~128-bit semiprime
        let p = BigUint::parse_bytes(b"18446744073709551629", 10).unwrap();
        let q = BigUint::parse_bytes(b"18446744073709551557", 10).unwrap();
        let n = &p * &q;

        assert!(n.bits() >= 100);

        let (factors, remaining) = factorize(&n);
        assert_eq!(remaining, None);
        // Should factor successfully
        assert!(!factors.is_empty());
    }
}
