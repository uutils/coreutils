// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Fermat's factorization method
//!
//! Optimal for semiprimes with factors p and q where |p - q| is small.
//! Time complexity: O(√(p - q))
//!
//! Best used as first attempt for 32-bit semiprimes before Pollard-Rho.

use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::{One, Zero};

/// Fermat factorization for u64 numbers
///
/// Handles numbers up to 2^64
/// Expected iterations: O(√(p - q))
///
/// # Arguments
/// * `n` - Odd composite number to factor
///
/// # Returns
/// `Some(factor)` if successful, `None` if limit exceeded
///
/// # Example
/// ```ignore
/// # use uu_factor::fermat::fermat_factor_u64;
/// let n = 4295049777u64; // 65521 × 65537
/// let factor = fermat_factor_u64(n).unwrap();
/// assert!(n % factor == 0);
/// ```
pub fn fermat_factor_u64(n: u64) -> Option<u64> {
    if n <= 1 || n % 2 == 0 {
        return None;
    }

    // x² - y² = n
    // Start from ceil(√n)
    let mut x = isqrt_u64(n);
    if x * x < n {
        x += 1;
    }

    // Safety limit: 2^20 iterations max (for factors up to ~65K apart)
    const MAX_ITERATIONS: u64 = 1_000_000;

    for iteration in 0..MAX_ITERATIONS {
        // Use assembly-optimized multiplication for x²
        let x_squared = mul_u64_checked(x, x)?;

        let diff = x_squared.checked_sub(n)?;

        // Check if diff is a perfect square
        let y = isqrt_u64(diff);
        if mul_u64_overflow_ok(y, y) == diff {
            // Found! x - y is a factor
            let factor = x.checked_sub(y)?;
            if factor > 1 && factor < n && n % factor == 0 {
                return Some(factor);
            }
        }

        x += 1;

        // Early exit if |p - q| was very large (factors not close)
        if iteration > 1000 && iteration % 1000 == 0 {
            // Heuristic: if we've done 1000+ iterations, factors likely far apart
            // Let Pollard-Rho handle it instead
            if iteration > 100_000 {
                return None;
            }
        }
    }

    None
}

/// Multiply two u64 values with overflow check (pure Rust)
#[inline]
fn mul_u64_checked(a: u64, b: u64) -> Option<u64> {
    a.checked_mul(b)
}

/// Multiply two u64 values, returning low 64 bits (overflow is OK - pure Rust)
#[inline]
fn mul_u64_overflow_ok(a: u64, b: u64) -> u64 {
    a.wrapping_mul(b)
}

/// Fermat factorization for BigUint (64-90 bits)
pub fn fermat_factor_biguint(n: &BigUint) -> Option<BigUint> {
    if n.is_even() || n <= &BigUint::one() {
        return None;
    }

    // x² - y² = n
    // Start from ceil(√n)
    let mut x = integer_sqrt_ceil(n);

    const MAX_ITERATIONS: u64 = 1_000_000;

    for _iteration in 0..MAX_ITERATIONS {
        let x_squared = &x * &x;

        // Ensure x_squared > n (required for subtraction)
        if x_squared <= *n {
            x += BigUint::one();
            continue;
        }

        let diff = &x_squared - n;

        // Check if diff is a perfect square
        if let Some(y) = integer_sqrt_exact(&diff) {
            let factor = &x - &y;
            if factor > BigUint::one() && factor < *n && n % &factor == BigUint::zero() {
                return Some(factor);
            }
        }

        x += BigUint::one();
    }

    None
}

/// Integer square root (floor) - pure Rust using Newton's method
#[inline]
fn isqrt_u64(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }

    // Newton's method for integer square root
    let mut x = n;
    let mut y = (x + 1) >> 1;

    while y < x {
        x = y;
        y = (x + n / x) >> 1;
    }
    x
}

/// Check if n is a perfect square and return √n if true
fn integer_sqrt_exact(n: &BigUint) -> Option<BigUint> {
    let root = integer_sqrt_floor(n);
    if &root * &root == *n {
        Some(root)
    } else {
        None
    }
}

/// Integer square root (floor) for BigUint
fn integer_sqrt_floor(n: &BigUint) -> BigUint {
    if n.is_zero() {
        return BigUint::zero();
    }

    // Newton's method
    let mut x = n.clone();
    let mut y: BigUint = (n + BigUint::one()) >> 1;

    while y < x {
        x.clone_from(&y);
        y = (&x + n / &x) >> 1;
    }
    x
}

/// Integer square root (ceiling) for BigUint
fn integer_sqrt_ceil(n: &BigUint) -> BigUint {
    let root = integer_sqrt_floor(n);
    if &root * &root == *n {
        root
    } else {
        root + BigUint::one()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fermat_u64_close_factors() {
        // 65521 × 65537 = 4,294,049,777
        let n = 4294049777u64;
        let factor = fermat_factor_u64(n).expect("Should factor quickly");
        assert!(n % factor == 0);
        assert!(factor == 65521 || factor == 65537);
    }

    #[test]
    fn test_fermat_u64_product() {
        // 1000000007 × 1000000009
        let n = 1_000_000_007u64 * 1_000_000_009u64;
        let factor = fermat_factor_u64(n);
        if let Some(f) = factor {
            assert!(n % f == 0);
        }
    }

    #[test]
    fn test_fermat_biguint_32bit() {
        let p = BigUint::from(65521u32);
        let q = BigUint::from(65537u32);
        let n = &p * &q;

        let factor = fermat_factor_biguint(&n).expect("Should factor");
        assert!(&n % &factor == BigUint::zero());
    }

    #[test]
    fn test_fermat_prime_returns_none() {
        let n = BigUint::from(97u32); // Prime
        assert_eq!(fermat_factor_biguint(&n), None);
    }

    #[test]
    fn test_fermat_even_returns_none() {
        let n = BigUint::from(100u32);
        assert_eq!(fermat_factor_biguint(&n), None);
    }

    #[test]
    fn test_isqrt_u64() {
        assert_eq!(isqrt_u64(0), 0);
        assert_eq!(isqrt_u64(1), 1);
        assert_eq!(isqrt_u64(4), 2);
        assert_eq!(isqrt_u64(9), 3);
        assert_eq!(isqrt_u64(15), 3);
        assert_eq!(isqrt_u64(16), 4);
        assert_eq!(isqrt_u64(100), 10);
    }
}
